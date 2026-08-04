---
title: FAQ
description: Direct answers to the questions people actually ask about HarnessXML — including the awkward ones about vendor neutrality and XML.
section: project
order: 50
status: stable
---

# FAQ

## Is this actually open, or is it open-ish?

Open. The specification text is CC BY 4.0, the code is Apache 2.0, and there is
no implementer agreement to sign, no royalty, and no notification requirement.
You may build a competing product on it and VisML has no say.

The two limits are honest ones: **trademarks** are reserved, so you cannot brand
your implementation as the official one; and VisML holds final editorial decision
on the specification. Both are stated in the [governance model](/governance/),
along with the fact that the CC BY and Apache grants already made are
irrevocable — so the specification is forkable regardless of what VisML does
later.

## VisML sells Rumima and stewards the specification. Isn't that a conflict?

Yes. It is named explicitly in
[governance §1](/governance/#1-the-bargain) rather than glossed over.

The structural answers: Rumima uses the same public extension mechanism as any
third party, with no privileged extension point. The reference runtime is a
separate Apache-2.0 implementation, not Rumima. And conformance is defined by a
published test suite — so "compatible" means "passes the suite", never "agrees
with Rumima".

If the specification and Rumima disagree, the specification is right and Rumima
has a bug.

## Do I need Rumima to use HarnessXML?

No. A document is a text file conforming to a published schema. Write it by hand,
generate it, or build your own editor. Rumima is one authoring environment.

## Why XML in 2026?

Because the requirements point there: normative schema validation including
cross-reference integrity, namespaced vendor extension, mature signing and
canonicalisation, comments, and universal tooling in every language. Documents
are mostly generated and read in diffs, which is exactly the case where verbosity
costs least and strictness pays most.

The longer answer is in [Why HarnessXML](/why-harnessxml/#why-xml), including
where JSON is genuinely the better fit — and the plan for a normative JSON
projection rather than pretending the question does not arise.

## How is this different from Airflow / Temporal / Prefect?

Those are runtimes, and good ones. In each, the workflow *is* the code, so it is
portable only to that runtime and readable only by people who read that language.

HarnessXML is a specification, not a runtime. Its output is a portable document
that several independent runtimes can execute identically. You could reasonably
implement a HarnessXML runtime *on top of* Temporal — that is a compiler, and
it's a listed component anyone may build.

## How is this different from BPMN?

BPMN is a genuine open standard with real vendor diversity, and it earned that.
It was designed for human business processes, so it has no first-class notion of
model resources, digest-identified artifacts, declared idempotence, or bounded
loops for unattended physical systems. Extending it into ML and robotics means
fighting its assumptions.

An importer from BPMN is on the [roadmap](/roadmap/). Migration paths matter more
to adoption than features do.

## What's the file extension? And what is `.visml`?

HarnessXML documents use **`.hxml`**, media type `application/harnessxml+xml`.
Neither affects validity — an implementation must recognise a document from its
root element and namespace, never from its filename.

`.visml` is VisML's own format, shared across its products including Rumima. It
carries canvas layout, colours, grouping and editor state that HarnessXML
deliberately excludes, and it **embeds a complete HarnessXML document** as a
child element. Export lifts that element out; import wraps it.

## Isn't HarnessXML then just a subset of VisML's format?

No, and the distinction is load-bearing rather than pedantic.

**The dependency runs one way only.** HarnessXML must be fully definable,
validatable and executable without reference to `.visml` or any other host
format, and a conforming implementation must never be required to understand a
host format to process an embedded HarnessXML document.

So HarnessXML is **not a subset of, profile of, or extension of** the VisML
markup language. It is an independent specification that a VisML document happens
to contain — exactly as an HTML page may contain an SVG document without SVG
becoming a subset of HTML. Any host may embed it by the same rule.

If that direction were reversed, implementing HarnessXML would require
understanding a vendor's proprietary format first, the normative namespace would
belong to that vendor, and "open and vendor-neutral" would be a claim the
format's own definition contradicted. See
[§2.9.1](/spec/v1.0/document-structure/#2-9-1-hxml-and-visml-embedding-not-subsetting).

## Can I extend it?

Yes, through a namespaced `<extension>` element you control. Set
`required="false"` and runtimes that do not understand it ignore it and still
execute the node; set `required="true"` and a runtime that does not understand it
must refuse to run the document rather than quietly behave differently.

The steward's namespace is reserved. Nobody, VisML included, gets a private
extension point. See [governance §6](/governance/#6-extension-mechanism).

## What happens when a runtime meets a node type it doesn't know?

It **must fail** with `HX-1003`. It must not skip the node.

This is the single most important rule in the specification. Silently ignoring an
unrecognised construct means a run reports success while having done less than it
was told — and the omitted step could have been the approval gate, the safety
check or the rollback.

## Will v1.0 change after release?

No. Released text is frozen at a permanent URL forever, including typos.
Corrections are published as dated errata, appended rather than inlined, so a
document written against v1.0 in 2026 still validates against the v1.0 served in
2036.

Additive changes land in v1.1. Breaking changes require v2.0, a migration guide
and a mechanical migration tool — or the major version does not ship.

## Is v1.0 finished?

**No.** The language design and schema are drafted and coherent; several
specification chapters are still marked draft or planned, the reference runtime's
executor is not written, and the conformance corpus is incomplete. Each page
states its own status, and the [roadmap](/roadmap/) states the release gate.

An open specification that overstates its readiness burns the credibility it
needs, so the status labels are deliberately unflattering.

## Who decides what goes in?

VisML holds final editorial decision, exercised through the
[HXEP process](/governance/#3-changing-the-specification-the-hxep-process):
public proposal, 30-day minimum review, written reasons for rejection, and
rejected proposals stay published permanently. Nothing becomes normative without
conformance tests.

Implementers have a standing right to be heard on any change that would break
them.

## What if VisML disappears?

Stewardship — trademarks included — transfers to a neutral body rather than
lapsing or being acquired as a proprietary asset. That is a promise.

The part that is not a promise: the CC BY 4.0 and Apache 2.0 grants already made
are irrevocable. The specification and reference implementation remain forkable
by anyone, permanently, whatever VisML does. See
[governance §9](/governance/#9-succession).

## How do I report an ambiguity?

Open an issue, and say what you thought the sentence meant and what the other
reading is. If two competent engineers can read a normative sentence and build
incompatible implementations, that sentence is a bug — and this is the most
valuable report the project can receive. See [contributing](/contributing/).
