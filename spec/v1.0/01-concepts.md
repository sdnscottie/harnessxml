---
title: Concepts and Object Model
description: The HarnessXML object model — harness, node, port, edge, resource, artifact — and the vocabulary the rest of the specification is written in.
section: specification
order: 2
status: draft
---

# 1. Concepts and Object Model

This chapter defines the object model normatively. The XML serialisation in
[chapter 2](/spec/v1.0/document-structure/) is one encoding of it; two
implementations agree about what a document *means* by agreeing about this model,
before they argue about how to execute it.

## 1.1 The model at a glance

```
Harness
├── Metadata            title, authorship, provenance
├── Security            document-level principal and classification
├── Resource*           external capabilities: models, datastores, devices
├── Artifact*           identified data: datasets, documents, models, logs
├── Node+               units of work
│   ├── Port*           typed inputs and outputs
│   ├── Config          static properties
│   ├── ResourceRef*    which resources this node needs
│   ├── ArtifactRef*    which artifacts it reads or writes
│   ├── Guard?          conditional execution of this node
│   ├── Retry?          retry policy
│   ├── Timeout?        bound on a single attempt
│   └── (type-specific) Cases | Loop | Subworkflow | Wait
└── Edge*               typed relationships between nodes
```

## 1.2 Harness

A **harness** is the whole workflow: one document, one unit of versioning,
validation and execution.

A harness has a document-unique `id`, a `specVersion`, and OPTIONALLY an explicit
`entry` node. It contains at least one node.

A harness is **not** a namespace for other harnesses. Composition is by
`subworkflow` node, which references another harness by URI — so each harness
remains independently validatable. This is deliberate: a workflow that can only
be validated in the context of its parent cannot be reviewed on its own.

## 1.3 Node

A **node** is a unit of work with an identity, a type, and a declared interface.

Every node has:

- an `id`, unique within the harness;
- a `type` from the closed set in [chapter 3](/spec/v1.0/nodes/);
- OPTIONALLY an `impl` — an **opaque** handle the runtime resolves to something
  executable. The specification assigns it no structure and never interprets it.

A node also declares whether it is **idempotent**. This is central rather than
incidental: it is the author's statement about whether the node may be executed
more than once with the same inputs and the same net effect. A runtime MUST NOT
automatically retry a node declared `idempotent="false"`.

> A runtime cannot deduce idempotence. Only the author knows whether the call
> behind `impl` charges a card, moves an arm, or reads a row. So the format makes
> the author say it, and makes the unsafe combination — a retry policy on a
> non-idempotent node — invalid rather than merely discouraged (`HX-3301`).

### 1.3.1 Ports

A node's interface is its **ports**: named, optionally typed inputs and outputs.

An input is satisfied in exactly one of two ways:

1. by an incoming **data edge** binding an upstream output to it; or
2. by a `value` on the port itself — a literal or an expression.

An input that is `required` (the default) and satisfied by neither is invalid
(`HX-2101`). Ports are matched **by name**, never by position, so adding a port
does not silently rebind existing wiring.

### 1.3.2 Configuration

`<config>` carries static properties of the node — a system prompt, a threshold,
a target pose. Configuration is not a port: it is fixed at authoring time and
does not participate in dataflow.

The distinction matters for review. A change to configuration is a change to the
*design* and shows up in the diff. A change to a port value at runtime is data.

## 1.4 Edge

An **edge** is a directed, **typed** relationship between two nodes.

The type is not decoration. It determines what the scheduler does:

| type | meaning |
|---|---|
| `control` | ordering — the target becomes ready when the source succeeds |
| `data` | dataflow — implies control, and additionally binds an output port to an input port |
| `dependency` | must-complete-before, with no data and no implied success — satisfied by *any* terminal state |
| `error` | taken only when the source reaches `FAILED` after retries are exhausted |
| `compensation` | rollback path, traversed only during compensation — never during forward execution |

Full semantics are in [chapter 4](/spec/v1.0/edges/).

> Collapsing these into one untyped arrow is the single most common way a
> workflow format stops being executable. Once an arrow can mean "then", "feeds",
> "if this breaks" or "to undo this", every implementation resolves the ambiguity
> differently and the picture no longer determines the behaviour.

## 1.5 Resource

A **resource** is a capability the workflow needs but does not contain: a model
endpoint, a database, a queue, a robot arm, an external service, a secret store.

Resources are declared once at document level and referenced by nodes. That
indirection is the point: moving a workflow between development, staging and
production is a change to the resource block and **nothing else**. No node
changes, so the diff shows exactly what varies by environment.

A resource may carry a `<credential ref="..."/>` — a **reference** to a secret in
a store, never the secret. A document containing a literal credential is invalid
(`HX-3501`).

## 1.6 Artifact

An **artifact** is identified data that flows into or out of the workflow: a
dataset, a model file, a document, an image, a log, a report.

An artifact MAY carry a **digest**. When it does, the artifact is content-identified,
and this is what makes a run reproducible and an audit trail meaningful — a trace
naming `sha256:…` says exactly which bytes were processed, years later.

Artifacts differ from ports: a port carries a value *between nodes within one
execution*; an artifact identifies data that exists *independently* of any
execution. Confusing the two produces workflows that cannot be re-run.

## 1.7 Execution instance

Executing a harness produces an **execution instance**: a run, with its own
identity, its own node states, and its own resolved values.

The specification defines the *semantics* of an instance — the state machine,
readiness, scheduling constraints — and deliberately not its representation.
Whether a runtime stores state in memory, in a database, or in an event log is an
implementation decision, and one implementations should compete on.

## 1.8 What is not in the object model

Stated explicitly, because their absence is a design decision rather than an
omission:

**Presentation.** Coordinates, colours, sizes, groupings and routing of drawn
edges are authoring-tool concerns. They do not affect execution, and a runtime
MUST NOT depend on them. A designer needing to round-trip layout SHOULD carry it
in a vendor `<extension>` with `required="false"`.

**Step implementations.** `impl` is opaque, permanently.

**Scheduling policy.** The specification defines when a node *may* run
(readiness), not which ready node a runtime picks first, how work is distributed,
or how a crash is recovered.

**Global mutable state.** There are no workflow-level variables that any node may
write. State flows along edges. A workflow with hidden shared state cannot be
statically analysed — and static analysis before execution is the whole value
proposition.
