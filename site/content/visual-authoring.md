---
title: Visual Authoring
description: How a HarnessXML document relates to the visual graph it came from — the lossless round trip, what counts as presentation rather than semantics, and how to build an editor.
section: implementing
order: 5
status: draft
---

# Visual Authoring

HarnessXML exists because [the visual graph is the authoritative
design](/philosophy/#1-the-visual-graph-is-the-authoritative-design). People
reason about workflows as pictures; the format's job is to make that picture
executable rather than decorative.

This page is about the relationship between the two: what a designer must
preserve, what it must *not* put in the document, and what you need to get right
if you are building an editor of your own.

## The round trip is a hard requirement

> **A document generated from a graph MUST reopen as that graph, without loss of
> anything the specification defines.**

This is the load-bearing requirement of the whole design, and it is easy to
underestimate.

The moment the round trip is lossy, someone edits the generated file by hand to
fix what the editor mangled. From then on the file is authoritative and the
graph is a stale picture — which is exactly the drift HarnessXML was built to
end. A tool that exports a workflow and then asks you to maintain the export has
already conceded the argument.

Concretely, a conforming editor must round-trip:

- every node, with its type, `impl`, `idempotent`, `joinPolicy` and `quorum`
- every edge, **with its type** — `control`, `data`, `dependency`, `error` and
  `compensation` are five different relationships, not five arrow styles
- ports, including `type`, `required`, `default` and `value`
- guards, cases **in document order**, loop bounds, retry policies, timeouts
- resources, artifacts, security blocks, metadata and provenance
- vendor `<extension>` elements it does not itself understand

That last one catches people out. An editor that silently drops another vendor's
extension on save has corrupted a document it was only asked to display.

## Presentation is not semantics

The document deliberately contains **no layout**. Coordinates, colours, sizes,
grouping, edge routing, collapsed/expanded state, zoom — none of it appears in a
`.hxml` file, and a runtime MUST NOT depend on any of it.

This is not an oversight. Two reasons:

1. **A workflow's meaning cannot depend on where a box sits.** If it did, two
   documents that execute identically would diff differently, and every cosmetic
   nudge would show up in code review as a change to a production workflow.
2. **Layout is per-tool.** Your canvas conventions are not another editor's, and
   forcing one tool's geometry into the interchange format would make every other
   implementation carry it.

So where does layout go? Two options, and the specification is explicit about
both:

**A vendor `<extension>`, marked non-required:**

```xml
<node id="classify" type="inference">
  <extension namespace="https://acme.example/editor/1" required="false">
    <acme:layout xmlns:acme="https://acme.example/editor/1" x="420" y="180"/>
  </extension>
</node>
```

`required="false"` is essential here. A runtime that does not understand your
layout extension MUST ignore it and MUST still execute the node — which is
exactly right, because canvas coordinates have no bearing on execution.

**Or a host document that embeds the harness**, which is what VisML does — see
below.

## Embedding: a host document that carries both

A designer usually needs to store more than the specification defines: layout,
editor state, work in progress that does not yet execute, comments mid-argument.
The clean way is a **host document that embeds a complete HarnessXML document**
as a child element.

Rumima nests three layers to do this:

```
.rmmx                          the Rumima document — the file on disk
  └── .visml markup            the markup standard inside it
        └── <harness>          a complete HarnessXML document, embedded
```

```xml
<visml xmlns="https://visml.com/schema/1.0" product="rumima">
  <rumima-document version="1">
    <canvas><!-- layout, colours, grouping: presentation, not semantics --></canvas>

    <harness xmlns="https://harnessxml.com/spec/1.0"
             id="document_triage" specVersion="1.0">
      <!-- a complete, independently valid HarnessXML document -->
    </harness>
  </rumima-document>
</visml>
```

The full path from design to execution:

```
Rumima Enterprise Studio  →  .rmmx  →  .hxml  →  any conforming runtime  →  execution
```

**Only `.hxml` crosses the boundary between tools.** The runtime is handed the
exported document and never sees `.rmmx` or `.visml` — which is exactly what
makes the runtime replaceable.

Export is then **lifting the element out**; import is wrapping it. Nothing is
translated, so nothing can be lost in translation — which is the cheapest
possible way to guarantee the round trip.

**The embedded element must be a complete document**, not a fragment or a
dialect. Serialised on its own it validates against
[`harnessxml-1.0.xsd`](/schema/v1.0/harnessxml-1.0.xsd).

> **The dependency runs one way only.** HarnessXML must be fully definable,
> validatable and executable without reference to `.visml`, `.rmmx` or any other
> host format. It is **not a subset of, profile of, or extension of** any of
> them — it is an independent specification that a host document happens to
> contain, exactly as an HTML page may contain an SVG document without SVG
> becoming a subset of HTML. See
> [§2.9.1](/spec/v1.0/document-structure/#2-9-1-hxml-visml-and-rmmx-embedding-not-subsetting).

Nothing about this is specific to VisML. Any editor, document format or
repository may embed a harness by the same rule.

## The dynamic layer, and the line it must not cross

A designer wants more than static structure. Rumima's `.visml` markup is
**dynamic**: it does finetuning, injects additional weights and attributes, and
parameterises a workflow, for maximum flexibility in authoring.

That is a genuinely useful capability, and it sits in tension with the thing
that makes HarnessXML worth having. So the boundary matters more here than
anywhere else on this page.

### Why the tension is real

HarnessXML's value proposition is that a workflow can be **validated before it
runs** — every reference resolved, every loop bounded, every non-idempotent step
identified, before anything touches production. That guarantee holds only if
**the document you reviewed is the workflow that ran**.

A vendor layer that rewrote core constructs at execution time would destroy it.
The retry policy in the diff would not be the retry policy that applied; the
approval threshold reviewed on Tuesday could be something else on Wednesday; a
validator's verdict would mean nothing.

### Where flexibility belongs

Two places, and the specification supports both:

**1. Resolve at export.** Templating, finetuning and injection happen while
producing the `.hxml`. The exported document is *concrete* — every value
already substituted — and is what gets validated, reviewed, signed and
executed. This is where most flexibility should live, and it costs the
specification nothing.

**2. Declare the variable parts.** Where a value genuinely must vary per run,
HarnessXML already has the mechanisms, and they are visible to a reader:

| need | construct |
|---|---|
| a per-environment endpoint or credential | `<resource>` + `<credential ref>` — change the resource block, nothing else |
| a tunable threshold | `<config>` property, which shows up in a diff |
| a value computed per execution | port `value` with a `${…}` expression |
| vendor tuning a runtime may ignore | `<extension required="false">` |
| vendor tuning a runtime must honour or refuse | `<extension required="true">` |

That last pair is the important one. An extension marked `required="false"`
carries weights, hints and tuning that a runtime may use or ignore, and the node
still executes either way. Marked `required="true"`, a runtime that does not
understand it **must refuse to run the document** rather than execute it
differently — which is how a vendor says "without this, the workflow is wrong"
without ever producing a silent behavioural difference.

### `.hxml` inside the designer, and finetuning during execution

Rumima also keeps `.hxml` **inside** the tool, and finetunes focused harness
attributes while a workflow is running. That is the sharpest form of the tension
above, so it needs the clearest rule.

The reconciliation is that HarnessXML already has the machinery — it just has to
be used honestly:

**Every adjustment produces a new document revision.** Not an edit applied to a
running document, but a *new*, complete, valid `.hxml` with its own identity:

```xml
<metadata>
  <documentVersion>7</documentVersion>
  <provenance>
    <generator name="Rumima Enterprise Studio" version="1.0" vendor="VisML"/>
    <source uri="rmmx://triage.rmmx" type="visual-graph" digest="sha256:…"/>
    <signature algorithm="ed25519" keyId="…" value="…"/>
  </provenance>
</metadata>
```

With that, "which workflow actually ran?" still has an answer — a digest, a
version and a signature — and the execution trace resolves back to it
([chapter 12](/spec/v1.0/provenance/)). Finetuning becomes a *sequence of
identified revisions* rather than an untracked mutation, and everything the
specification promises survives.

Without it, the guarantee is gone. A document edited underneath a running
instance means the validated artifact and the executed artifact are different
things, the diff someone reviewed describes neither, and an audit two years
later has nothing to point at.

So:

| doing this | is | because |
|---|---|---|
| adjusting `<config>` or a port `value` between runs, re-exporting, re-validating | **fine** | the new revision is a real document with its own identity |
| a runtime being handed revision 8 while revision 7's instance finishes | **fine** | each instance names the revision it ran |
| tuning carried in `<extension required="false">` that a runtime may use or ignore | **fine** | declared, visible, and optional by construction |
| mutating a running instance's harness attributes in place, with no new identity | **not conformant** | nothing can say afterwards what executed |

The distinction is not bureaucratic. It is the difference between a workflow you
can attest to and one you can only describe.

### The rule

> Whatever a vendor layer does, **the exported `.hxml` must be complete, valid
> and executable on its own**. A runtime is handed that document and nothing
> else. It never reads `.rmmx`, never reads `.visml`, and never needs to.

Injection that resolves *into* the exported document is flexibility. Injection
that a runtime would have to reach back into a vendor format to resolve is a
dependency, and the
[one-way rule](/spec/v1.0/document-structure/#2-9-1-hxml-visml-and-rmmx-embedding-not-subsetting)
forbids it — not to limit Rumima, but so that a HarnessXML document keeps
meaning the same thing everywhere.

## Building your own editor

You need no permission. The specification is [CC BY 4.0](/licensing/), the
schema and reference code are Apache 2.0, and there is nothing to sign.

What to get right, in rough order of how often it is got wrong:

1. **Draw edge types distinguishably.** If a reader cannot tell a `data` edge
   from a `compensation` edge at a glance, the picture is not showing them the
   workflow. This is the single biggest visual design decision in a HarnessXML
   editor.
2. **Show `idempotent="false"` loudly.** It is the attribute that decides whether
   a step may be retried, and getting it wrong duplicates payments and re-grasps
   parts. It deserves to be visible on the node, not buried in a properties panel.
3. **Make `maxIterations` unavoidable.** It is required, and a good editor
   refuses to draw a loop without one rather than emitting an invalid document.
4. **Surface validation inline.** Every rule carries an
   [`HX-nnnn` code](/spec/v1.0/errors/); show them on the canvas where the
   problem is, not in a modal after export.
5. **Preserve unknown extensions on save**, as above.
6. **Never reorder `<case>` elements.** Document order is normative — the first
   true case wins — so a "tidy" that sorts them alphabetically silently rewrites
   the routing policy.

Check yourself against the [conformance suite](/conformance/) rather than against
any particular product, including VisML's.

## Rumima Enterprise Studio

**Rumima** is VisML's commercial visual designer for HarnessXML, and the
flagship authoring environment for the format. It saves documents as **`.rmmx`**,
whose contents use the **`.visml`** markup standard, and exports **`.hxml`** by
the embedding rule above.

It is **one implementation, not the definition**. If Rumima and this
specification ever disagree, the specification is right and Rumima has a bug —
that is a
[published governance commitment](/governance/#1-the-bargain), alongside the
fact that Rumima uses the same public extension mechanism as any third party and
gets no privileged extension point.

You do not need it to use HarnessXML. A document is a text file conforming to a
published schema: write it by hand, generate it, or build your own editor.

Rumima is available from **[visml.com](https://visml.com)**.
