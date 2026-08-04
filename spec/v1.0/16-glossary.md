---
title: Glossary
description: Normative definitions of every HarnessXML term.
section: specification
order: 17
status: draft
---

# 16. Glossary

These definitions are **normative**. Where the body of the specification and this
glossary appear to disagree, that is a defect — please
[report it](/contributing/).

---

**Artifact** — Identified data that exists independently of any one execution: a
dataset, model file, document, image, log or report. Declared at document level,
referenced by nodes, and optionally content-identified by a **digest**. Contrast
**port**.

**Attempt** — One execution of a node's `impl`. A node with `maxAttempts="4"` may
have up to four attempts: the first, plus three retries.

**Backoff** — The delay between attempts, computed per the node's retry policy:
`none`, `fixed`, `linear` or `exponential`, optionally with **jitter**.

**Barrier** — A node type whose purpose is to join incoming edges according to a
**join policy**. Pure control flow; takes no `impl`.

**Classification** — A sensitivity label — `public`, `internal`, `confidential`,
`restricted` — carried by an artifact or a `<security>` block. Declarative; the
specification defines no enforcement.

**Compensation** — The act of undoing a node that already **succeeded**, during
unwinding. Reached along a `compensation` edge, executed in reverse completion
order. A compensated node transitions `SUCCEEDED` → `COMPENSATED`.

**Conformance level** — `Core`, `Executing` or `Full`. Cumulative, and defined by
the published test suite rather than by agreement with any implementation.

**Control edge** — An edge asserting ordering. Its target becomes ready when its
source reaches a terminal *successful* state (`SUCCEEDED` or `SKIPPED`).

**Data edge** — An edge asserting dataflow. Implies control, and additionally
binds an output port to an input port. Requires `fromPort` and `toPort`.

**Decision** — A node type that evaluates cases **in document order** and routes
control to **exactly one** successor. The first true case wins.

**Dependency edge** — An edge asserting must-complete-before with no data and no
implied success. Satisfied by **any** terminal state, including `FAILED`.

**Digest** — A content hash, e.g. `sha256:…`, identifying exact bytes rather than
a name. What makes a run reproducible and an audit trail verifiable.

**Edge** — A directed, **typed** relationship between two nodes. The type —
`control`, `data`, `dependency`, `error`, `compensation` — determines what the
scheduler does.

**Entry set** — The nodes where execution begins: the single node named by
`@entry`, or every node with no incoming `control`, `data` or `dependency` edge.
Must not be empty (`HX-3001`).

**Erratum** — A dated correction to a released specification version, **appended**
to it. Released text is never edited in place.

**Error class** — A category assigned to a runtime failure — `transient`,
`rate_limit`, `invalid_input` and so on — matched by a retry policy's `retryOn`.

**Error edge** — An edge traversed only when its source reaches `FAILED` **after
retries are exhausted**. A node with an outgoing error edge has its failure
*handled*, so the failure does not propagate.

**Execution instance** — One run of a harness: an identity, a state for every
node, and a set of resolved values.

**Expression** — A statically analysable, side-effect-free expression in `${ }`.
No user-defined functions, no recursion, no environment access.

**Extension** — A namespaced vendor addition. `required="false"` may be ignored;
`required="true"` MUST cause rejection by a runtime that does not understand it.

**Guard** — A condition on whether a **single node** runs. False → the node is
`SKIPPED`, and its control successors still run. Contrast **decision**.

**Harness** — The whole workflow: one document, one unit of versioning,
validation and execution.

**HarnessXML** — This open specification, and the `.hxml` documents conforming to
it. Not a subset, profile or extension of any vendor format.

**`.hxml`** — The file extension for a HarnessXML document. Media type
`application/harnessxml+xml`. Advisory: implementations identify a document by
root element and namespace, never by filename.

**HXEP** — HarnessXML Enhancement Proposal. The public process by which normative
change happens, with a 30-day minimum review. Rejected proposals stay published.

**Idempotent** — Declared by the author: whether a node may execute more than once
with the same net effect. `idempotent="false"` means a runtime MUST NOT retry
automatically, and MUST NOT cancel the node while `RUNNING`.

**Impl** — An **opaque** handle a runtime resolves to something executable. The
specification assigns it no structure and never interprets it — this is the
boundary of scope.

**Join policy** — How a node's incoming edges combine into readiness: `all`
(default), `any` or `quorum`.

**Jitter** — Randomisation applied to a backoff delay. Defaults to on, because
synchronised retries turn a dependency blip into an outage.

**Loop** — A node type that repeats a body node. `maxIterations` is **required** —
there is no unbounded form.

**Node** — A unit of work with an identity, a type, and a declared interface of
ports.

**Port** — A named, optionally typed input or output of a node, carrying a value
**within one execution**. Matched by name, never by position. Contrast
**artifact**.

**Principal** — The identity a node executes as. Opaque to the specification. A
runtime that cannot resolve one MUST fail rather than fall back to a default.

**Provenance** — A record of where a document came from: the generating tool, the
source design and its digest, and optionally a signature.

**Resolved-negative** — The state of an edge whose `condition` evaluated false. It
never becomes satisfied; it neither blocks a join nor counts toward one.

**Resource** — An external capability the workflow needs but does not contain: a
model, datastore, queue, device, service or secret store. Declared once,
referenced by nodes, so that changing environment is a change to one block.

**Retry policy** — `<retry>`: how many attempts, with what backoff, on which error
classes. Absent means one attempt. Invalid on a non-idempotent node (`HX-3301`).

**RuMima** — VisML's commercial visual designer for HarnessXML. One
implementation, not the definition. If it and the specification disagree, the
specification is right.

**Skipped** — A **terminal successful** state, reached when a guard evaluates
false or every incoming edge resolved negative. Control successors still run;
data outputs are unavailable.

**Specification version** — `specVersion`, the version of HarnessXML a document is
written against. Distinct from `documentVersion`, the author's own revision.

**Subworkflow** — A node type invoking another harness by URI. Independently
validatable; recursion is invalid (`HX-3002`).

**Terminal state** — `SUCCEEDED`, `SKIPPED`, `FAILED`, `CANCELLED` or
`COMPENSATED`. A node in a terminal state never transitions again, except
`SUCCEEDED` → `COMPENSATED`.

**Transform** — A node type declared to be a **pure** function of its inputs.
Always freely retryable, and its output may be cached.

**Unwinding** — What a runtime does when a failure propagates: cancel in-flight
work, then compensate succeeded nodes in reverse completion order.

**`.visml`** — VisML's own vendor format, shared across its products. May
**embed** a complete HarnessXML document. No conforming implementation is ever
expected to read it.

**VisML** — Creator and steward of the HarnessXML specification. Holds final
editorial decision and the trademarks; bound by the published
[governance](/governance/).
