---
title: Conditionals and Loops
description: Guards, decision nodes, edge conditions and the four HarnessXML loop kinds — including why maxIterations is required.
section: specification
order: 8
status: draft
---

# 7. Conditionals and Loops

## 7.1 Three ways to be conditional

HarnessXML has three conditional constructs, and choosing between them is the
question this specification is asked most.

| construct | question | effect when false |
|---|---|---|
| `<guard>` on a node | "should this node run at all?" | node → `SKIPPED`; control successors **still run** |
| `condition` on an edge | "should this path be taken?" | that edge is not traversed |
| `decision` node | "which **one** of these?" | n/a — exactly one successor is always chosen |

### 7.1.1 Guards

```xml
<node id="match_po" type="task" impl="erp.match">
  <guard when="${extract.poNumber != null}"/>
</node>
```

The guard is evaluated when the node is `READY`, **before** inputs are resolved
and before any side effect. False → `SKIPPED`, which is a terminal *successful*
state ([chapter 6](/spec/v1.0/lifecycle/)).

Use a guard for a step that is optional in itself, where everything downstream
proceeds either way.

> A guard is the construct people reach for when they mean a decision. The test:
> if skipping this node should change *where control goes next*, you want a
> decision. If it should only change *whether this one thing happened*, you want
> a guard.

### 7.1.2 Edge conditions

```xml
<edge from="review" to="gate" type="control" condition="${review.approved}"/>
```

Evaluated when the source reaches a terminal state. False → the edge is
*resolved-negative*: it never becomes satisfied, and it neither blocks a join nor
counts toward one ([§5.3](/spec/v1.0/execution-semantics/#5-3-readiness)).

Use an edge condition when one path among several independent ones is optional.

### 7.1.3 Decision nodes

```xml
<node id="route" type="decision">
  <cases>
    <case when="${classify.confidence >= 0.90}" to="auto_file"/>
    <case when="${classify.confidence >= 0.60}" to="human_review"/>
    <otherwise to="quarantine"/>
  </cases>
</node>
```

**Cases are evaluated in document order. The first true case wins. Exactly one
successor receives control.**

Document order is normative, not an implementation detail. When an approval
threshold has to be explained to an auditor, "the first matching rule, reading
top to bottom" is an answer — "it depends on evaluation order" is not.

Rules:

- `<cases>` is required on `type="decision"` and forbidden elsewhere (`HX-2201`).
- At least one `<case>` (`HX-2206`).
- `<otherwise>` is optional but **RECOMMENDED**. Without one, a decision where no
  case matches is a runtime error (`HX-4103`) — the workflow has reached a state
  its author did not describe, and guessing is worse than failing.
- Every `@to` must name a node in the harness (schema-enforced).
- Case targets **SHOULD** be distinct. Two cases routing to the same node is
  legal but usually means the conditions should have been combined.

A decision node takes no `impl`. It is pure control flow.

## 7.2 Loops

```xml
<node id="pick_cycle" type="loop">
  <loop kind="forEach"
        over="${detect.grasps}"
        var="grasp" indexVar="i"
        maxIterations="24"
        maxConcurrency="1"
        onItemFailure="continue">
    <body ref="grasp_part"/>
  </loop>
</node>
```

`<loop>` is required on `type="loop"` and forbidden elsewhere (`HX-2202`).

### 7.2.1 `maxIterations` is required

There is no unbounded loop. No `maxIterations="unlimited"`, no omitting it.

This is the specification's most opinionated constraint, and it is deliberate. An
unbounded loop in a workflow that runs unattended — driving an arm, pushing
configuration to a fleet, calling a metered API — is a defect, not a feature. The
common outcome is not an infinite loop; it is a very expensive finite one that
nobody bounded because the library defaulted to forever.

Exceeding `maxIterations` is a runtime failure (`HX-4104`), never a silent stop.
A loop that quietly halted at its limit would report success having processed
part of its input.

### 7.2.2 The four kinds

| kind | requires | iterates |
|---|---|---|
| `forEach` | `over` | once per element of the collection |
| `while` | `while` | while the expression is true, tested **before** each iteration |
| `until` | `while` | until the expression is true, tested **after** each iteration (so at least once) |
| `times` | `count` | exactly `count` times |

Missing the attribute a kind requires is invalid (`HX-2207`).

```xml
<loop kind="forEach" over="${rows}" var="row" maxIterations="10000"><body ref="process"/></loop>
<loop kind="while"   while="${!sensor.settled}" maxIterations="120"><body ref="poll"/></loop>
<loop kind="until"   while="${queue.empty}"     maxIterations="500"><body ref="drain"/></loop>
<loop kind="times"   count="3"                  maxIterations="3"><body ref="sample"/></loop>
```

For `times`, `count` **MUST NOT** exceed `maxIterations` (`HX-2208`) — otherwise
the document states two different bounds.

### 7.2.3 Iteration variables

`var` (default `item`) binds the current element; `indexVar` (default `index`)
binds the zero-based position. Both are visible to the body node and to anything
it evaluates, and **only** there:

```xml
<input name="pose" type="pose6d" value="${grasp}"/>
```

Referencing an iteration variable outside its loop body is invalid (`HX-3102`).

### 7.2.4 Concurrency

`maxConcurrency` (default `1`) bounds simultaneous iterations.

The default is sequential on purpose. Parallelism should be a decision someone
made and a reviewer saw, not something that happens because a library defaulted
to it — there is exactly one arm, and the fleet rollout that hits every switch at
once is the incident.

With `maxConcurrency > 1`, iterations may complete in any order. A workflow whose
result depends on completion order has a race the specification will not paper
over.

### 7.2.5 Failure within a loop

| `onItemFailure` | behaviour | loop node outcome |
|---|---|---|
| `fail` (default) | stop immediately | `FAILED` |
| `continue` | record the failure, keep going | `SUCCEEDED` if any iteration succeeded, else `FAILED` |
| `break` | stop, treat what completed as the result | `SUCCEEDED` |

`continue` is right when items are independent — picking twenty parts, where one
bad grasp should not end the shift. `break` is right when order matters and
stopping cleanly beats continuing past a problem — a staged rollout that should
halt at the first bad device.

### 7.2.6 The body

`<body ref="…"/>` names a node in the harness. That node:

- **MUST NOT** be the target of an incoming `control` or `dependency` edge from
  outside the loop (`HX-3004`). A body that is also a normal step in the main
  flow will run at the wrong time.
- **MAY** be the target of an incoming `data` edge from outside the loop. This is
  how a **loop-invariant input** is bound, and every iteration sees the same
  value.
- MAY have outgoing `error` edges, evaluated per iteration.
- MAY be referenced by more than one loop.

> The `data` exemption is deliberate and is the one place where "data implies
> control" ([§4.2](/spec/v1.0/edges/#4-2-data)) does **not** also imply
> membership of the main flow. Without it a body could receive nothing from
> outside the loop — no configuration, no target, no model — and loops would be
> nearly unusable. What the rule actually prevents is a body being *sequenced*
> from outside, which is what `control` and `dependency` express.
>
> This exemption exists because the reference implementation rejected a
> legitimate example under the original wording. That is what a reference
> implementation is for.

Nested loops are expressed by making the body a `loop` node itself. Each level
carries its own `maxIterations`, so the total is bounded by their product — a
number a reviewer can actually compute.

## 7.3 Parallel and barrier

```xml
<node id="fan" type="parallel"/>
<node id="join" type="barrier" joinPolicy="quorum" quorum="2"/>
```

`parallel` releases control along **all** outgoing control edges at once.
`barrier` joins per its `joinPolicy` — `all`, `any`, `quorum`
([§5.3.1](/spec/v1.0/execution-semantics/#5-3-1-join-policies)).

Both are pure control flow with no `impl`. They exist so that fan-out and fan-in
are **visible as nodes** rather than emergent from counting edges. A reader
should not have to work out that four outgoing arrows means a fan-out.

> `joinPolicy` is an attribute available on *every* node, so a barrier is not
> strictly necessary to join. Use an explicit `barrier` when the join is the
> point; use `joinPolicy` on a working node when the join is incidental to it.
