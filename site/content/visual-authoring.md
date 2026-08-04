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
as a child element:

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

Export is then **lifting the element out**; import is wrapping it. Nothing is
translated, so nothing can be lost in translation — which is the cheapest
possible way to guarantee the round trip.

**The embedded element must be a complete document**, not a fragment or a
dialect. Serialised on its own it validates against
[`harnessxml-1.0.xsd`](/schema/v1.0/harnessxml-1.0.xsd).

> **The dependency runs one way only.** HarnessXML must be fully definable,
> validatable and executable without reference to any host format. It is **not a
> subset of, profile of, or extension of** `.visml` or anything else — it is an
> independent specification that a host document happens to contain, exactly as
> an HTML page may contain an SVG document without SVG becoming a subset of
> HTML. See [§2.9.1](/spec/v1.0/document-structure/#2-9-1-hxml-and-visml-embedding-not-subsetting).

Nothing about this is specific to VisML. Any editor, document format or
repository may embed a harness by the same rule.

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
flagship authoring environment for the format. It authors in `.visml` and exports
`.hxml` by the embedding rule above.

It is **one implementation, not the definition**. If Rumima and this
specification ever disagree, the specification is right and Rumima has a bug —
that is a
[published governance commitment](/governance/#1-the-bargain), alongside the
fact that Rumima uses the same public extension mechanism as any third party and
gets no privileged extension point.

You do not need it to use HarnessXML. A document is a text file conforming to a
published schema: write it by hand, generate it, or build your own editor.

Rumima is available from **[visml.com](https://visml.com)**.
