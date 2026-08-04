---
title: Execution Semantics
description: How a conforming runtime executes a HarnessXML document — readiness, scheduling, join policies, and what is deliberately left to the implementation.
section: specification
order: 6
status: draft
---

# 5. Execution Semantics

This chapter defines what a conforming runtime **must** do. Where it is silent,
implementations are free — and are expected to differ.

## 5.1 The execution instance

Executing a harness creates an **execution instance**: an identity, a state for
every node, and a set of resolved port values.

A runtime **MUST NOT** execute a document that fails validation
([chapter 13](/spec/v1.0/validation/)). Validation is a gate, not advice.

## 5.2 Starting

Execution begins by moving the **entry set** to `READY`:

- if `@entry` is present, that single node;
- otherwise, every node with no incoming `control`, `data` or `dependency` edge.

Incoming `error` and `compensation` edges do not count — a node reachable only by
an error edge is a handler, not a start.

## 5.3 Readiness

A node in `PENDING` becomes `READY` when its **join condition** over incoming
edges is satisfied. Only `control`, `data` and `dependency` edges participate.

An incoming edge is **satisfied** when:

| edge type | satisfied when the source is |
|---|---|
| `control` | `SUCCEEDED` or `SKIPPED` |
| `data` | `SUCCEEDED` or `SKIPPED`, **and** the bound value is available |
| `dependency` | in **any** terminal state — including `FAILED` and `CANCELLED` |

An edge carrying a `condition` that evaluates **false** is **not** satisfied, and
never will be. It is *resolved-negative*: it neither blocks forever nor counts
toward a join.

### 5.3.1 Join policies

`joinPolicy` on the target node decides how satisfied edges combine:

| policy | ready when |
|---|---|
| `all` (default) | every incoming edge is satisfied **or** resolved-negative, and at least one is satisfied |
| `any` | the first incoming edge is satisfied |
| `quorum` | at least `@quorum` incoming edges are satisfied |

`quorum` is required when `joinPolicy="quorum"`, and MUST NOT exceed the number
of incoming edges (`HX-2401`, `HX-2402`).

The "at least one satisfied" clause on `all` matters: if *every* incoming edge
resolved negative, no path reached the node, so it is `SKIPPED` rather than run.

### 5.3.2 Cancellation under `any`

When a node with `joinPolicy="any"` becomes ready, sibling branches still running
that could only have reached this node **MUST** be moved to `CANCELLED`.

A runtime **MUST NOT** cancel a node that is `RUNNING` and declared
`idempotent="false"` — it must let it finish and then discard the result. A
half-completed non-idempotent action is exactly the state the format exists to
avoid: interrupting a payment mid-flight leaves a question nobody can answer.

## 5.4 Scheduling

A runtime **MAY** execute any `READY` nodes in any order, and any number of them
concurrently — **subject to**:

1. **Loop concurrency.** A loop's `maxConcurrency` bounds concurrent iterations.
2. **Declared exclusivity.** A vendor extension MAY constrain further; the core
   specification has no global concurrency control.
3. **Determinism where it is observable.** Decision cases evaluate in document
   order; loop iterations with `maxConcurrency="1"` execute in sequence order.

Everything else is an implementation decision: which ready node to pick, how to
distribute work, how to recover from a crash. Runtimes should compete here.

> A workflow whose *result* depends on which ready node ran first has a race, and
> the specification will not paper over it. If ordering matters, express it with
> an edge.

## 5.5 Executing a node

For a node moving from `READY` to `RUNNING`:

1. **Evaluate the guard**, if present. If false, the node becomes `SKIPPED` —
   a terminal *successful* state — and its control successors proceed.
2. **Resolve inputs.** Each input takes the value from its incoming data edge, or
   from `value`, or from `default`. A required input with no value is a runtime
   error (`HX-4101`).
3. **Resolve resources.** Each `resourceRef` is resolved, credentials fetched
   from their declared store. Failure to resolve is `HX-4102`.
4. **Execute** via `impl`, subject to `<timeout>`.
5. **Bind outputs.** Declared outputs become available to outgoing data edges.
6. **Transition** to `SUCCEEDED` or `FAILED`.

Steps 1–3 happen **before** execution begins, so that a node which cannot run
fails without side effects.

## 5.6 Data availability

An output value is available to downstream nodes once its producer reaches
`SUCCEEDED`.

If a producer is `SKIPPED`, its outputs are **unavailable**. A downstream
required input bound to an unavailable output is a runtime error (`HX-4101`)
unless the input declares a `default`.

> This catches a real and common design mistake: guarding a node while leaving a
> required downstream consumer wired to its output. The document is statically
> valid; the workflow is wrong. A validator **SHOULD** warn about the pattern, and
> the runtime **MUST** fail rather than pass a null downstream — silently
> substituting null is how a workflow proceeds with data nobody produced.

Values are **immutable** once bound. There is no shared mutable state; a value
travels along an edge and cannot be modified in place by its consumer.

## 5.7 Completion

An execution instance is complete when no node is `PENDING`, `READY`, `RUNNING`
or `RETRYING`.

| outcome | when |
|---|---|
| **succeeded** | every reached node is `SUCCEEDED` or `SKIPPED` |
| **failed** | at least one node is `FAILED` with no outgoing `error` edge |
| **compensated** | a failure triggered unwinding and compensation completed |
| **cancelled** | terminated externally |

Nodes never reached remain `PENDING` at completion. That is normal, not an error:
a decision took one branch, so the other branch's nodes were never reached.

## 5.8 Failure propagation

When a node reaches `FAILED` and its retry policy is exhausted:

1. If it has outgoing `error` edges, **all** are traversed. The failure is
   **handled** and the workflow continues.
2. If it has none, the failure **propagates**: the runtime begins unwinding, and
   compensation runs per [chapter 8](/spec/v1.0/failure/).

## 5.9 What is deliberately unspecified

Naming these prevents an implementation from assuming a guarantee that was never
made:

**Persistence and recovery.** What happens when the runtime process dies
mid-execution is out of scope in v1.0. Implementations differ enormously here and
none of it changes what a document *means*. It is the question implementers ask
most, and a candidate for a future version — see the [roadmap](/roadmap/).

**Distribution.** Whether nodes run in one process, many, or across machines.

**Queueing and prioritisation.** Which ready node runs first when capacity is
limited.

**Observability format.** Traces are RECOMMENDED and their *content* is
constrained by [chapter 12](/spec/v1.0/provenance/), but the wire format is an
implementation choice.

**Resource pooling.** Whether two nodes referencing the same resource share a
connection.
