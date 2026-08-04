---
title: Roadmap
description: Where HarnessXML is heading. A non-binding statement of direction — only released versions are commitments.
section: project
order: 40
status: stable
---

# Roadmap

> **This roadmap is not a commitment.** Only released specification versions are
> commitments. Everything below is a statement of current intent, revised in the
> open as it changes. Nothing here should be implemented against.

## Where things actually stand

Stated plainly, because a roadmap that overstates its present is worthless.

| piece | state |
|---|---|
| Object model and language design | drafted, coherent, not yet reviewed by anyone outside VisML |
| XSD 1.0 schema | written, compiles, enforces referential integrity |
| Reference examples (AI, robotics, networking, enterprise, training) | written, schema-valid, used as fixtures |
| Governance, licensing, contribution policy | published |
| Specification chapters | **partly written** — each page states its own status |
| Reference runtime (Rust) | parser, validator **and executor** working — 43 tests |
| Conformance suite | fixture format defined; corpus incomplete |
| SDKs beyond Rust | not started |

Any page marked **draft** is not normative. Any page marked **planned** has not
been written at all — it is an outline of intent, listed so the shape is public.

## Toward v1.0 final

The gate for calling v1.0 released, and freezing it forever:

1. **Every normative rule has a conformance fixture.** A rule the suite cannot
   check is a rule implementers will diverge on. No exceptions — this is the
   condition most likely to delay the release, and it should be.
2. ~~The reference runtime executes the full model, not just validation.~~ **Done** — lifecycle, join policies, decisions, loops, retries, compensation and traces are implemented and tested.
3. **At least one independent implementation** exists and passes Core level. A
   specification validated only by its author is a file format.
4. **The specification survives an outside read.** Ambiguity reports from people
   who did not write it are the most valuable input this project can get, and the
   ones that reliably find the sentences two engineers read differently.
5. **A public issue tracker with real traffic.** Governance that has never been
   exercised has not been tested.

There is no date. A specification that freezes its released text forever should
not be rushed into freezing the wrong text.

## Candidate directions after 1.0

Ideas, not plans. Each would go through the [HXEP process](/governance/#3-changing-the-specification-the-hxep-process),
in public, with the 30-day review.

**Streaming and long-running semantics.** Nodes that emit incrementally rather
than returning once — the shape agent systems increasingly have. Needs care: it
touches the lifecycle state machine, which is the part most expensive to change.

**Richer type system for ports.** Today a port type is an open-ended string and
compatibility across a data edge is checked loosely. A stronger optional type
layer would catch more at validation time. The tension is obvious — stronger
typing means more documents rejected for reasons the author considers pedantic.

**A JSON projection.** A normative, lossless JSON encoding of the same object
model, for HTTP APIs and browser tooling. The rule would be that the XML remains
canonical for signing and archival.

**Distributed execution semantics.** What a conforming runtime must guarantee
when a node runs on a machine that then disappears. Currently out of scope, and
arguably belongs there — but it is the question implementers ask most.

**Formal semantics.** A machine-checked model of the execution semantics. The
honest reason to want this is that it finds ambiguities prose review does not.

**Importers.** BPMN, CI/CD workflow YAML, and popular agent-framework graphs.
Migration paths matter more to adoption than features do.

## What will not happen

Useful to say, because these get proposed repeatedly and rejecting them once in
public is cheaper than rejecting them annually.

- **A standard library of step implementations.** The specification stops at the
  boundary of the work. Standardising the steps is what dated every prior attempt.
- **A general-purpose expression language.** Expressions stay statically
  analysable. If you need real logic, that is what a `transform` node is for.
- **Unbounded loops.** `maxIterations` stays required.
- **Silent tolerance of unknown constructs.** A runtime that meets something it
  does not understand will keep failing loudly.
