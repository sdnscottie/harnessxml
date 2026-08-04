---
title: Philosophy
description: The three-part philosophy behind HarnessXML — the visual graph is the authoritative design, HarnessXML is the portable representation, and the runtime executes the specification consistently across platforms.
section: introduction
order: 3
status: stable
---

# Philosophy

Three statements, in order. Each one only makes sense because of the one before it.

## 1. The visual graph is the authoritative design

People do not reason about workflows as text. They reason about them as
pictures — boxes, arrows, a branch, a loop back. Ask an engineer to explain an
orchestration and they reach for a whiteboard, every time.

Most tooling treats that as a weakness to be corrected: draw the picture to
communicate, then write the *real* thing in code. HarnessXML takes the opposite
position. **The graph is not a simplification of the design. It is the design.**
The code is the lossy artifact — it scatters one coherent structure across
decorators, config files and exception handlers until no single place shows the
shape any more.

So the graph is authoritative. Editing it is editing the system. A tool that
lets you draw a workflow and then generates a starting point you are expected to
maintain by hand has already conceded the argument.

This is also why *round-tripping* is a hard requirement rather than a feature. A
document generated from a graph must be re-openable as that graph, without loss.
The moment the round trip is lossy, the code becomes authoritative again and
everything above collapses.

## 2. HarnessXML is the portable machine-readable representation

A graph in one vendor's editor is an asset with an expiry date. It needs a
serialisation that is:

- **complete** — enough to execute, not a sketch that needs code to fill in;
- **portable** — readable by tools that vendor never heard of;
- **diffable** — reviewable in a pull request, line by line;
- **stable** — a document written today opens in ten years.

XML is used because those requirements point at it: a mature schema language for
structural validation, namespaces for vendor extension without forking, mature
signing and canonicalisation for provenance, and universal tooling in every
language an SDK might target.

But **HarnessXML is not an XML project**, and the site does not market XML. The
serialisation is an implementation detail of the specification. What is being
specified is a *language* — an object model, a set of typed relationships and an
execution model. The angle brackets are how it is written down.

## 3. The runtime executes the specification consistently across platforms

The third statement is what turns the first two from a nice idea into
infrastructure. If two conforming runtimes execute the same document differently,
then the document does not mean anything and the graph was never authoritative —
it was authoritative *for one engine*.

So the specification defines behaviour precisely, and mechanically:

- a **lifecycle state machine** every node passes through, with named states;
- **typed edges** whose type determines what the scheduler does — control, data,
  dependency, error and compensation are not annotations, they are semantics;
- **deterministic decisions** — cases evaluate in document order, first true wins;
- **bounded loops** — `maxIterations` is required, because an unbounded loop in
  an unattended workflow is a defect;
- **explicit idempotence** — a node that must not be retried says so, and a
  conforming runtime must honour it;
- **loud failure on the unknown** — a runtime meeting a construct it does not
  recognise must fail with an error, never skip it.

That last rule deserves its own paragraph, because it is the one most formats get
wrong. Silently ignoring an unrecognised element feels tolerant and generous. In
a workflow format it is catastrophic: the run *reports success* while having done
less than it was told. A missing approval step, a skipped safety check, an
unexecuted rollback — all reported green. HarnessXML would rather refuse to run.

## What follows from all three

If the graph is authoritative, and the document is a lossless portable form of
it, and every conforming runtime executes it identically, then a workflow becomes
something an organisation can actually own: reviewed like code, versioned like
code, audited like a record, and portable between vendors like a standard.

That is the whole argument. Everything in the [specification](/spec/v1.0/) is
downstream of it — and the [design principles](/design-principles/) are the
rules used to settle the cases where these three statements pull against each
other.
