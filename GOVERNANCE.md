# HarnessXML Governance

**Status:** active · **Applies from:** v1.0 · **Steward:** VisML

HarnessXML is an open, vendor-neutral specification for executable intelligent
system workflows. This document states who decides what, how a change is
proposed, and what guarantees an implementer can rely on. It is deliberately
published *before* v1.0 is final, because a specification that arrives without
governance asks implementers to trust a promise rather than a process.

---

## 1. The bargain

VisML created HarnessXML and stewards it. In exchange for that recognition,
VisML accepts binding constraints on how it may change the specification.

**What VisML commits to:**

| commitment | mechanism |
|---|---|
| The specification is permissively licensed | CC BY 4.0 text, Apache-2.0 code — see `LICENSE-SPEC`, `LICENSE-CODE` |
| Anyone may implement it, commercially, without permission or royalty | no implementer agreement exists to sign |
| Released versions never change | immutability rule, §4 |
| Breaking changes only at a major version, with rationale | compatibility policy, §5 |
| Changes are proposed in public, with a written record | HXEP process, §3 |
| A competing implementation can prove conformance | conformance suite, §7 |

**What VisML retains:** final editorial decision on the specification, and the
`HarnessXML`, `VisML` and `Rumima` trademarks. Trademarks are the one thing not
open-licensed — they are what prevents a fork from calling itself official
while diverging. Anyone may *implement* HarnessXML; nobody may claim to *be* it.

**The conflict of interest, stated plainly.** VisML also sells Rumima
Enterprise Studio, the flagship commercial authoring environment for
HarnessXML. That is a real conflict and pretending otherwise would be worse
than naming it. The structural answer:

- **No privileged extension point.** Rumima uses the same public extension
  mechanism (§6) as any third party. If Rumima needs a capability, it is
  proposed as an HXEP in public, or it lives in a vendor namespace where
  everyone can see it is not part of the core specification.
- **The reference runtime is not Rumima.** `reference-runtime/` is an
  independent Apache-2.0 implementation. If the specification and Rumima
  disagree, the specification is right and Rumima has a bug.
- **Conformance is defined by the test suite, not by matching Rumima.**

---

## 2. Roles

**Steward (VisML).** Maintains the specification, runs the release process,
holds final editorial decision, operates the conformance suite.

**Editors.** Named per specification version, listed in that version's front
matter. Editors write normative text and are accountable for its precision.

**Implementers.** Anyone shipping a parser, validator, runtime, compiler,
importer, exporter, editor or SDK. Implementers have a standing right to be
heard on any HXEP that would break them — an HXEP cannot be accepted while an
implementer objection is unanswered, though it may be accepted over an
objection that has been answered.

**Contributors.** Anyone filing an issue, a proposal, a test case or a
correction. No agreement to sign; see `CONTRIBUTING.md` for the terms under
which contributions are accepted.

---

## 3. Changing the specification — the HXEP process

Every normative change goes through a **HarnessXML Enhancement Proposal**.
Editorial fixes (typos, clarifications that change no behaviour) do not.

```
  DRAFT ──► REVIEW ──► ACCEPTED ──► IMPLEMENTED ──► ACTIVE
    │          │            │
    └──► WITHDRAWN          └──► REJECTED  (with written reasons, kept published)
```

| stage | what it means | who moves it |
|---|---|---|
| **Draft** | written up, has a number, not yet argued | author |
| **Review** | open for public comment, minimum **30 days** | author requests |
| **Accepted** | editorially approved, normative text agreed | steward |
| **Implemented** | present in the reference runtime **and** covered by conformance tests | steward |
| **Active** | shipped in a released specification version | release |
| **Rejected** | declined — the reasons stay published permanently | steward |

**An HXEP must contain:** the problem, why the existing specification cannot
express it, the proposed normative text, at least one worked example, the
compatibility impact (§5), and the conformance tests that would prove it.

**Rejected proposals are never deleted.** The record of what was considered and
declined is as much a part of a specification's value as what was accepted —
it stops the same argument being had every year.

**Nothing becomes Active without tests.** A feature that the conformance suite
cannot check is a feature implementers will diverge on.

---

## 4. Versioning and immutability

Specification versions are `MAJOR.MINOR`.

- **MINOR** (`1.0 → 1.1`) — additive only. Every valid 1.0 document is a valid
  1.1 document, and a 1.0 runtime executing a 1.1 document behaves correctly
  for every construct it recognises.
- **MAJOR** (`1.x → 2.0`) — may remove or redefine. Requires a migration guide
  and a mechanical migration tool in the reference implementation, or the major
  version does not ship.

**Immutability.** Once a version is released it is frozen at a permanent URL:

```
https://harnessxml.com/spec/v1.0/          the specification, forever
https://harnessxml.com/schema/v1.0/harnessxml-1.0.xsd
```

Released text is never edited in place — not for typos. Corrections are
published as a dated **erratum** appended to that version, so a document
written against v1.0 in 2026 still validates against the v1.0 that is served in
2036. This is what makes the specification safe to cite in a contract.

**Namespaces are versioned by major version only:**

```
xmlns="https://harnessxml.com/spec/1.0"      used by 1.0, 1.1, 1.2 …
xmlns="https://harnessxml.com/spec/2.0"      used by 2.0, 2.1 …
```

A minor version does not change the namespace, because a namespace change
breaks every existing document — which is exactly what a minor version
promises not to do.

---

## 5. Compatibility guarantees

**Within a major version, VisML will never:**

- remove an element, attribute or enumeration value
- narrow the value space of an existing attribute
- change the runtime meaning of an existing construct
- make an optional attribute required
- change a default value

**Within a major version, VisML may:**

- add optional elements and attributes
- add enumeration values *to attributes documented as open-ended*
- add new node types, edge types and error codes
- deprecate a construct — which marks it, warns on it, and **keeps it working**

**Deprecation runs for one full major version minimum.** A construct deprecated
in 1.3 keeps working through every 1.x and may only be removed in 2.0.

**Forward compatibility is a runtime obligation.** A conforming runtime meeting
a construct it does not recognise must fail loudly with `HX-1003` rather than
skip it. Silently ignoring an unrecognised node is the failure mode that
destroys trust in a workflow format: the workflow appears to succeed while
doing less than it was asked to. See the specification, *Error Reporting*.

---

## 6. Extension mechanism

Extension is designed in, so that vendors do not fork the core to ship a
feature.

```xml
<node id="grasp" type="task">
  <extension namespace="https://acme.example/harness/1" required="false">
    <acme:forceLimit xmlns:acme="https://acme.example/harness/1">12.5</acme:forceLimit>
  </extension>
</node>
```

- `required="false"` — a runtime that does not understand the extension MUST
  ignore it and MUST still execute the node.
- `required="true"` — a runtime that does not understand it MUST refuse to
  execute the document (`HX-1004`). This is how a vendor states "without this,
  the workflow is wrong", instead of quietly producing different behaviour.

**Extensions must be namespaced to a URI the vendor controls.** The
`https://harnessxml.com/` namespace is reserved to the steward. An extension
that proves broadly useful is the normal route to an HXEP for core inclusion.

---

## 7. Conformance and certification

Conformance is defined by the published suite in `conformance/`, not by
agreement with any implementation.

**Three levels:**

| level | an implementation must |
|---|---|
| **Core** | parse, validate and reject per every structural and reference rule |
| **Executing** | additionally run the execution model — dependencies, scheduling, conditionals, loops, retries, lifecycle states |
| **Full** | additionally implement resources, artifacts, provenance and the security model |

Each level has a machine-runnable fixture set: valid documents that must be
accepted, invalid documents that must be rejected **with the specified error
code**, and execution traces that must match.

**Self-certification is the default.** Run the suite, publish the results,
state the level. VisML does not gatekeep. What VisML controls is the badge:
using the HarnessXML conformance mark requires published, reproducible results
for a tagged release. False conformance claims are a trademark matter — the
only enforcement lever the licences leave, and the reason trademarks are
withheld from the open grant.

---

## 8. Public record

| what | where |
|---|---|
| Issue tracking | public tracker, linked from harnessxml.com |
| HXEPs | `spec/hxep/`, all states including Rejected |
| Release notes | per version, with the HXEPs each contains |
| Errata | per version, dated, appended never inlined |
| Conformance results | published per implementation |
| Roadmap | non-binding statement of direction, revised openly |

The roadmap is explicitly **not** a commitment. Only released versions are
commitments.

---

## 9. Succession

A specification nobody can maintain but its author is not infrastructure.

If VisML can no longer steward HarnessXML, it commits to transferring
stewardship — the trademarks included — to a neutral body (a foundation or
standards organisation) rather than letting the specification lapse or
allowing it to be acquired as a proprietary asset. Absent such a transfer, the
CC BY 4.0 and Apache-2.0 grants already made are irrevocable: the specification
and reference implementation remain forkable by anyone, permanently.

That is the floor. Everything above it is a promise; this is the part that
holds even if the promises do not.
