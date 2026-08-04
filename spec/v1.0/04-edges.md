---
title: Typed Relationships
description: The five HarnessXML edge types — control, data, dependency, error and compensation — and what each one means to the scheduler.
section: specification
order: 5
status: draft
---

# 4. Typed Relationships

An edge is a directed relationship between two nodes. **The type determines what
the scheduler does.** Edge types are semantics, not diagram styling.

```xml
<edge id="e1" from="extract" to="classify" type="data"
      fromPort="text" toPort="text"/>
<edge id="e2" from="classify" to="notify" type="error"/>
```

| attribute | required | meaning |
|---|---|---|
| `from` | **yes** | source node `@id` |
| `to` | **yes** | target node `@id` |
| `type` | no, default `control` | one of the five below |
| `fromPort` / `toPort` | for `data` | the output and input ports bound |
| `condition` | no | expression guarding traversal |
| `id` | no | identifier, for diagnostics and diffs |

`@from` and `@to` are enforced by the schema (`xs:keyref`), so a dangling edge is
rejected by any plain schema-validating parser.

> Giving edges a single untyped meaning is the most common way a workflow format
> stops being executable. Once one arrow can mean "then", "feeds", "if this
> breaks" or "to undo this", every implementation resolves the ambiguity
> differently, and the picture no longer determines the behaviour.

## 4.1 `control`

Ordering. The target becomes ready when the source reaches a **terminal
successful** state — `SUCCEEDED` or `SKIPPED`.

```xml
<edge from="classify" to="route" type="control"/>
```

`SKIPPED` counting as success is deliberate and easy to get wrong. A guarded node
whose guard was false has *completed correctly* — it did what the document asked.
Treating skip as failure would mean any optional step halted everything after it.

A `control` edge carries no data. If the target needs a value, use `data`.

## 4.2 `data`

Dataflow. **Implies control**, and additionally binds an upstream output port to
a downstream input port.

```xml
<edge from="extract" to="classify" type="data"
      fromPort="text" toPort="text"/>
```

Requirements:

- `fromPort` and `toPort` are **required** on a data edge (`HX-2301`).
- `fromPort` **MUST** name an output on the source node (`HX-2302`).
- `toPort` **MUST** name an input on the target node (`HX-2303`).
- At most **one** data edge may target a given input (`HX-2304`) — two producers
  for one input has no defined winner, so it is invalid rather than resolved by a
  rule nobody would remember.
- An input may not be fed by both a data edge and a port `value` (`HX-2102`).

**Type compatibility.** When both ports declare a `type`, they **MUST** be
compatible (`HX-3201`). Compatible means identical, or the target is `json` (which
accepts anything structured), or the source is a subtype under a rule the
runtime documents. If either port omits `type`, no check is performed — untyped
is permitted, and it means "unchecked", not "any".

Because data implies control, an explicit `control` edge alongside a `data` edge
between the same pair is redundant. It is not an error, but a validator **SHOULD**
warn: redundancy in a graph is how readers come to believe there are two
relationships.

## 4.3 `dependency`

Must-complete-before, with **no data transfer and no implied success**. Satisfied
by *any* terminal state — `SUCCEEDED`, `SKIPPED`, `FAILED` or `CANCELLED`.

```xml
<edge from="release_gripper" to="scan_bin" type="dependency"/>
```

This is the edge for "these must not overlap" rather than "this must work first".
Use it when the ordering constraint is about a **shared resource or physical
reality** rather than about a result:

- do not scan the bin while the gripper is still closing — even if closing failed;
- do not start the migration while the backup job is running — however it ended;
- do not power the sensor while the actuator is drawing current.

Using `control` in these cases produces a workflow that deadlocks on the first
failure it should have tolerated.

## 4.4 `error`

Taken **only** when the source reaches `FAILED` after its retry policy is
exhausted.

```xml
<edge from="classify" to="notify_failure" type="error"/>
```

- Never traversed on `SUCCEEDED`, `SKIPPED` or `CANCELLED`.
- Never traversed while retries remain — an error edge fires once, at the end,
  not on each failed attempt.
- A node MAY have several outgoing error edges; **all** are taken.
- A node reachable *only* by error edges is a handler, and does not count as an
  entry point (see [chapter 2 §2.5](/spec/v1.0/document-structure/#2-5-entry-points)).

If a node has at least one outgoing error edge, its failure is **handled**: the
workflow continues down the error path rather than terminating. With no error
edge, failure propagates per [chapter 8](/spec/v1.0/failure/).

## 4.5 `compensation`

The rollback path. Traversed **only** during compensation — never during forward
execution.

```xml
<edge from="post_ledger" to="reverse_entry" type="compensation"/>
```

The direction reads *forward*: `from` is the action, `to` is the undo. The
scheduler traverses it in reverse, when the workflow is unwinding.

- The target **SHOULD** declare `compensates="<source id>"`. When it does and the
  two disagree, the document is invalid (`HX-2004`).
- The target **MUST NOT** be reachable by forward control or data edges
  (`HX-2005`). A node that is both a normal step and a rollback will eventually
  run at the wrong time.
- Compensation nodes **SHOULD** be idempotent. Compensation runs during failure
  handling, which is exactly when a runtime is most likely to try again.

Full unwinding semantics — what gets compensated, and in what order — are in
[chapter 8](/spec/v1.0/failure/).

## 4.6 Conditions on edges

Any edge MAY carry a `condition`. The edge is traversed only if it evaluates
true:

```xml
<edge from="review" to="gate" type="control" condition="${review.approved}"/>
```

**Choosing between an edge condition, a guard and a decision** is the question
this specification is asked most:

| construct | question it answers | effect |
|---|---|---|
| `<guard>` on a node | "should this node run at all?" | node becomes `SKIPPED`; control successors still run |
| edge `condition` | "should this particular path be taken?" | that edge is not traversed |
| `decision` node | "which *one* of these paths?" | exactly one successor, chosen deterministically |

Use a **decision** when the alternatives are mutually exclusive and a reader
should see them listed together. Use an **edge condition** when one path among
several independent ones is optional. Use a **guard** when a single step is
conditional and everything downstream proceeds regardless.

## 4.7 Cycles

**Control flow must be acyclic** (`HX-3003`). A cycle formed by `control`, `data`
or `dependency` edges is invalid.

Repetition is expressed with a `loop` node, which is bounded by a required
`maxIterations`. This is the trade the specification makes: cycles in the graph
would allow unbounded repetition that no validator can detect, while a loop node
makes the bound explicit and checkable.

`error` and `compensation` edges are **excluded** from the acyclicity check. An
error handler may legitimately point back at an earlier node, and a compensation
edge points backwards by definition.

## 4.8 Worked example

```xml
<edges>
  <!-- data implies control: classify runs after extract, and gets its text -->
  <edge from="extract"  to="classify" type="data" fromPort="text" toPort="text"/>

  <!-- pure ordering -->
  <edge from="classify" to="route"    type="control"/>

  <!-- ordering that must hold however the source ended -->
  <edge from="release"  to="scan"     type="dependency"/>

  <!-- only after retries are exhausted -->
  <edge from="classify" to="page_oncall" type="error"/>

  <!-- rollback, traversed backwards during unwinding -->
  <edge from="post"     to="reverse"  type="compensation"/>
</edges>
```

Five arrows, five distinct meanings, none of which a reader has to infer from
context.
