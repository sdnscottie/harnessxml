---
title: Nodes
description: The twelve HarnessXML node types, their ports, configuration, and the type-specific elements each requires.
section: specification
order: 4
status: draft
---

# 3. Nodes

## 3.1 Common structure

Every node, whatever its type:

```xml
<node id="classify" type="inference" name="Classify document"
      impl="visml.models.classify" idempotent="true">
  <description>…</description>
  <inputs>  <input  name="text"       type="string"/> </inputs>
  <outputs> <output name="category"   type="string"/> </outputs>
  <config>  <property name="temperature" value="0"/>  </config>
  <resourceRef ref="classifier" role="model"/>
  <artifactRef ref="taxonomy" direction="in"/>
  <guard when="${enabled}"/>
  <retry maxAttempts="4" backoff="exponential"/>
  <timeout duration="PT3M" onTimeout="retry"/>
  <!-- type-specific: cases | loop | subworkflow | wait -->
  <security classification="confidential"/>
  <extension namespace="…"/>
</node>
```

Child element order is fixed and schema-enforced, for the same reason document
sections are: a reviewer always finds the retry policy in the same place.

| attribute | default | meaning |
|---|---|---|
| `id` | — | required, unique among nodes |
| `type` | — | required, from §3.2 |
| `name` | — | human-readable; no semantics |
| `impl` | — | **opaque** handle the runtime resolves |
| `idempotent` | `true` | may this node be executed more than once with the same net effect? |
| `joinPolicy` | `all` | how incoming edges are joined — see [chapter 5](/spec/v1.0/execution-semantics/) |
| `quorum` | — | required when `joinPolicy="quorum"` (`HX-2401`) |
| `compensates` | — | marks this node as another node's compensating action |

### 3.1.1 `impl` is opaque

The specification assigns `impl` no structure and never interprets it. It may be
a function name, a container image, a URI, or anything else a runtime agrees with
its authors.

This is the boundary that keeps the specification from going stale. Every prior
attempt that also standardised *what steps do* became dated as soon as the
technology underneath moved — and in this field it moves constantly.

A node MAY omit `impl` entirely when its type fully determines behaviour:
`decision`, `loop`, `parallel` and `barrier` nodes are pure control flow.

### 3.1.2 `idempotent`

The author's statement about whether the node may run more than once with the
same inputs and the same net effect.

- `idempotent="true"` (default): a runtime **MAY** retry automatically.
- `idempotent="false"`: a runtime **MUST NOT** retry automatically.

A node with `idempotent="false"` **and** a `<retry>` policy is invalid
(`HX-3301`) — the combination is a contradiction, and making it unrepresentable
is better than documenting that it is unwise.

> This cannot be inferred. Only the author knows whether the call behind `impl`
> charges a card, moves an arm, or reads a row. Getting it wrong produces the
> duplicated payment and the second grasp on an already-held part.

## 3.2 Node types

The enumeration is **closed**. A runtime meeting an unrecognised type MUST reject
the document (`HX-1003`).

| type | purpose | requires |
|---|---|---|
| `task` | generic unit of work | — |
| `inference` | invocation of a model resource | a `resourceRef` to a `model` (`HX-2501`) |
| `transform` | pure function of its inputs | — |
| `decision` | routes control to exactly one successor | `<cases>` |
| `loop` | repeats a body node | `<loop>` |
| `parallel` | fans control out to all successors | — |
| `barrier` | join point | — |
| `subworkflow` | invokes another harness | `<subworkflow>` |
| `source` | boundary node producing data in | — |
| `sink` | boundary node consuming data out | — |
| `wait` | blocks on a duration or event | `<wait>` |
| `human` | human-in-the-loop decision | — |

A type-specific element on the wrong node type is invalid: `<cases>` outside a
`decision` is `HX-2201`, `<loop>` outside a `loop` is `HX-2202`, `<subworkflow>`
outside a `subworkflow` is `HX-2203`, `<wait>` outside a `wait` is `HX-2204`.

### 3.2.1 `task`

The default. A unit of work the runtime resolves through `impl`.

### 3.2.2 `inference`

A model invocation. Distinguished from `task` because it is the single most
common step in this domain and because runtimes need to treat it specially —
token accounting, rate limiting, prompt capture for provenance, caching.

A conforming runtime **SHOULD** record the resolved model identity in the
execution trace. Which model answered is exactly the question asked afterwards.

### 3.2.3 `transform`

A **pure** function of its inputs: no side effects, no external calls, no
dependence on wall-clock time.

Declaring purity buys two things: a transform is always freely retryable
regardless of `idempotent`, and a runtime **MAY** cache its output keyed on its
inputs. A `transform` that in fact has side effects is a defect in the document
that no validator can catch — which is why it is a distinct type rather than an
attribute, so the claim is conspicuous.

### 3.2.4 `decision`

Evaluates `<cases>` and routes control to **exactly one** successor. Cases are
evaluated **in document order; the first true case wins**. Full semantics in
[chapter 7](/spec/v1.0/control-flow/).

### 3.2.5 `loop`

Repeats a body node. `maxIterations` is **required** — there is no unbounded
form. See [chapter 7](/spec/v1.0/control-flow/).

### 3.2.6 `parallel` and `barrier`

`parallel` releases control along **all** outgoing control edges simultaneously.
`barrier` joins, according to its `joinPolicy` — `all`, `any` or `quorum`.

Both are pure control flow and take no `impl`. They exist so that fan-out and
fan-in are **visible in the graph** rather than emergent from edge counts.

### 3.2.7 `subworkflow`

Invokes another harness by URI:

```xml
<node id="sub" type="subworkflow">
  <subworkflow href="https://example.com/workflows/enrich.hxml"
               digest="sha256:…"
               specVersion="1.0"/>
</node>
```

A `digest` is **RECOMMENDED**. Without one, the meaning of the parent workflow
can change without the parent changing — which defeats both reproducibility and
review.

Recursion, direct or indirect, is invalid (`HX-3002`).

### 3.2.8 `source` and `sink`

Boundary nodes. A `source` produces data into the workflow from outside it; a
`sink` consumes data out of it.

They are distinct types rather than tasks-that-happen-to-be-at-the-edge because
the boundary is exactly where classification, provenance and permissions need to
be declared, and a reader should be able to see the edges of the system at a
glance.

### 3.2.9 `wait`

Blocks on a duration, an expression becoming true, or a named external event:

```xml
<node id="soak" type="wait"><wait duration="PT10M"/></node>
<node id="settle" type="wait"><wait until="${sensor.stable}"/></node>
<node id="approval" type="wait"><wait event="external.approval.received"/></node>
```

Exactly one of `duration`, `until` or `event` **MUST** be present (`HX-2205`).

### 3.2.10 `human`

Blocks for an out-of-band human decision. Almost always `idempotent="false"` — a
retry that silently enqueues a second review request is a common and confusing
defect.

A `human` node **SHOULD** declare a `<timeout>`. A workflow that waits forever on
a person who has left the company is a workflow nobody notices is stuck.

## 3.3 Ports

```xml
<inputs>
  <input name="text" type="string"/>
  <input name="threshold" type="number" required="false" default="0.9"/>
  <input name="taxonomy" type="json" value="${artifact('taxonomy')}"/>
</inputs>
<outputs>
  <output name="category" type="string"/>
</outputs>
```

| attribute | meaning |
|---|---|
| `name` | required; unique among ports of the same direction on that node |
| `type` | **open-ended** — the core names below are interoperable, but any string is legal |
| `required` | default `true`; a required input must be satisfied |
| `default` | value used when an optional input is unsatisfied |
| `value` | literal or expression bound directly, for an input with no data edge |

An input is satisfied by an incoming data edge **or** by `value`. Both at once is
invalid (`HX-2102`) — the reader cannot tell which wins, so neither does. A
required input satisfied by neither is invalid (`HX-2101`).

Interoperable core type names: `string`, `number`, `integer`, `boolean`, `json`,
`binary`, `date`, `dateTime`, `array<T>`, `map<K,V>`. Anything else is legal and
checked structurally only; see [chapter 4](/spec/v1.0/edges/) on data-edge
compatibility (`HX-3201`).

> Port types are open-ended on purpose. A robotics workflow needs `pose6d` and
> `pointcloud`; a closed type list would either exclude those domains or grow
> without bound. The cost — weaker cross-edge checking for domain types — is
> accepted, and a stronger optional type layer is on the [roadmap](/roadmap/).

## 3.4 Configuration versus ports

Both carry values into a node, and confusing them is common.

| | `<config>` | ports |
|---|---|---|
| when fixed | authoring time | execution time |
| shows in a diff | **yes** | no |
| participates in dataflow | no | yes |
| changed by | editing the document | upstream nodes |

Rule of thumb: **if changing it is a design decision, it is config. If it varies
per execution, it is a port.** A confidence threshold is config. The document
being classified is a port.
