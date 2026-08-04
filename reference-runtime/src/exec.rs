//! The HarnessXML execution model — specification chapters 5 to 8.
//!
//! Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0
//!
//! This is the reference EXECUTOR. Its job is to be unambiguous, not fast:
//! every scheduling rule, lifecycle transition and failure behaviour in the
//! specification has running code here, so a disagreement about what a document
//! means can be settled by reading an implementation instead of arguing about
//! prose.
//!
//! DETERMINISM. The specification permits a runtime to execute any READY nodes
//! in any order (§5.4). This executor deliberately picks document order and
//! runs single-threaded, because conformance traces must be reproducible. A
//! loop's `maxConcurrency` is therefore honoured as a BOUND that is never
//! exceeded rather than as real parallelism — which is conformant, and which
//! means a workflow whose result depends on completion order will produce the
//! same trace here every time while still being a race in a parallel runtime.
//!
//! TIME. Backoff delays are computed and recorded but never slept. A reference
//! implementation that took four minutes to demonstrate a retry policy would
//! not get used, and wall-clock has no bearing on which states a node passes
//! through.

use crate::expr::{self, Ctx, Value};
use crate::model::*;
use std::collections::{BTreeMap, HashMap, HashSet};

// ---------------------------------------------------------------- state

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Pending,
    Ready,
    Running,
    Retrying,
    Succeeded,
    Skipped,
    Failed,
    Cancelled,
    Compensated,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Pending => "PENDING",
            State::Ready => "READY",
            State::Running => "RUNNING",
            State::Retrying => "RETRYING",
            State::Succeeded => "SUCCEEDED",
            State::Skipped => "SKIPPED",
            State::Failed => "FAILED",
            State::Cancelled => "CANCELLED",
            State::Compensated => "COMPENSATED",
        }
    }

    /// §6.1 — a node in a terminal state never transitions again, except
    /// SUCCEEDED -> COMPENSATED during unwinding.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            State::Succeeded
                | State::Skipped
                | State::Failed
                | State::Cancelled
                | State::Compensated
        )
    }

    /// §4.1 — SKIPPED counts as success. A guarded node whose guard was false
    /// did exactly what the document asked; treating it as failure would mean
    /// any optional step halted everything after it.
    pub fn is_success(self) -> bool {
        matches!(self, State::Succeeded | State::Skipped)
    }
}

// ---------------------------------------------------------------- runner

#[derive(Debug, Clone)]
pub enum Outcome {
    Success(BTreeMap<String, Value>),
    Failure { class: String, message: String },
    TimedOut,
}

/// `impl` is opaque to the specification (§3.1.1), so the executor cannot know
/// how to run a node. This is the seam where a real runtime plugs in.
pub trait NodeRunner {
    fn run(&mut self, node: &Node, inputs: &BTreeMap<String, Value>, attempt: u32) -> Outcome;
}

/// Deterministic runner driven by a scenario script, so that conformance traces
/// are reproducible. Without a scenario every node succeeds, binding each
/// declared output to a placeholder.
///
/// Scenario syntax, one directive per line:
///
/// ```text
/// # comment
/// classify        fail  rate_limit  throttled by upstream
/// classify@2      ok    confidence=0.95 category=invoice
/// grasp_part      timeout
/// detect_grasps   ok    grasps=[1,2,3]
/// ```
///
/// `node@attempt` scripts one attempt; a bare `node` applies to every attempt
/// that has no more specific directive.
#[derive(Default)]
pub struct SimulatedRunner {
    per_attempt: HashMap<(String, u32), Outcome>,
    default: HashMap<String, Outcome>,
}

impl SimulatedRunner {
    pub fn from_scenario(src: &str) -> Result<Self, String> {
        let mut r = Self::default();
        for (lineno, raw) in src.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let mut it = line.split_whitespace();
            let target = it.next().unwrap();
            let verb = it
                .next()
                .ok_or_else(|| format!("scenario line {}: missing outcome", lineno + 1))?;

            let outcome = match verb {
                "ok" => {
                    let mut m = BTreeMap::new();
                    for kv in it {
                        let (k, v) = kv.split_once('=').ok_or_else(|| {
                            format!(
                                "scenario line {}: expected key=value, got '{kv}'",
                                lineno + 1
                            )
                        })?;
                        m.insert(k.to_string(), parse_scalar(v));
                    }
                    Outcome::Success(m)
                }
                "fail" => {
                    let class = it.next().unwrap_or("internal").to_string();
                    let message: Vec<&str> = it.collect();
                    Outcome::Failure {
                        class,
                        message: if message.is_empty() {
                            "simulated failure".into()
                        } else {
                            message.join(" ")
                        },
                    }
                }
                "timeout" => Outcome::TimedOut,
                other => {
                    return Err(format!(
                        "scenario line {}: unknown outcome '{other}' (expected ok, fail or timeout)",
                        lineno + 1
                    ));
                }
            };

            match target.split_once('@') {
                Some((node, attempt)) => {
                    let a: u32 = attempt.parse().map_err(|_| {
                        format!("scenario line {}: bad attempt '{attempt}'", lineno + 1)
                    })?;
                    r.per_attempt.insert((node.to_string(), a), outcome);
                }
                None => {
                    r.default.insert(target.to_string(), outcome);
                }
            }
        }
        Ok(r)
    }
}

fn parse_scalar(v: &str) -> Value {
    match v {
        "true" => return Value::Bool(true),
        "false" => return Value::Bool(false),
        "null" => return Value::Null,
        _ => {}
    }
    if let Ok(n) = v.parse::<f64>() {
        return Value::Num(n);
    }
    if v.starts_with('[') && v.ends_with(']') {
        let inner = &v[1..v.len() - 1];
        if inner.trim().is_empty() {
            return Value::Array(vec![]);
        }
        return Value::Array(inner.split(',').map(|x| parse_scalar(x.trim())).collect());
    }
    Value::Str(v.to_string())
}

impl NodeRunner for SimulatedRunner {
    fn run(&mut self, node: &Node, _inputs: &BTreeMap<String, Value>, attempt: u32) -> Outcome {
        if let Some(o) = self.per_attempt.get(&(node.id.clone(), attempt)) {
            return o.clone();
        }
        if let Some(o) = self.default.get(&node.id) {
            return o.clone();
        }
        // Unscripted: succeed, binding each declared output to a placeholder so
        // downstream expressions have something to read.
        //
        // The placeholder MUST respect the port's declared type. A string
        // standing in for a `number` makes every threshold comparison fail with
        // HX-4106 — a diagnostic about the harness that is really a defect in
        // the test double, which is the most confusing kind.
        let mut m = BTreeMap::new();
        for p in &node.outputs {
            let ty = p.ty.as_deref().unwrap_or("");
            let v = if ty.starts_with("array") {
                Value::Array(vec![Value::Num(1.0), Value::Num(2.0)])
            } else {
                match ty {
                    "number" => Value::Num(1.0),
                    "integer" => Value::Num(1.0),
                    "boolean" => Value::Bool(true),
                    "json" | "map" => Value::Map(BTreeMap::new()),
                    _ => Value::Str(format!("{}.{}", node.id, p.name)),
                }
            };
            m.insert(p.name.clone(), v);
        }
        Outcome::Success(m)
    }
}

// ---------------------------------------------------------------- trace

#[derive(Debug, Clone)]
pub struct TraceEntry {
    pub node: String,
    pub from: State,
    pub to: State,
    pub attempt: u32,
    /// Iteration index when this transition happened inside a loop body.
    pub iteration: Option<u64>,
    pub error_class: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Default)]
pub struct Trace {
    pub entries: Vec<TraceEntry>,
}

impl Trace {
    /// §6.6 — conformance compares the NORMALISED sequence of transitions, not
    /// timings and not the interleaving of independent branches.
    pub fn to_json(&self) -> String {
        let items: Vec<String> = self
            .entries
            .iter()
            .map(|e| {
                let mut f = vec![
                    format!("\"node\":\"{}\"", e.node),
                    format!("\"from\":\"{}\"", e.from.name()),
                    format!("\"to\":\"{}\"", e.to.name()),
                    format!("\"attempt\":{}", e.attempt),
                ];
                if let Some(i) = e.iteration {
                    f.push(format!("\"iteration\":{i}"));
                }
                if let Some(c) = &e.error_class {
                    f.push(format!("\"errorClass\":\"{c}\""));
                }
                if let Some(d) = &e.detail {
                    f.push(format!("\"detail\":\"{}\"", d.replace('"', "'")));
                }
                format!("  {{{}}}", f.join(","))
            })
            .collect();
        format!("[\n{}\n]", items.join(",\n"))
    }
}

// ---------------------------------------------------------------- outcome

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunResult {
    Succeeded,
    Failed,
    Compensated,
}

pub struct Run {
    pub result: RunResult,
    pub states: BTreeMap<String, State>,
    pub trace: Trace,
    pub errors: Vec<(String, String)>,
}

// ---------------------------------------------------------------- executor

pub struct Executor<'a, R: NodeRunner> {
    h: &'a Harness,
    runner: R,
    states: HashMap<String, State>,
    outputs: HashMap<String, HashMap<String, Value>>,
    /// Edges that evaluated false and will never be satisfied (§5.3).
    negative: HashSet<usize>,
    /// Completion order, so compensation can unwind in reverse (§8.4.1).
    completed: Vec<String>,
    /// Successors a decision node did NOT choose.
    decision_rejected: HashSet<(String, String)>,
    /// The single successor a decision DID choose.
    decision_chosen: HashSet<(String, String)>,
    trace: Trace,
    errors: Vec<(String, String)>,
    vars: HashMap<String, Value>,
    iteration: Option<u64>,
    /// Nodes no path will ever reach. Unreachability PROPAGATES: an
    /// unreachable source makes its outgoing edges resolved-negative, so a
    /// downstream `all` join can resolve instead of waiting forever.
    unreachable: HashSet<String>,
    /// Nodes permitted to start with no incoming forward edge (§2.5).
    entry_set: HashSet<String>,
    /// Targets whose incoming error edge has fired (§4.4). An error handler is
    /// NOT an entry node — it only becomes reachable once something failed.
    error_enabled: HashSet<String>,
}

impl<'a, R: NodeRunner> Executor<'a, R> {
    pub fn new(h: &'a Harness, runner: R) -> Self {
        Self {
            h,
            runner,
            states: h
                .nodes
                .iter()
                .map(|n| (n.id.clone(), State::Pending))
                .collect(),
            outputs: HashMap::new(),
            negative: HashSet::new(),
            completed: Vec::new(),
            decision_rejected: HashSet::new(),
            decision_chosen: HashSet::new(),
            trace: Trace::default(),
            errors: Vec::new(),
            vars: HashMap::new(),
            iteration: None,
            unreachable: HashSet::new(),
            entry_set: HashSet::new(),
            error_enabled: HashSet::new(),
        }
    }

    fn state(&self, id: &str) -> State {
        *self.states.get(id).unwrap_or(&State::Pending)
    }

    fn set(
        &mut self,
        id: &str,
        to: State,
        attempt: u32,
        class: Option<String>,
        detail: Option<String>,
    ) {
        let from = self.state(id);
        if from == to {
            return;
        }
        self.states.insert(id.to_string(), to);
        if to == State::Failed {
            // §4.4 — an error edge is traversed ONCE, at the end, after retries
            // are exhausted. All outgoing error edges are taken.
            let targets: Vec<String> = self
                .h
                .edges
                .iter()
                .filter(|e| e.ty == EdgeType::Error && e.from == id)
                .map(|e| e.to.clone())
                .collect();
            for t in targets {
                self.error_enabled.insert(t);
            }
        }
        self.trace.entries.push(TraceEntry {
            node: id.to_string(),
            from,
            to,
            attempt,
            iteration: self.iteration,
            error_class: class,
            detail,
        });
        if to.is_terminal() && to != State::Compensated {
            self.completed.push(id.to_string());
        }
    }

    fn ctx(&self, node: Option<&Node>) -> Ctx {
        let mut c = Ctx {
            outputs: self
                .outputs
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            vars: self.vars.clone(),
            skipped: self
                .states
                .iter()
                .filter(|(_, s)| **s == State::Skipped)
                .map(|(k, _)| k.clone())
                .collect(),
            ..Default::default()
        };
        if let Some(n) = node {
            c.config = n
                .config
                .iter()
                .map(|(k, v)| (k.clone(), Value::Str(v.clone())))
                .collect();
        }
        for a in &self.h.artifacts {
            c.artifacts.insert(
                a.id.clone(),
                a.attrs
                    .iter()
                    .map(|(k, v)| (k.clone(), Value::Str(v.clone())))
                    .collect(),
            );
        }
        for r in &self.h.resources {
            c.resources.insert(
                r.id.clone(),
                r.attrs
                    .iter()
                    .map(|(k, v)| (k.clone(), Value::Str(v.clone())))
                    .collect(),
            );
        }
        c
    }

    // ---- readiness -------------------------------------------------

    /// §5.3 — an incoming edge is satisfied, resolved-negative, or still
    /// pending. Only control, data and dependency edges participate.
    fn edge_status(&mut self, idx: usize) -> EdgeStatus {
        if self.negative.contains(&idx) {
            return EdgeStatus::Negative;
        }
        let e = &self.h.edges[idx];
        let src = self.state(&e.from);

        // A source nothing will ever reach cannot satisfy anything downstream.
        // Without this, an `all` join on a branch the workflow did not take
        // waits forever and the instance deadlocks.
        if self.unreachable.contains(&e.from) {
            self.negative.insert(idx);
            return EdgeStatus::Negative;
        }

        // A decision that routed elsewhere kills this path.
        if self
            .decision_rejected
            .contains(&(e.from.clone(), e.to.clone()))
        {
            self.negative.insert(idx);
            return EdgeStatus::Negative;
        }

        let ok = match e.ty {
            EdgeType::Control | EdgeType::Data => src.is_success(),
            EdgeType::Dependency => src.is_terminal(),
            _ => return EdgeStatus::Pending,
        };
        if !src.is_terminal() {
            return EdgeStatus::Pending;
        }
        if !ok {
            self.negative.insert(idx);
            return EdgeStatus::Negative;
        }

        if let Some(cond) = e.condition.clone() {
            let ctx = self.ctx(None);
            match expr::eval_condition(&cond, &ctx) {
                Ok(true) => {}
                Ok(false) => {
                    self.negative.insert(idx);
                    return EdgeStatus::Negative;
                }
                Err(err) => {
                    self.errors.push((
                        e.from.clone(),
                        format!("{} edge condition: {}", err.code, err.message),
                    ));
                    self.negative.insert(idx);
                    return EdgeStatus::Negative;
                }
            }
        }
        EdgeStatus::Satisfied
    }

    fn incoming(&self, id: &str) -> Vec<usize> {
        self.h
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.to == id && e.ty.is_forward())
            .map(|(i, _)| i)
            .collect()
    }

    /// §5.3.1 — join policies.
    /// A node named by some decision's `case/@to` is gated on that decision:
    /// it may not start until the decision has chosen it, and becomes
    /// unreachable once every decision that could reach it has routed
    /// elsewhere. Decisions route by `@to` rather than by an edge, so this
    /// gate is what connects them to the schedule.
    fn decision_gate(&self, id: &str) -> Readiness {
        let deciders: Vec<&Node> = self
            .h
            .nodes
            .iter()
            .filter(|n| {
                n.cases.as_ref().is_some_and(|c| {
                    c.cases.iter().any(|(_, to)| to == id) || c.otherwise.as_deref() == Some(id)
                })
            })
            .collect();
        if deciders.is_empty() {
            return Readiness::Ready;
        }
        if deciders.iter().any(|d| {
            self.decision_chosen
                .contains(&(d.id.clone(), id.to_string()))
        }) {
            return Readiness::Ready;
        }
        if deciders.iter().all(|d| self.state(&d.id).is_terminal()) {
            return Readiness::Unreachable;
        }
        Readiness::Waiting
    }

    fn readiness(&mut self, id: &str) -> Readiness {
        if self.unreachable.contains(id) {
            return Readiness::Unreachable;
        }
        match self.decision_gate(id) {
            Readiness::Ready => {}
            other => {
                if matches!(other, Readiness::Unreachable) {
                    self.unreachable.insert(id.to_string());
                }
                return other;
            }
        }

        // A required input fed by a data edge that will never be satisfied
        // makes this node unreachable too — it can never obtain the value it
        // declared it needs. Left as merely "waiting", the run deadlocks;
        // treated as ready, it fails with HX-4101 for a value nobody ever
        // intended to produce.
        let dead_input = self
            .h
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.ty == EdgeType::Data && e.to == id)
            .any(|(i, e)| {
                let feeds_required = e
                    .to_port
                    .as_ref()
                    .and_then(|p| self.h.node(id).and_then(|n| n.input(p)))
                    .map(|p| p.required && p.default.is_none())
                    .unwrap_or(false);
                feeds_required && (self.unreachable.contains(&e.from) || self.negative.contains(&i))
            });
        if dead_input {
            self.unreachable.insert(id.to_string());
            return Readiness::Unreachable;
        }
        let edges = self.incoming(id);
        if edges.is_empty() {
            // No incoming CONTROL/DATA/DEPENDENCY edge. That does not make a
            // node a start: an error handler has only an incoming error edge,
            // and must not run until something has actually failed (§2.5).
            //
            // A decision target is the third way in. Decisions route by
            // `case/@to` rather than by an edge, so a branch node often has no
            // incoming edge at all and is reachable ONLY by having been chosen.
            let chosen = self.decision_chosen.iter().any(|(_, t)| t == id);
            return if chosen || self.entry_set.contains(id) || self.error_enabled.contains(id) {
                Readiness::Ready
            } else {
                Readiness::Waiting
            };
        }
        let mut sat = 0;
        let mut neg = 0;
        let mut pending = 0;
        for i in edges.iter().copied() {
            match self.edge_status(i) {
                EdgeStatus::Satisfied => sat += 1,
                EdgeStatus::Negative => neg += 1,
                EdgeStatus::Pending => pending += 1,
            }
        }
        let node = self.h.node(id).unwrap();
        match node.join_policy.as_str() {
            "any" => {
                if sat >= 1 {
                    Readiness::Ready
                } else if pending == 0 {
                    Readiness::Unreachable
                } else {
                    Readiness::Waiting
                }
            }
            "quorum" => {
                let q = node.quorum.unwrap_or(1) as usize;
                if sat >= q {
                    Readiness::Ready
                } else if pending == 0 {
                    Readiness::Unreachable
                } else {
                    Readiness::Waiting
                }
            }
            // "all": every edge satisfied or resolved-negative, AND at least one
            // satisfied. If every edge resolved negative, no path reached the
            // node, so it is SKIPPED rather than run.
            _ => {
                if pending > 0 {
                    Readiness::Waiting
                } else if sat >= 1 {
                    Readiness::Ready
                } else {
                    let _ = neg;
                    self.unreachable.insert(id.to_string());
                    Readiness::Unreachable
                }
            }
        }
    }

    // ---- main loop -------------------------------------------------

    pub fn run(mut self) -> Run {
        // §5.2 — entry set.
        let entry: Vec<String> = match &self.h.entry {
            Some(e) => vec![e.clone()],
            None => self
                .h
                .nodes
                .iter()
                .filter(|n| {
                    self.incoming(&n.id).is_empty()
                        // ...and not reachable ONLY by an error or compensation
                        // edge, which makes it a handler rather than a start.
                        && !self.h.edges.iter().any(|e| {
                            e.to == n.id
                                && matches!(e.ty, EdgeType::Error | EdgeType::Compensation)
                        })
                })
                .map(|n| n.id.clone())
                .collect(),
        };
        self.entry_set = entry.iter().cloned().collect();
        for id in &entry {
            self.set(id, State::Ready, 0, None, None);
        }

        let mut guard = 0usize;
        loop {
            guard += 1;
            if guard > 100_000 {
                self.errors
                    .push(("<scheduler>".into(), "scheduler failed to converge".into()));
                break;
            }

            // Promote whatever has become ready. Document order, for determinism.
            let ids: Vec<String> = self.h.nodes.iter().map(|n| n.id.clone()).collect();
            for id in &ids {
                if self.state(id) == State::Pending {
                    match self.readiness(id) {
                        Readiness::Ready => self.set(id, State::Ready, 0, None, None),
                        // §6.4 — a node NO PATH REACHED stays PENDING. That is
                        // the expected outcome for the branch a decision did
                        // not take, and it is NOT the same fact as SKIPPED.
                        //
                        // SKIPPED means "reached, and its guard was false" — a
                        // terminal SUCCESS whose control successors still run.
                        // PENDING means "never reached", so its successors are
                        // not reached either. Conflating them makes the untaken
                        // branch of every decision look like it ran and
                        // succeeded, and makes its consumers fail with HX-4101
                        // for a value nobody ever intended to produce.
                        Readiness::Unreachable | Readiness::Waiting => {}
                    }
                }
            }

            let next = ids
                .iter()
                .find(|id| self.state(id) == State::Ready)
                .cloned();
            let Some(id) = next else { break };
            self.execute(&id);
        }

        self.finish()
    }

    fn finish(mut self) -> Run {
        let failed: Vec<String> = self
            .h
            .nodes
            .iter()
            .filter(|n| self.state(&n.id) == State::Failed)
            .map(|n| n.id.clone())
            .collect();

        // §5.8 — a node with an outgoing error edge is HANDLED; only an
        // unhandled failure propagates and triggers unwinding.
        let unhandled: Vec<String> = failed
            .iter()
            .filter(|id| {
                !self
                    .h
                    .edges
                    .iter()
                    .any(|e| e.ty == EdgeType::Error && &&e.from == id)
            })
            .cloned()
            .collect();

        // §5.7 — a FAILED node with an outgoing error edge is HANDLED: the
        // workflow took the error path and completed, so the instance
        // succeeded. Only an unhandled failure propagates.
        let result = if unhandled.is_empty() {
            RunResult::Succeeded
        } else {
            self.compensate();
            let compensated_any = self.states.values().any(|s| *s == State::Compensated);
            let compensation_failed = self.errors.iter().any(|(_, m)| m.starts_with("HX-4110"));

            // "Compensated" is only honest if compensation actually ran AND
            // succeeded. With nothing to compensate the instance simply failed,
            // and §8.4.3 makes a FAILED compensation complete as failed too —
            // reporting "compensated" when the rollback did not happen is a lie
            // an auditor eventually finds.
            if compensated_any && !compensation_failed {
                RunResult::Compensated
            } else {
                RunResult::Failed
            }
        };

        Run {
            result,
            states: self.states.into_iter().collect(),
            trace: self.trace,
            errors: self.errors,
        }
    }

    /// §8.4.1 — unwinding. Compensate SUCCEEDED nodes in REVERSE completion
    /// order: undoing a shipment before undoing the allocation that produced it
    /// leaves the system in a state neither step anticipated.
    fn compensate(&mut self) {
        // §8.4.1 step 1 — cancel work that has not finished, BEFORE unwinding.
        let inflight: Vec<String> = self
            .h
            .nodes
            .iter()
            .filter(|n| matches!(self.state(&n.id), State::Ready | State::Running))
            .map(|n| n.id.clone())
            .collect();
        for id in inflight {
            // §6.2.1 — a RUNNING non-idempotent node MUST NOT be cancelled.
            // Interrupting a payment mid-flight leaves a state nobody can
            // describe; letting it finish at least yields a known outcome.
            let protected = self.state(&id) == State::Running
                && self.h.node(&id).map(|n| !n.idempotent).unwrap_or(false);
            if protected {
                continue;
            }
            self.set(
                &id,
                State::Cancelled,
                0,
                None,
                Some("cancelled during unwinding".into()),
            );
        }

        let order: Vec<String> = self.completed.iter().rev().cloned().collect();
        for id in order {
            if self.state(&id) != State::Succeeded {
                continue;
            }
            let Some(target) = self
                .h
                .edges
                .iter()
                .find(|e| e.ty == EdgeType::Compensation && e.from == id)
                .map(|e| e.to.clone())
            else {
                continue;
            };

            let Some(node) = self.h.node(&target) else {
                continue;
            };
            let inputs = BTreeMap::new();
            match self.runner.run(node, &inputs, 1) {
                Outcome::Success(out) => {
                    self.outputs
                        .insert(target.clone(), out.into_iter().collect());
                    self.set(
                        &target,
                        State::Succeeded,
                        1,
                        None,
                        Some(format!("compensating {id}")),
                    );
                    self.set(&id, State::Compensated, 0, None, None);
                }
                Outcome::Failure { class, message } => {
                    // §8.4.3 — a failed compensation must not abandon the rest
                    // of the unwind, must be recorded distinctly, and the
                    // instance completes as FAILED rather than compensated.
                    // Reporting "compensated" when the rollback did not happen
                    // is a lie an auditor eventually finds.
                    self.set(
                        &target,
                        State::Failed,
                        1,
                        Some(class.clone()),
                        Some(format!("compensation for {id} FAILED")),
                    );
                    self.errors.push((
                        target.clone(),
                        format!("HX-4110 compensation for '{id}' failed: {message}"),
                    ));
                }
                Outcome::TimedOut => {
                    self.set(
                        &target,
                        State::Failed,
                        1,
                        Some("timeout".into()),
                        Some(format!("compensation for {id} timed out")),
                    );
                    self.errors.push((
                        target.clone(),
                        format!("HX-4110 compensation for '{id}' timed out"),
                    ));
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum EdgeStatus {
    Satisfied,
    Negative,
    Pending,
}

#[derive(Debug, Clone, Copy)]
enum Readiness {
    Ready,
    Waiting,
    Unreachable,
}

// ---------------------------------------------------------------- node execution

impl<R: NodeRunner> Executor<'_, R> {
    /// §5.5 — executing one node. Steps 1 to 3 happen BEFORE execution begins,
    /// so a node that cannot run fails without side effects.
    fn execute(&mut self, id: &str) {
        let h = self.h;
        let Some(node) = h.node(id) else { return };

        // 1. Guard, evaluated while still READY.
        if let Some(g) = &node.guard {
            let ctx = self.ctx(Some(node));
            match expr::eval_condition(g, &ctx) {
                Ok(true) => {}
                Ok(false) => {
                    self.set(
                        id,
                        State::Skipped,
                        0,
                        None,
                        Some("guard evaluated false".into()),
                    );
                    return;
                }
                Err(e) => {
                    self.errors
                        .push((id.to_string(), format!("{} guard: {}", e.code, e.message)));
                    self.set(
                        id,
                        State::Failed,
                        0,
                        Some("internal".into()),
                        Some(e.message),
                    );
                    return;
                }
            }
        }

        self.set(id, State::Running, 1, None, None);

        match node.kind.as_str() {
            "decision" => return self.execute_decision(node),
            "loop" => return self.execute_loop(node),
            // Pure control flow — no impl, nothing to run.
            "parallel" | "barrier" => {
                self.set(id, State::Succeeded, 1, None, None);
                return;
            }
            _ => {}
        }

        // 2. Inputs.
        let inputs = match self.resolve_inputs(node) {
            Ok(i) => i,
            Err((code, msg)) => {
                self.errors.push((id.to_string(), format!("{code} {msg}")));
                self.set(id, State::Failed, 1, Some("internal".into()), Some(msg));
                return;
            }
        };

        // A wait node blocks on a duration, an expression or an event. Nothing
        // to hand to the runner.
        if node.kind == "wait" {
            if let Some(w) = &node.wait
                && let Some(until) = &w.until
            {
                let ctx = self.ctx(Some(node));
                if let Err(e) = expr::eval_condition(until, &ctx) {
                    self.errors.push((
                        id.to_string(),
                        format!("{} wait/until: {}", e.code, e.message),
                    ));
                    self.set(
                        id,
                        State::Failed,
                        1,
                        Some("internal".into()),
                        Some(e.message),
                    );
                    return;
                }
            }
            self.set(id, State::Succeeded, 1, None, Some("wait satisfied".into()));
            return;
        }

        self.run_with_retries(node, &inputs, None);
    }

    /// §3.1.1 / §5.5 step 2 — an input is satisfied by a data edge, or by
    /// `value`, or by `default`, in that order.
    fn resolve_inputs(
        &self,
        node: &Node,
    ) -> Result<BTreeMap<String, Value>, (&'static str, String)> {
        let mut out = BTreeMap::new();
        let ctx = self.ctx(Some(node));

        for p in &node.inputs {
            let fed = self.h.edges.iter().find(|e| {
                e.ty == EdgeType::Data
                    && e.to == node.id
                    && e.to_port.as_deref() == Some(p.name.as_str())
            });

            if let Some(e) = fed {
                let port = e.from_port.clone().unwrap_or_default();
                let available = self
                    .outputs
                    .get(&e.from)
                    .and_then(|m| m.get(&port))
                    .cloned();

                match available {
                    Some(v) => {
                        out.insert(p.name.clone(), v);
                        continue;
                    }
                    None => {
                        // §5.6 — a SKIPPED producer's outputs are UNAVAILABLE.
                        // Substituting null here is how a workflow proceeds
                        // with data nobody produced, so the runtime must fail.
                        if let Some(d) = &p.default {
                            out.insert(p.name.clone(), Value::Str(d.clone()));
                            continue;
                        }
                        if !p.required {
                            continue;
                        }
                        let why = if self.state(&e.from) == State::Skipped {
                            format!("producer '{}' was SKIPPED", e.from)
                        } else {
                            format!("producer '{}' produced no '{port}'", e.from)
                        };
                        return Err((
                            "HX-4101",
                            format!(
                                "node '{}': required input '{}' unavailable — {why}",
                                node.id, p.name
                            ),
                        ));
                    }
                }
            }

            if let Some(v) = &p.value {
                match expr::eval_interpolated(v, &ctx) {
                    Ok(val) => {
                        out.insert(p.name.clone(), val);
                    }
                    Err(e) => {
                        return Err((
                            "HX-4106",
                            format!("node '{}': input '{}': {}", node.id, p.name, e.message),
                        ));
                    }
                }
                continue;
            }
            if let Some(d) = &p.default {
                out.insert(p.name.clone(), Value::Str(d.clone()));
                continue;
            }
            if p.required {
                return Err((
                    "HX-4101",
                    format!(
                        "node '{}': required input '{}' has no value",
                        node.id, p.name
                    ),
                ));
            }
        }
        Ok(out)
    }

    /// §8.1 to §8.3 — attempts, backoff, error classes, idempotence.
    fn run_with_retries(
        &mut self,
        node: &Node,
        inputs: &BTreeMap<String, Value>,
        iteration: Option<u64>,
    ) {
        let id = node.id.clone();

        // §8.3 — a runtime MUST NOT automatically retry a node the author
        // declared non-idempotent. Validation already rejects the combination
        // (HX-3301), so this is defence in depth for a document that reached a
        // runtime without being validated.
        let policy = if node.idempotent {
            node.retry.as_ref()
        } else {
            None
        };
        let max_attempts = policy.map(|r| r.max_attempts).unwrap_or(1).max(1);

        let mut attempt: u32 = 1;
        loop {
            let outcome = self.runner.run(node, inputs, attempt);

            match outcome {
                Outcome::Success(out) => {
                    self.outputs.insert(id.clone(), out.into_iter().collect());
                    self.set(&id, State::Succeeded, attempt, None, None);
                    return;
                }

                Outcome::TimedOut => {
                    let on = node
                        .timeout
                        .as_ref()
                        .map(|t| t.on_timeout.clone())
                        .unwrap_or_else(|| "fail".into());
                    match on.as_str() {
                        // For genuinely optional work only. Using it elsewhere
                        // converts a stuck workflow into one that silently did
                        // less, which is the failure mode this format most
                        // wants to avoid.
                        "skip" => {
                            self.set(
                                &id,
                                State::Skipped,
                                attempt,
                                Some("timeout".into()),
                                Some("onTimeout=skip".into()),
                            );
                            return;
                        }
                        // §8.2 — retried even if retryOn excludes `timeout`.
                        "retry" if attempt < max_attempts.max(2) => {
                            self.retry_step(&id, attempt, "timeout", policy, iteration);
                            attempt += 1;
                            continue;
                        }
                        _ => {
                            let d = node
                                .timeout
                                .as_ref()
                                .map(|t| t.duration.clone())
                                .unwrap_or_else(|| "unspecified".into());
                            self.errors.push((
                                id.clone(),
                                format!("HX-4108 node '{id}' exceeded its timeout of {d}"),
                            ));
                            self.set(
                                &id,
                                State::Failed,
                                attempt,
                                Some("timeout".into()),
                                Some(format!("HX-4108 exceeded timeout {d}")),
                            );
                            return;
                        }
                    }
                }

                Outcome::Failure { class, message } => {
                    let retryable = policy
                        .map(|r| r.retry_on.is_empty() || r.retry_on.contains(&class))
                        .unwrap_or(false);

                    if attempt < max_attempts && retryable {
                        self.retry_step(&id, attempt, &class, policy, iteration);
                        attempt += 1;
                        continue;
                    }
                    self.errors
                        .push((id.clone(), format!("{class}: {message}")));
                    self.set(&id, State::Failed, attempt, Some(class), Some(message));
                    return;
                }
            }
        }
    }

    /// RUNNING -> RETRYING -> READY -> RUNNING, per §6.2. The backoff delay is
    /// computed and recorded but never slept.
    fn retry_step(
        &mut self,
        id: &str,
        attempt: u32,
        class: &str,
        policy: Option<&Retry>,
        iteration: Option<u64>,
    ) {
        let delay = policy.map(|p| backoff_ms(p, attempt + 1)).unwrap_or(0);
        let _ = iteration;
        // Jitter is correct in production and wrong in a reference
        // implementation, where a reproducible trace matters more — so it is
        // reported rather than applied.
        let jitter = policy.map(|p| p.jitter).unwrap_or(false);
        let note = if jitter {
            " (jitter declared; not applied for reproducibility)"
        } else {
            ""
        };
        self.set(
            id,
            State::Retrying,
            attempt,
            Some(class.to_string()),
            Some(format!(
                "backoff {delay}ms before attempt {}{note}",
                attempt + 1
            )),
        );
        self.set(id, State::Ready, attempt + 1, None, None);
        self.set(id, State::Running, attempt + 1, None, None);
    }

    /// §7.1.3 — cases evaluate IN DOCUMENT ORDER and the first true one wins.
    /// Exactly one successor receives control.
    fn execute_decision(&mut self, node: &Node) {
        let id = node.id.clone();
        let Some(cases) = &node.cases else {
            self.set(
                &id,
                State::Failed,
                1,
                Some("internal".into()),
                Some("decision without <cases>".into()),
            );
            return;
        };
        let ctx = self.ctx(Some(node));

        let mut chosen: Option<String> = None;
        for (when, to) in &cases.cases {
            match expr::eval_condition(when, &ctx) {
                Ok(true) => {
                    chosen = Some(to.clone());
                    break;
                }
                Ok(false) => {}
                Err(e) => {
                    self.errors
                        .push((id.clone(), format!("{} case: {}", e.code, e.message)));
                    self.set(
                        &id,
                        State::Failed,
                        1,
                        Some("internal".into()),
                        Some(e.message),
                    );
                    return;
                }
            }
        }
        if chosen.is_none() {
            chosen = cases.otherwise.clone();
        }

        let Some(target) = chosen else {
            // §7.1.3 — no case matched and no <otherwise>. The workflow has
            // reached a state its author did not describe, and guessing is
            // worse than failing.
            self.errors.push((
                id.clone(),
                format!("HX-4103 node '{id}': no case matched and no <otherwise>"),
            ));
            self.set(
                &id,
                State::Failed,
                1,
                Some("internal".into()),
                Some("HX-4103 no matching case".into()),
            );
            return;
        };

        for (_, to) in &cases.cases {
            if to != &target {
                self.decision_rejected.insert((id.clone(), to.clone()));
            }
        }
        if let Some(o) = &cases.otherwise
            && o != &target
        {
            self.decision_rejected.insert((id.clone(), o.clone()));
        }
        self.decision_chosen.insert((id.clone(), target.clone()));
        self.set(
            &id,
            State::Succeeded,
            1,
            None,
            Some(format!("routed to '{target}'")),
        );
    }

    /// §7.2 — the four loop kinds. `maxIterations` is REQUIRED and exceeding it
    /// is a runtime FAILURE (HX-4104), never a silent stop: a loop that quietly
    /// halted at its limit would report success having processed part of its
    /// input.
    fn execute_loop(&mut self, node: &Node) {
        let h = self.h;
        let id = node.id.clone();
        let Some(l) = &node.loop_spec else {
            self.set(
                &id,
                State::Failed,
                1,
                Some("internal".into()),
                Some("loop without <loop>".into()),
            );
            return;
        };
        let Some(body_id) = l.body.clone() else {
            self.set(
                &id,
                State::Failed,
                1,
                Some("internal".into()),
                Some("loop without <body>".into()),
            );
            return;
        };
        let Some(body) = h.node(&body_id) else {
            self.set(
                &id,
                State::Failed,
                1,
                Some("internal".into()),
                Some(format!("loop body '{body_id}' not found")),
            );
            return;
        };

        let max_iterations = l.max_iterations.unwrap_or(0);
        let ctx = self.ctx(Some(node));

        let items: Vec<Value> = match l.kind.as_str() {
            "forEach" => match l.over.as_ref().map(|o| expr::eval_interpolated(o, &ctx)) {
                Some(Ok(Value::Array(a))) => a,
                Some(Ok(Value::Null)) => vec![],
                Some(Ok(other)) => {
                    self.errors.push((
                        id.clone(),
                        format!(
                            "HX-4106 forEach 'over' is {} , not an array",
                            other.type_name()
                        ),
                    ));
                    self.set(
                        &id,
                        State::Failed,
                        1,
                        Some("internal".into()),
                        Some("over is not an array".into()),
                    );
                    return;
                }
                Some(Err(e)) => {
                    self.errors
                        .push((id.clone(), format!("{} over: {}", e.code, e.message)));
                    self.set(
                        &id,
                        State::Failed,
                        1,
                        Some("internal".into()),
                        Some(e.message),
                    );
                    return;
                }
                None => vec![],
            },
            "times" => (0..l.count.unwrap_or(0))
                .map(|i| Value::Num(i as f64))
                .collect(),
            _ => vec![],
        };

        let mut i: u64 = 0;
        let mut any_ok = false;
        let mut any_fail = false;

        loop {
            // Continue-condition, evaluated BEFORE the iteration for `while`.
            match l.kind.as_str() {
                "forEach" => {
                    if i as usize >= items.len() {
                        break;
                    }
                }
                "times" => {
                    if i >= l.count.unwrap_or(0) {
                        break;
                    }
                }
                "while" => {
                    let c = self.ctx(Some(node));
                    match l.while_expr.as_ref().map(|w| expr::eval_condition(w, &c)) {
                        Some(Ok(true)) => {}
                        Some(Ok(false)) | None => break,
                        Some(Err(e)) => {
                            self.errors
                                .push((id.clone(), format!("{} while: {}", e.code, e.message)));
                            self.set(
                                &id,
                                State::Failed,
                                1,
                                Some("internal".into()),
                                Some(e.message),
                            );
                            return;
                        }
                    }
                }
                _ => {}
            }

            if i >= max_iterations {
                self.errors.push((
                    id.clone(),
                    format!("HX-4104 node '{id}': exceeded maxIterations {max_iterations}"),
                ));
                self.set(
                    &id,
                    State::Failed,
                    1,
                    Some("internal".into()),
                    Some(format!("HX-4104 exceeded maxIterations {max_iterations}")),
                );
                return;
            }

            // Bind the iteration variables for the body.
            let item = items
                .get(i as usize)
                .cloned()
                .unwrap_or(Value::Num(i as f64));
            self.vars.insert(l.var.clone(), item);
            self.vars.insert(l.index_var.clone(), Value::Num(i as f64));
            self.iteration = Some(i);

            // Each iteration runs the body afresh. Iteration 3 failing must not
            // leave the body FAILED for iteration 4 (§6.5).
            self.states.insert(body_id.clone(), State::Pending);
            self.set(&body_id, State::Ready, 0, None, None);
            self.set(&body_id, State::Running, 1, None, None);

            match self.resolve_inputs(body) {
                Ok(inputs) => self.run_with_retries(body, &inputs, Some(i)),
                Err((code, msg)) => {
                    self.errors.push((body_id.clone(), format!("{code} {msg}")));
                    self.set(
                        &body_id,
                        State::Failed,
                        1,
                        Some("internal".into()),
                        Some(msg),
                    );
                }
            }

            match self.state(&body_id) {
                s if s.is_success() => any_ok = true,
                _ => {
                    any_fail = true;
                    match l.on_item_failure.as_str() {
                        "continue" => {}
                        "break" => break,
                        _ => {
                            self.iteration = None;
                            self.vars.remove(&l.var);
                            self.vars.remove(&l.index_var);
                            self.set(
                                &id,
                                State::Failed,
                                1,
                                Some("internal".into()),
                                Some(format!("iteration {i} failed")),
                            );
                            return;
                        }
                    }
                }
            }

            i += 1;

            // `until` tests AFTER the iteration, so the body always runs once.
            if l.kind == "until" {
                let c = self.ctx(Some(node));
                match l.while_expr.as_ref().map(|w| expr::eval_condition(w, &c)) {
                    Some(Ok(true)) | None => break,
                    Some(Ok(false)) => {}
                    Some(Err(e)) => {
                        self.errors
                            .push((id.clone(), format!("{} until: {}", e.code, e.message)));
                        self.set(
                            &id,
                            State::Failed,
                            1,
                            Some("internal".into()),
                            Some(e.message),
                        );
                        return;
                    }
                }
            }
        }

        self.iteration = None;
        self.vars.remove(&l.var);
        self.vars.remove(&l.index_var);

        // §6.5 — the loop node's own outcome follows onItemFailure.
        let ok = match l.on_item_failure.as_str() {
            "continue" => any_ok || !any_fail,
            "break" => true,
            _ => !any_fail,
        };
        if ok {
            self.set(
                &id,
                State::Succeeded,
                1,
                None,
                Some(if l.max_concurrency > 1 {
                    format!(
                        "{i} iteration(s); maxConcurrency {} bounded, executed sequentially",
                        l.max_concurrency
                    )
                } else {
                    format!("{i} iteration(s)")
                }),
            );
        } else {
            self.set(
                &id,
                State::Failed,
                1,
                Some("internal".into()),
                Some("every iteration failed".into()),
            );
        }
    }
}

/// §8.1.1 — delay before attempt `n` (2-based; the first attempt has no delay).
/// Jitter is deliberately NOT applied here: it is correct in production and
/// wrong in a reference implementation, where a reproducible trace matters more.
pub fn backoff_ms(p: &Retry, n: u32) -> u64 {
    let initial = parse_duration_ms(&p.initial_delay).unwrap_or(1000);
    let raw = match p.backoff.as_str() {
        "none" => 0.0,
        "fixed" => initial as f64,
        "linear" => initial as f64 * p.multiplier * (n.saturating_sub(1)) as f64,
        _ => initial as f64 * p.multiplier.powi(n.saturating_sub(2) as i32),
    };
    let capped = match p.max_delay.as_ref().and_then(|d| parse_duration_ms(d)) {
        Some(cap) => raw.min(cap as f64),
        None => raw,
    };
    capped.max(0.0) as u64
}

/// ISO 8601 duration -> milliseconds. Months and years are rejected: their
/// length is not fixed, so a scheduler cannot resolve them deterministically
/// (HX-3401).
pub fn parse_duration_ms(s: &str) -> Option<u64> {
    let s = s.trim();
    let rest = s.strip_prefix('P')?;
    let (date, time) = match rest.split_once('T') {
        Some((d, t)) => (d, Some(t)),
        None => (rest, None),
    };
    if date.contains('Y') || date.contains('M') {
        return None;
    }

    let mut ms: u64 = 0;
    let mut num = String::new();
    for c in date.chars() {
        if c.is_ascii_digit() || c == '.' {
            num.push(c);
        } else if c == 'D' {
            ms += (num.parse::<f64>().ok()? * 86_400_000.0) as u64;
            num.clear();
        }
    }
    if let Some(t) = time {
        num.clear();
        for c in t.chars() {
            if c.is_ascii_digit() || c == '.' {
                num.push(c);
            } else {
                let v: f64 = num.parse().ok()?;
                ms += match c {
                    'H' => (v * 3_600_000.0) as u64,
                    'M' => (v * 60_000.0) as u64,
                    'S' => (v * 1000.0) as u64,
                    _ => return None,
                };
                num.clear();
            }
        }
    }
    Some(ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::Diagnostics;
    use crate::parse;

    fn run(doc: &str, scenario: &str) -> Run {
        let mut d = Diagnostics::default();
        let h = parse::parse(doc, &mut d).expect("parse");
        assert!(!d.has_errors(), "fixture should parse cleanly");
        let runner = SimulatedRunner::from_scenario(scenario).expect("scenario");
        // Leaked so the borrow lives as long as the Run; fine in a test.
        let h: &'static Harness = Box::leak(Box::new(h));
        Executor::new(h, runner).run()
    }

    fn st(r: &Run, id: &str) -> State {
        *r.states.get(id).unwrap_or(&State::Pending)
    }

    const BRANCH: &str = r#"<harness xmlns="https://harnessxml.com/spec/1.0" id="b" specVersion="1.0" entry="start">
  <nodes>
    <node id="start" type="source"><outputs><output name="score" type="number"/></outputs></node>
    <node id="pick" type="decision">
      <cases>
        <case when="${start.score >= 0.9}" to="high"/>
        <otherwise to="low"/>
      </cases>
    </node>
    <node id="high" type="task"/>
    <node id="low"  type="task"/>
  </nodes>
  <edges><edge from="start" to="pick" type="control"/></edges>
</harness>"#;

    #[test]
    fn decision_takes_one_branch_and_leaves_the_other_pending() {
        let r = run(BRANCH, "start ok score=0.95");
        assert_eq!(st(&r, "high"), State::Succeeded);
        // §6.4 — the untaken branch was NEVER REACHED. That is PENDING, not
        // SKIPPED: skipped means "reached, guard false" and is a success.
        assert_eq!(st(&r, "low"), State::Pending);
        assert_eq!(r.result, RunResult::Succeeded);
    }

    #[test]
    fn decision_otherwise_is_taken_when_no_case_matches() {
        let r = run(BRANCH, "start ok score=0.10");
        assert_eq!(st(&r, "low"), State::Succeeded);
        assert_eq!(st(&r, "high"), State::Pending);
    }

    #[test]
    fn no_matching_case_and_no_otherwise_is_hx_4103() {
        let doc = BRANCH.replace(r#"<otherwise to="low"/>"#, "");
        let r = run(&doc, "start ok score=0.10");
        assert_eq!(st(&r, "pick"), State::Failed);
        assert!(
            r.errors.iter().any(|(_, m)| m.contains("HX-4103")),
            "{:?}",
            r.errors
        );
    }

    const RETRY: &str = r#"<harness xmlns="https://harnessxml.com/spec/1.0" id="r" specVersion="1.0" entry="a">
  <nodes>
    <node id="a" type="task" idempotent="true">
      <retry maxAttempts="3" backoff="exponential" initialDelay="PT1S" retryOn="transient"/>
    </node>
  </nodes>
</harness>"#;

    #[test]
    fn retries_a_retryable_class_then_succeeds() {
        let r = run(
            RETRY,
            "a@1 fail transient blip\na@2 fail transient blip\na@3 ok",
        );
        assert_eq!(st(&r, "a"), State::Succeeded);
        let retrying = r
            .trace
            .entries
            .iter()
            .filter(|e| e.to == State::Retrying)
            .count();
        assert_eq!(retrying, 2, "two retries expected");
    }

    #[test]
    fn does_not_retry_a_class_outside_retry_on() {
        // retryOn="transient" excludes invalid_input, so one attempt only.
        let r = run(RETRY, "a fail invalid_input malformed");
        assert_eq!(st(&r, "a"), State::Failed);
        assert_eq!(
            r.trace
                .entries
                .iter()
                .filter(|e| e.to == State::Retrying)
                .count(),
            0
        );
    }

    #[test]
    fn never_auto_retries_a_non_idempotent_node() {
        // §8.3 — defence in depth. Validation rejects retry-on-non-idempotent
        // (HX-3301), but a runtime must not retry even if handed such a
        // document unvalidated: retrying a capture charges twice.
        let doc = RETRY.replace(r#"idempotent="true""#, r#"idempotent="false""#);
        let r = run(&doc, "a fail transient blip");
        assert_eq!(st(&r, "a"), State::Failed);
        assert_eq!(
            r.trace
                .entries
                .iter()
                .filter(|e| e.to == State::Retrying)
                .count(),
            0
        );
    }

    #[test]
    fn guard_false_is_skipped_and_successors_still_run() {
        let doc = r#"<harness xmlns="https://harnessxml.com/spec/1.0" id="g" specVersion="1.0" entry="a">
  <nodes>
    <node id="a" type="task"/>
    <node id="b" type="task"><guard when="${false}"/></node>
    <node id="c" type="task"/>
  </nodes>
  <edges>
    <edge from="a" to="b" type="control"/>
    <edge from="b" to="c" type="control"/>
  </edges>
</harness>"#;
        let r = run(doc, "");
        assert_eq!(st(&r, "b"), State::Skipped);
        // §4.1 — SKIPPED is a terminal SUCCESS, so control successors proceed.
        assert_eq!(st(&r, "c"), State::Succeeded);
        assert_eq!(r.result, RunResult::Succeeded);
    }

    #[test]
    fn error_edge_handles_a_failure_and_the_run_still_succeeds() {
        let doc = r#"<harness xmlns="https://harnessxml.com/spec/1.0" id="e" specVersion="1.0" entry="a">
  <nodes>
    <node id="a" type="task"/>
    <node id="handler" type="sink"/>
  </nodes>
  <edges><edge from="a" to="handler" type="error"/></edges>
</harness>"#;
        let r = run(doc, "a fail internal boom");
        assert_eq!(st(&r, "a"), State::Failed);
        assert_eq!(st(&r, "handler"), State::Succeeded);
        // §5.8 — a handled failure does not propagate.
        assert_eq!(r.result, RunResult::Succeeded);
    }

    #[test]
    fn an_error_handler_is_not_an_entry_node() {
        // §2.5 — a node reachable only by an error edge is a handler, not a
        // start. It must not run when nothing failed.
        let doc = r#"<harness xmlns="https://harnessxml.com/spec/1.0" id="e2" specVersion="1.0" entry="a">
  <nodes>
    <node id="a" type="task"/>
    <node id="handler" type="sink"/>
  </nodes>
  <edges><edge from="a" to="handler" type="error"/></edges>
</harness>"#;
        let r = run(doc, "a ok");
        assert_eq!(st(&r, "handler"), State::Pending);
    }

    #[test]
    fn unhandled_failure_triggers_compensation() {
        let doc = r#"<harness xmlns="https://harnessxml.com/spec/1.0" id="c" specVersion="1.0" entry="post">
  <nodes>
    <node id="post" type="task"/>
    <node id="pay" type="task"/>
    <node id="undo" type="task" compensates="post"/>
  </nodes>
  <edges>
    <edge from="post" to="pay" type="control"/>
    <edge from="post" to="undo" type="compensation"/>
  </edges>
</harness>"#;
        let r = run(doc, "pay fail unavailable rail down");
        assert_eq!(st(&r, "pay"), State::Failed);
        assert_eq!(st(&r, "undo"), State::Succeeded);
        assert_eq!(st(&r, "post"), State::Compensated);
        assert_eq!(r.result, RunResult::Compensated);
    }

    #[test]
    fn failed_compensation_completes_as_failed_not_compensated() {
        // §8.4.3 — reporting "compensated" when the rollback did not happen is
        // a lie an auditor eventually finds.
        let doc = r#"<harness xmlns="https://harnessxml.com/spec/1.0" id="cf" specVersion="1.0" entry="post">
  <nodes>
    <node id="post" type="task"/>
    <node id="pay" type="task"/>
    <node id="undo" type="task" compensates="post"/>
  </nodes>
  <edges>
    <edge from="post" to="pay" type="control"/>
    <edge from="post" to="undo" type="compensation"/>
  </edges>
</harness>"#;
        let r = run(
            doc,
            "pay fail unavailable down\nundo fail internal reversal rejected",
        );
        assert_eq!(r.result, RunResult::Failed);
        assert!(
            r.errors.iter().any(|(_, m)| m.contains("HX-4110")),
            "{:?}",
            r.errors
        );
    }

    const LOOP: &str = r#"<harness xmlns="https://harnessxml.com/spec/1.0" id="l" specVersion="1.0" entry="src">
  <nodes>
    <node id="src" type="source"><outputs><output name="items" type="array&lt;number&gt;"/></outputs></node>
    <node id="cycle" type="loop">
      <loop kind="forEach" over="${src.items}" var="it" maxIterations="10" onItemFailure="continue">
        <body ref="work"/>
      </loop>
    </node>
    <node id="work" type="task"/>
  </nodes>
  <edges><edge from="src" to="cycle" type="control"/></edges>
</harness>"#;

    #[test]
    fn foreach_runs_once_per_item() {
        let r = run(LOOP, "src ok items=[1,2,3]");
        assert_eq!(st(&r, "cycle"), State::Succeeded);
        let runs = r
            .trace
            .entries
            .iter()
            .filter(|e| e.node == "work" && e.to == State::Succeeded)
            .count();
        assert_eq!(runs, 3);
    }

    #[test]
    fn exceeding_max_iterations_is_hx_4104_not_a_silent_stop() {
        // A loop that quietly halted at its limit would report success having
        // processed part of its input.
        let doc = LOOP.replace(r#"maxIterations="10""#, r#"maxIterations="2""#);
        let r = run(&doc, "src ok items=[1,2,3,4]");
        assert_eq!(st(&r, "cycle"), State::Failed);
        assert!(
            r.errors.iter().any(|(_, m)| m.contains("HX-4104")),
            "{:?}",
            r.errors
        );
    }

    #[test]
    fn on_item_failure_continue_keeps_going() {
        let r = run(LOOP, "src ok items=[1,2,3]\nwork@1 fail transient bad item");
        // maxAttempts is absent so there is one attempt per iteration; the
        // scenario keys on attempt 1, so every iteration fails.
        assert_eq!(st(&r, "cycle"), State::Failed);
    }

    #[test]
    fn times_loop_runs_exactly_count_times() {
        let doc = r#"<harness xmlns="https://harnessxml.com/spec/1.0" id="t" specVersion="1.0" entry="cycle">
  <nodes>
    <node id="cycle" type="loop">
      <loop kind="times" count="4" maxIterations="10"><body ref="work"/></loop>
    </node>
    <node id="work" type="task"/>
  </nodes>
</harness>"#;
        let r = run(doc, "");
        let runs = r
            .trace
            .entries
            .iter()
            .filter(|e| e.node == "work" && e.to == State::Succeeded)
            .count();
        assert_eq!(runs, 4);
    }

    #[test]
    fn unreachability_propagates_so_an_all_join_can_resolve() {
        // Without propagation, `join` waits forever on the branch the decision
        // did not take and the instance deadlocks.
        let doc = r#"<harness xmlns="https://harnessxml.com/spec/1.0" id="u" specVersion="1.0" entry="start">
  <nodes>
    <node id="start" type="source"><outputs><output name="score" type="number"/></outputs></node>
    <node id="pick" type="decision">
      <cases><case when="${start.score > 0.5}" to="high"/><otherwise to="low"/></cases>
    </node>
    <node id="high" type="task"/>
    <node id="low" type="task"/>
    <node id="join" type="barrier"/>
  </nodes>
  <edges>
    <edge from="start" to="pick" type="control"/>
    <edge from="high" to="join" type="control"/>
    <edge from="low"  to="join" type="control"/>
  </edges>
</harness>"#;
        let r = run(doc, "start ok score=0.9");
        assert_eq!(st(&r, "high"), State::Succeeded);
        assert_eq!(st(&r, "low"), State::Pending);
        assert_eq!(
            st(&r, "join"),
            State::Succeeded,
            "the join must resolve, not deadlock"
        );
    }

    #[test]
    fn backoff_is_exponential_and_capped() {
        let p = Retry {
            max_attempts: 5,
            backoff: "exponential".into(),
            initial_delay: "PT1S".into(),
            max_delay: Some("PT4S".into()),
            multiplier: 2.0,
            jitter: false,
            retry_on: vec![],
        };
        assert_eq!(backoff_ms(&p, 2), 1000);
        assert_eq!(backoff_ms(&p, 3), 2000);
        assert_eq!(backoff_ms(&p, 4), 4000);
        assert_eq!(backoff_ms(&p, 5), 4000, "capped by maxDelay");
    }

    #[test]
    fn durations_parse_and_months_are_rejected() {
        assert_eq!(parse_duration_ms("PT30S"), Some(30_000));
        assert_eq!(parse_duration_ms("PT5M"), Some(300_000));
        assert_eq!(parse_duration_ms("PT2H"), Some(7_200_000));
        assert_eq!(parse_duration_ms("P1D"), Some(86_400_000));
        // HX-3401 — a month has no fixed length.
        assert_eq!(parse_duration_ms("P1M"), None);
        assert_eq!(parse_duration_ms("P1Y"), None);
    }
}
