//! The validation rules of specification chapter 13.
//!
//! Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0
//!
//! Every rule here carries the code the specification assigns it, and every
//! code has a conformance fixture that must be rejected with exactly that code.
//! That correspondence is the whole point of the reference implementation: a
//! disagreement about what the specification means can be settled by reading
//! running code rather than by arguing about prose.

use crate::diag::{Diagnostic, Diagnostics};
use crate::model::*;
use std::collections::{HashMap, HashSet};

pub fn validate(h: &Harness, diags: &mut Diagnostics) {
    document(h, diags);
    identifiers(h, diags);
    references(h, diags);
    node_shape(h, diags);
    ports_and_edges(h, diags);
    graph(h, diags);
    policy(h, diags);
    credentials(h, diags);
}

// ---------------------------------------------------------------- HX-1xxx

fn document(h: &Harness, diags: &mut Diagnostics) {
    if h.spec_version.is_none() {
        diags.push(Diagnostic::error(
            "HX-1002",
            1,
            "<harness> has no specVersion; a runtime cannot safely guess which semantics apply",
        ));
    }
    if h.nodes.is_empty() {
        diags.push(Diagnostic::error(
            "HX-1102",
            1,
            "<nodes> must contain at least one <node>",
        ));
    }
    for n in &h.nodes {
        if !NODE_TYPES.contains(&n.kind.as_str()) {
            diags.push(Diagnostic::error(
                "HX-1003",
                n.line,
                format!(
                    "node '{}': unrecognised type '{}'. A runtime must refuse rather than skip it",
                    n.id, n.kind
                ),
            ));
        }
    }
}

fn identifiers(h: &Harness, diags: &mut Diagnostics) {
    let dup =
        |kind: &str, seen: &mut HashSet<String>, id: &str, line: usize, d: &mut Diagnostics| {
            if id.is_empty() {
                return;
            }
            if !seen.insert(id.to_string()) {
                d.push(Diagnostic::error(
                    "HX-1101",
                    line,
                    format!("duplicate {kind} id '{id}'; every reference to it would be ambiguous"),
                ));
            }
        };

    let mut s = HashSet::new();
    for n in &h.nodes {
        dup("node", &mut s, &n.id, n.line, diags);
    }
    let mut s = HashSet::new();
    for r in &h.resources {
        dup("resource", &mut s, &r.id, r.line, diags);
    }
    let mut s = HashSet::new();
    for a in &h.artifacts {
        dup("artifact", &mut s, &a.id, a.line, diags);
    }
    let mut s = HashSet::new();
    for e in h.edges.iter().filter(|e| e.id.is_some()) {
        dup("edge", &mut s, e.id.as_ref().unwrap(), e.line, diags);
    }

    for n in &h.nodes {
        for (dir, ports) in [("input", &n.inputs), ("output", &n.outputs)] {
            let mut seen = HashSet::new();
            for p in ports {
                if !seen.insert(p.name.clone()) {
                    diags.push(Diagnostic::error(
                        "HX-1101",
                        p.line,
                        format!("node '{}': duplicate {dir} port '{}'", n.id, p.name),
                    ));
                }
            }
        }
    }
}

// ---------------------------------------------------------------- HX-2xxx

fn references(h: &Harness, diags: &mut Diagnostics) {
    let nodes: HashSet<&str> = h.nodes.iter().map(|n| n.id.as_str()).collect();
    let resources: HashSet<&str> = h.resources.iter().map(|r| r.id.as_str()).collect();
    let artifacts: HashSet<&str> = h.artifacts.iter().map(|a| a.id.as_str()).collect();

    for e in &h.edges {
        for (which, id) in [("from", &e.from), ("to", &e.to)] {
            if !nodes.contains(id.as_str()) {
                diags.push(Diagnostic::error(
                    "HX-2001",
                    e.line,
                    format!(
                        "edge{}: {which} names '{id}', which is not a declared node",
                        e.id.as_ref().map(|i| format!(" '{i}'")).unwrap_or_default()
                    ),
                ));
            }
        }
    }

    for n in &h.nodes {
        if let Some(c) = &n.cases {
            for (_, to) in &c.cases {
                if !nodes.contains(to.as_str()) {
                    diags.push(Diagnostic::error(
                        "HX-2001",
                        c.line,
                        format!(
                            "node '{}': case targets '{to}', which is not a declared node",
                            n.id
                        ),
                    ));
                }
            }
            if let Some(o) = &c.otherwise
                && !nodes.contains(o.as_str())
            {
                diags.push(Diagnostic::error(
                    "HX-2001",
                    c.line,
                    format!(
                        "node '{}': otherwise targets '{o}', which is not a declared node",
                        n.id
                    ),
                ));
            }
        }
        if let Some(l) = &n.loop_spec
            && let Some(b) = &l.body
            && !nodes.contains(b.as_str())
        {
            diags.push(Diagnostic::error(
                "HX-2001",
                l.line,
                format!(
                    "node '{}': loop body references '{b}', which is not a declared node",
                    n.id
                ),
            ));
        }
        for r in &n.resource_refs {
            if !resources.contains(r.target.as_str()) {
                diags.push(Diagnostic::error(
                    "HX-2002",
                    r.line,
                    format!(
                        "node '{}': resourceRef '{}' is not a declared resource",
                        n.id, r.target
                    ),
                ));
            }
        }
        for a in &n.artifact_refs {
            if !artifacts.contains(a.target.as_str()) {
                diags.push(Diagnostic::error(
                    "HX-2003",
                    a.line,
                    format!(
                        "node '{}': artifactRef '{}' is not a declared artifact",
                        n.id, a.target
                    ),
                ));
            }
        }
        if let Some(c) = &n.compensates
            && !nodes.contains(c.as_str())
        {
            diags.push(Diagnostic::error(
                "HX-2004",
                n.line,
                format!(
                    "node '{}': compensates '{c}', which is not a declared node",
                    n.id
                ),
            ));
        }
    }
}

fn node_shape(h: &Harness, diags: &mut Diagnostics) {
    for n in &h.nodes {
        let is = |k: &str| n.kind == k;

        if n.cases.is_some() != is("decision") {
            let (code, msg) = if is("decision") {
                (
                    "HX-2201",
                    format!("node '{}': type=\"decision\" requires <cases>", n.id),
                )
            } else {
                (
                    "HX-2201",
                    format!(
                        "node '{}': <cases> is only permitted on type=\"decision\", not \"{}\"",
                        n.id, n.kind
                    ),
                )
            };
            diags.push(Diagnostic::error(code, n.line, msg));
        }
        if n.loop_spec.is_some() != is("loop") {
            diags.push(Diagnostic::error(
                "HX-2202",
                n.line,
                format!("node '{}': <loop> belongs on type=\"loop\" and nowhere else (this node is \"{}\")", n.id, n.kind),
            ));
        }
        if n.subworkflow.is_some() != is("subworkflow") {
            diags.push(Diagnostic::error(
                "HX-2203",
                n.line,
                format!(
                    "node '{}': <subworkflow> belongs on type=\"subworkflow\" and nowhere else",
                    n.id
                ),
            ));
        }
        if n.wait.is_some() != is("wait") {
            diags.push(Diagnostic::error(
                "HX-2204",
                n.line,
                format!(
                    "node '{}': <wait> belongs on type=\"wait\" and nowhere else",
                    n.id
                ),
            ));
        }

        if let Some(w) = &n.wait {
            let count =
                w.duration.is_some() as u8 + w.until.is_some() as u8 + w.event.is_some() as u8;
            if count != 1 {
                diags.push(Diagnostic::error(
                    "HX-2205",
                    w.line,
                    format!("node '{}': <wait> must declare exactly one of duration, until or event (found {count})", n.id),
                ));
            }
        }

        if let Some(c) = &n.cases {
            if c.cases.is_empty() {
                diags.push(Diagnostic::error(
                    "HX-2206",
                    c.line,
                    format!("node '{}': <cases> must contain at least one <case>", n.id),
                ));
            }
            if c.otherwise.is_none() {
                diags.push(Diagnostic::warning(
                    "HX-4103",
                    c.line,
                    format!(
                        "node '{}': no <otherwise>; if no case matches at runtime this fails with HX-4103",
                        n.id
                    ),
                ));
            }
        }

        if let Some(l) = &n.loop_spec {
            if l.max_iterations.is_none() {
                diags.push(Diagnostic::error(
                    "HX-1001",
                    l.line,
                    format!(
                        "node '{}': loop has no maxIterations. There is no unbounded form — an unbounded loop in an unattended workflow is a defect",
                        n.id
                    ),
                ));
            }
            let missing = match l.kind.as_str() {
                "forEach" => l.over.is_none().then_some("over"),
                "while" | "until" => l.while_expr.is_none().then_some("while"),
                "times" => l.count.is_none().then_some("count"),
                other => {
                    diags.push(Diagnostic::error(
                        "HX-1003",
                        l.line,
                        format!("node '{}': unrecognised loop kind '{other}'", n.id),
                    ));
                    None
                }
            };
            if let Some(attr) = missing {
                diags.push(Diagnostic::error(
                    "HX-2207",
                    l.line,
                    format!(
                        "node '{}': loop kind=\"{}\" requires the '{attr}' attribute",
                        n.id, l.kind
                    ),
                ));
            }
            if let (Some(c), Some(m)) = (l.count, l.max_iterations)
                && c > m
            {
                diags.push(Diagnostic::error(
                        "HX-2208",
                        l.line,
                        format!("node '{}': count {c} exceeds maxIterations {m}; the document states two different bounds", n.id),
                    ));
            }
            if l.body.is_none() {
                diags.push(Diagnostic::error(
                    "HX-1001",
                    l.line,
                    format!("node '{}': <loop> has no <body>", n.id),
                ));
            }
        }

        let incoming = h.edges.iter().filter(|e| e.to == n.id).count();
        if n.join_policy == "quorum" {
            match n.quorum {
                None => diags.push(Diagnostic::error(
                    "HX-2401",
                    n.line,
                    format!("node '{}': joinPolicy=\"quorum\" requires @quorum", n.id),
                )),
                Some(q) if q as usize > incoming => diags.push(Diagnostic::error(
                    "HX-2402",
                    n.line,
                    format!("node '{}': quorum {q} exceeds its {incoming} incoming edge(s); it can never be satisfied", n.id),
                )),
                _ => {}
            }
        }

        if is("inference") {
            let has_model = n.resource_refs.iter().any(|r| {
                h.resources
                    .iter()
                    .any(|res| res.id == r.target && res.kind == "model")
            });
            if !has_model {
                diags.push(Diagnostic::error(
                    "HX-2501",
                    n.line,
                    format!(
                        "node '{}': type=\"inference\" must reference a resource of type=\"model\"",
                        n.id
                    ),
                ));
            }
        }
    }
}

fn ports_and_edges(h: &Harness, diags: &mut Diagnostics) {
    // HX-2304: at most one data edge per input.
    let mut fed: HashMap<(&str, &str), usize> = HashMap::new();

    for e in &h.edges {
        if e.ty != EdgeType::Data {
            continue;
        }
        let (Some(fp), Some(tp)) = (&e.from_port, &e.to_port) else {
            diags.push(Diagnostic::error(
                "HX-2301",
                e.line,
                format!(
                    "edge {} -> {}: a data edge must declare both fromPort and toPort",
                    e.from, e.to
                ),
            ));
            continue;
        };
        if let Some(src) = h.node(&e.from)
            && src.output(fp).is_none()
        {
            diags.push(Diagnostic::error(
                "HX-2302",
                e.line,
                format!(
                    "edge {} -> {}: fromPort '{fp}' is not an output on '{}'",
                    e.from, e.to, e.from
                ),
            ));
        }
        if let Some(dst) = h.node(&e.to)
            && dst.input(tp).is_none()
        {
            diags.push(Diagnostic::error(
                "HX-2303",
                e.line,
                format!(
                    "edge {} -> {}: toPort '{tp}' is not an input on '{}'",
                    e.from, e.to, e.to
                ),
            ));
        }
        *fed.entry((e.to.as_str(), tp.as_str())).or_insert(0) += 1;

        // HX-3201 — checked only when BOTH ports declare a type. Untyped means
        // unchecked, not "any".
        if let (Some(src), Some(dst)) = (h.node(&e.from), h.node(&e.to))
            && let (Some(sp), Some(dp)) = (src.output(fp), dst.input(tp))
            && let (Some(st), Some(dt)) = (&sp.ty, &dp.ty)
            && st != dt
            && dt != "json"
        {
            diags.push(Diagnostic::error(
                "HX-3201",
                e.line,
                format!(
                    "edge {} -> {}: type '{st}' is not compatible with '{dt}'",
                    e.from, e.to
                ),
            ));
        }
    }

    for ((node, port), n) in &fed {
        if *n > 1 {
            let line = h.node(node).map(|x| x.line).unwrap_or(1);
            diags.push(Diagnostic::error(
                "HX-2304",
                line,
                format!("node '{node}': input '{port}' is fed by {n} data edges; there is no defined winner"),
            ));
        }
    }

    for node in &h.nodes {
        for p in &node.inputs {
            if !p.required {
                continue;
            }
            let by_edge = fed.contains_key(&(node.id.as_str(), p.name.as_str()));
            if !by_edge && !p.has_value && !p.has_default {
                diags.push(Diagnostic::error(
                    "HX-2101",
                    p.line,
                    format!(
                        "node '{}': required input '{}' is satisfied by neither a data edge nor a value",
                        node.id, p.name
                    ),
                ));
            }
            if by_edge && p.has_value {
                diags.push(Diagnostic::error(
                    "HX-2102",
                    p.line,
                    format!(
                        "node '{}': input '{}' has both a data edge and a value; a reader cannot tell which wins",
                        node.id, p.name
                    ),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------- HX-3xxx

fn graph(h: &Harness, diags: &mut Diagnostics) {
    // Entry set — §2.5.
    let entry: Vec<&str> = match &h.entry {
        Some(e) => vec![e.as_str()],
        None => h
            .nodes
            .iter()
            .filter(|n| !h.edges.iter().any(|e| e.ty.is_forward() && e.to == n.id))
            .map(|n| n.id.as_str())
            .collect(),
    };

    if entry.is_empty() && !h.nodes.is_empty() {
        diags.push(Diagnostic::error(
            "HX-3001",
            1,
            "the entry set is empty: every node waits for another, so nothing can begin",
        ));
    }

    // HX-3003 — acyclicity over forward edges only.
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in h.edges.iter().filter(|e| e.ty.is_forward()) {
        adj.entry(e.from.as_str()).or_default().push(e.to.as_str());
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        White,
        Grey,
        Black,
    }
    let mut mark: HashMap<&str, Mark> = h
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), Mark::White))
        .collect();
    let mut cycle: Option<String> = None;

    fn dfs<'a>(
        at: &'a str,
        adj: &HashMap<&'a str, Vec<&'a str>>,
        mark: &mut HashMap<&'a str, Mark>,
        stack: &mut Vec<&'a str>,
        cycle: &mut Option<String>,
    ) {
        if cycle.is_some() {
            return;
        }
        mark.insert(at, Mark::Grey);
        stack.push(at);
        for &next in adj.get(at).map(|v| v.as_slice()).unwrap_or(&[]) {
            match mark.get(next).copied().unwrap_or(Mark::White) {
                Mark::Grey => {
                    let start = stack.iter().position(|&x| x == next).unwrap_or(0);
                    let mut path: Vec<&str> = stack[start..].to_vec();
                    path.push(next);
                    *cycle = Some(path.join(" -> "));
                    return;
                }
                Mark::White => dfs(next, adj, mark, stack, cycle),
                Mark::Black => {}
            }
        }
        stack.pop();
        mark.insert(at, Mark::Black);
    }

    let ids: Vec<&str> = h.nodes.iter().map(|n| n.id.as_str()).collect();
    for id in &ids {
        if mark.get(id).copied().unwrap_or(Mark::White) == Mark::White {
            dfs(id, &adj, &mut mark, &mut Vec::new(), &mut cycle);
        }
    }
    if let Some(path) = cycle {
        diags.push(Diagnostic::error(
            "HX-3003",
            1,
            format!("control flow contains a cycle: {path}. Use a loop node, which carries a required bound"),
        ));
    }

    // HX-3004 — a loop body must not be reachable by CONTROL or DEPENDENCY
    // edges from outside the loop, which would make it a normal step as well as
    // a body and run it at the wrong time.
    //
    // `data` edges into a body are explicitly permitted: that is how a
    // loop-invariant input is bound, and forbidding them would make loops
    // nearly unusable. Every iteration sees the same value.
    for n in &h.nodes {
        if let Some(l) = &n.loop_spec
            && let Some(body) = &l.body
            && h.edges.iter().any(|e| {
                matches!(e.ty, EdgeType::Control | EdgeType::Dependency)
                    && &e.to == body
                    && e.from != n.id
            })
        {
            diags.push(Diagnostic::error(
                        "HX-3004",
                        l.line,
                        format!(
                            "node '{}': loop body '{body}' is also reachable by a forward edge from outside the loop; it would run at the wrong time",
                            n.id
                        ),
                    ));
        }
    }

    // HX-2005 — a compensation target must not be reachable forward.
    for e in h.edges.iter().filter(|e| e.ty == EdgeType::Compensation) {
        if h.edges
            .iter()
            .any(|f| matches!(f.ty, EdgeType::Control | EdgeType::Data) && f.to == e.to)
        {
            diags.push(Diagnostic::error(
                "HX-2005",
                e.line,
                format!(
                    "node '{}' is a compensation target but is also reachable by a forward edge; it will eventually run at the wrong time",
                    e.to
                ),
            ));
        }
        if let Some(target) = h.node(&e.to)
            && let Some(c) = &target.compensates
            && c != &e.from
        {
            diags.push(Diagnostic::error(
                        "HX-2004",
                        e.line,
                        format!(
                            "node '{}' declares compensates=\"{c}\" but a compensation edge arrives from '{}'",
                            e.to, e.from
                        ),
                    ));
        }
    }

    // HX-3005 — reachability. A warning: legitimate mid-authoring.
    let mut seen: HashSet<&str> = HashSet::new();
    let mut queue: Vec<&str> = entry.clone();
    while let Some(at) = queue.pop() {
        if !seen.insert(at) {
            continue;
        }
        for e in h.edges.iter().filter(|e| e.from == at) {
            queue.push(e.to.as_str());
        }
        if let Some(n) = h.node(at) {
            if let Some(c) = &n.cases {
                for (_, to) in &c.cases {
                    queue.push(to.as_str());
                }
                if let Some(o) = &c.otherwise {
                    queue.push(o.as_str());
                }
            }
            if let Some(l) = &n.loop_spec
                && let Some(b) = &l.body
            {
                queue.push(b.as_str());
            }
        }
    }
    for n in &h.nodes {
        if !seen.contains(n.id.as_str()) {
            diags.push(Diagnostic::warning(
                "HX-3005",
                n.line,
                format!("node '{}' is not reachable from the entry set", n.id),
            ));
        }
    }
}

fn policy(h: &Harness, diags: &mut Diagnostics) {
    for n in &h.nodes {
        // HX-3301 — the contradiction the format makes unrepresentable.
        if !n.idempotent && n.has_retry {
            diags.push(Diagnostic::error(
                "HX-3301",
                n.line,
                format!(
                    "node '{}' is declared idempotent=\"false\" but carries a retry policy; retrying it duplicates its effect",
                    n.id
                ),
            ));
        }
        // HX-3401 — months and years have no fixed length, so a scheduler
        // cannot resolve them deterministically. In ISO 8601 the designators
        // before 'T' are the date part: 'M' there is months, 'M' after 'T' is
        // minutes and is fine.
        for (where_, d) in &n.durations {
            let date_part = d.split('T').next().unwrap_or("");
            if date_part.contains('Y') || date_part.contains('M') {
                diags.push(Diagnostic::error(
                    "HX-3401",
                    n.line,
                    format!(
                        "node '{}': {where_} duration '{d}' uses months or years, whose length is not fixed",
                        n.id
                    ),
                ));
            }
        }
    }
}

/// HX-3501 — a literal credential in a document that is designed to be
/// committed to git, diffed in pull requests and archived for audit.
///
/// Detection is necessarily heuristic, so a confident hit is an error and a
/// suspicious one is a warning (§13.3): a false positive that blocks a build is
/// worse than a warning nobody had to argue with.
fn credentials(h: &Harness, diags: &mut Diagnostics) {
    const CONFIDENT: &[&str] = &[
        "sk-ant-",
        "sk-",
        "AKIA",
        "ghp_",
        "github_pat_",
        "xoxb-",
        "AIza",
        "-----BEGIN",
    ];
    const SUSPICIOUS_NAMES: &[&str] =
        &["key", "secret", "token", "password", "passwd", "credential"];

    for s in &h.scalar_values {
        let value = s.value.trim();
        // An expression or a reference is exactly what the format wants.
        if value.is_empty() || value.starts_with("${") {
            continue;
        }
        if CONFIDENT.iter().any(|p| value.starts_with(p)) {
            diags.push(Diagnostic::error(
                "HX-3501",
                s.line,
                format!(
                    "{} appears to contain a literal credential. Use <credential ref=\"…\" store=\"…\"/> instead",
                    s.context
                ),
            ));
            continue;
        }
        let lname = s.name.to_ascii_lowercase();
        if SUSPICIOUS_NAMES.iter().any(|n| lname.contains(n)) && value.len() >= 20 {
            diags.push(Diagnostic::warning(
                "HX-3501",
                s.line,
                format!(
                    "{} is named like a secret and holds a long literal; if it is a credential, reference it instead",
                    s.context
                ),
            ));
        }
    }
}
