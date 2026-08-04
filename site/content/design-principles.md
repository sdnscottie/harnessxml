---
title: Design Principles
description: The rules used to settle design arguments in HarnessXML — the ones that decide what gets into the specification and what stays out.
section: introduction
order: 5
status: stable
---

# Design Principles

These are decision rules, not slogans. Each one has been used to reject something
that would otherwise have looked reasonable, and each names the failure it exists
to prevent.

## 1. Fail loudly on the unknown

A runtime that meets a construct it does not recognise **must refuse to run the
document** (`HX-1003`), never skip it.

*Prevents:* a workflow reporting success while having silently omitted a step.
An unrecognised node could have been the approval gate, the safety check or the
rollback. Every other tolerance-vs-strictness tradeoff in this specification is
decided in favour of strictness for this reason.

*Cost, accepted:* forward compatibility is harder. A 1.0 runtime cannot run a
1.2 document that uses a 1.2 node type. That is the correct outcome — it should
say so rather than guess.

## 2. Dangerous defaults are not defaults

Anything whose wrong setting is unsafe is **required**, not defaulted.

- `maxIterations` on every loop — required. There is no "unbounded" value.
- `maxAttempts` on a retry policy — required.
- `specVersion` on the document — required.

*Prevents:* the class of incident where nobody chose the behaviour, so nobody
reviewed it. A default of "infinite" is a decision made by whoever wrote the
library, applied to a robot arm at 3am.

## 3. Idempotence is declared, never inferred

A node states whether it may be retried. A runtime **must not** automatically
retry `idempotent="false"`, and a document combining that with a retry policy is
invalid (`HX-3301`).

*Prevents:* a duplicated payment, a double ledger entry, a second grasp on a part
already held. A runtime cannot deduce this — only the author knows. So the format
makes the author say it, and makes the unsafe combination unrepresentable rather
than merely discouraged.

## 4. Relationship type carries meaning

Edges are typed — `control`, `data`, `dependency`, `error`, `compensation` — and
the type determines what the scheduler does. They are not labels for a diagram.

*Prevents:* the ambiguity where an arrow might mean "then", "feeds", "if this
breaks" or "to undo this". Once one arrow can mean four things, the picture stops
being executable, and every implementation resolves the ambiguity differently.

## 5. Determinism where a human will be asked to explain it

Decision cases evaluate **in document order, first true wins**. Not "highest
priority", not "most specific", not unordered.

*Prevents:* two engineers reading the same routing table and disagreeing about
which branch fires. When an approval threshold has to be explained to an auditor,
"the first matching rule, reading top to bottom" is an answer. "It depends on
evaluation order" is not.

## 6. The specification stops at the boundary of the work

A node carries an opaque `impl` handle. What runs — a function, a container, a
NETCONF call, a trajectory — is out of scope, permanently.

*Prevents:* obsolescence. Every specification that tried to also standardise the
work itself became dated as soon as the technology under it moved, and this field
moves faster than most. The workflow shape is durable; the step implementations
are not.

## 7. Secrets are referenced, never contained

`<credential ref="..."/>` names a secret in a store. A literal credential in a
document is invalid (`HX-3501`).

*Prevents:* the most predictable leak in the industry. These documents are
designed to be committed to git, diffed in pull requests and archived for audit —
three excellent ways to publish a key. The format refuses to make it convenient.

## 8. Everything normative is testable

A rule with no conformance test does not go into the specification. Not "should
have a test" — does not ship.

*Prevents:* prose that sounds normative but that two implementers read
differently, with no mechanism to discover the divergence. If the rule cannot be
expressed as "this document must be rejected with this code", the rule is not
precise enough yet.

## 9. Extension without forking

Vendors extend through a namespaced `<extension>` element, declaring whether it
is `required`. The steward's namespace is reserved; nobody, including VisML, gets
a privileged private extension point.

*Prevents:* the fork. A vendor blocked from shipping a capability will ship it
anyway — the only question is whether it appears in a clearly-marked vendor
namespace or as a silent incompatibility in the core. `required="true"` also lets
a vendor say "without this, the workflow is wrong", instead of a runtime
producing subtly different behaviour without saying so.

## 10. Released text never changes

Once a version is released it is frozen at a permanent URL. Corrections are
published as dated errata, appended — never edits to the original.

*Prevents:* the situation where a document validated against v1.0 in 2026 and
fails against "v1.0" in 2030. A specification that can be edited after release is
not something you can safely cite in a contract, a certification or an audit.

## 11. Small expression language, on purpose

Expressions read outputs, compare values and combine them. No user-defined
functions, no recursion, no side effects.

*Prevents:* a workflow that cannot be statically analysed. The whole value
proposition is validating a workflow *before* it runs; every expressiveness win
that costs analysability is a bad trade here. If real logic is needed, that is
what a `transform` node is for — it is the honest place to put code.

## 12. Optimise for reading, not for writing

Documents are mostly generated from a visual editor and read in a diff. So the
format optimises for the reader: explicit over implicit, named over positional,
verbose over clever.

*Prevents:* the terse-format trap, where the syntax is pleasant to hand-write and
unreadable in a three-line change six months later. Nobody's productivity depends
on this format being brief. Several people's decisions depend on it being clear.
