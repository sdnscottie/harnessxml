---
title: Architecture Overview
description: How a visual graph becomes an executing workflow — object model, HarnessXML document, validator, runtime, monitoring — and which parts anyone may reimplement.
section: introduction
order: 6
status: stable
---

# Architecture Overview

## The pipeline

<div class="diagram">
<svg viewBox="0 0 900 380" xmlns="http://www.w3.org/2000/svg" role="img"
     aria-labelledby="pipetitle pipedesc">
  <title id="pipetitle">HarnessXML end-to-end architecture</title>
  <desc id="pipedesc">A visual graph is authored in a designer such as Rumima and
    held as an internal object model. It is serialised to a HarnessXML document,
    which is the open interchange point. A validator checks the document against
    the schema and the specification rules. A conforming runtime loads the
    validated document, resolves resources and artifacts, and executes it,
    emitting traces to monitoring. Authoring tools, validators, runtimes and SDKs
    may all be independently implemented against the specification.</desc>
  <defs>
    <marker id="a2" viewBox="0 0 10 10" refX="9" refY="5"
            markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M0 0 L10 5 L0 10 z" fill="currentColor"/>
    </marker>
  </defs>

  <g font-size="12.5" text-anchor="middle" fill="currentColor">

    <text x="140" y="22" font-size="11" opacity=".6" font-weight="700" letter-spacing="1">AUTHORING</text>
    <rect x="40" y="34" width="200" height="94" rx="7" fill="none" stroke="currentColor" stroke-width="1.5" opacity=".7"/>
    <text x="140" y="60" font-weight="600">Visual Graph</text>
    <text x="140" y="79" font-size="11.5" opacity=".72">Rumima Enterprise Studio</text>
    <text x="140" y="96" font-size="11.5" opacity=".72">or any other editor</text>
    <text x="140" y="116" font-size="11" opacity=".55">the authoritative design</text>

    <path d="M140 128 V162" fill="none" stroke="currentColor" stroke-width="1.5" opacity=".55" marker-end="url(#a2)"/>

    <rect x="40" y="166" width="200" height="60" rx="7" fill="none" stroke="currentColor" stroke-width="1.5" opacity=".7"/>
    <text x="140" y="191" font-weight="600">Internal Object Model</text>
    <text x="140" y="209" font-size="11.5" opacity=".72">nodes · edges · resources</text>

    <path d="M240 196 H300" fill="none" stroke="currentColor" stroke-width="1.5" opacity=".55" marker-end="url(#a2)"/>

    <rect x="304" y="150" width="192" height="92" rx="7" fill="currentColor" opacity=".12"/>
    <rect x="304" y="150" width="192" height="92" rx="7" fill="none" stroke="currentColor" stroke-width="2.5"/>
    <text x="400" y="178" font-weight="700" font-size="14">HarnessXML</text>
    <text x="400" y="197" font-size="11.5" opacity=".8">the document — .hxml</text>
    <text x="400" y="215" font-size="11.5" opacity=".8">portable · diffable · signable</text>
    <text x="400" y="233" font-size="11" opacity=".65">THE OPEN INTERCHANGE POINT</text>

    <path d="M496 196 H556" fill="none" stroke="currentColor" stroke-width="1.5" opacity=".55" marker-end="url(#a2)"/>

    <text x="740" y="22" font-size="11" opacity=".6" font-weight="700" letter-spacing="1">EXECUTION</text>
    <rect x="560" y="166" width="150" height="60" rx="7" fill="none" stroke="currentColor" stroke-width="1.5" opacity=".7"/>
    <text x="635" y="191" font-weight="600">Validator</text>
    <text x="635" y="209" font-size="11.5" opacity=".72">XSD + HX-nnnn rules</text>

    <path d="M710 196 H756" fill="none" stroke="currentColor" stroke-width="1.5" opacity=".55" marker-end="url(#a2)"/>

    <rect x="760" y="34" width="120" height="192" rx="7" fill="none" stroke="currentColor" stroke-width="1.5" opacity=".7"/>
    <text x="820" y="60" font-weight="600">Harness</text>
    <text x="820" y="78" font-weight="600">Runtime</text>
    <text x="820" y="104" font-size="11.5" opacity=".72">schedule</text>
    <text x="820" y="122" font-size="11.5" opacity=".72">resolve</text>
    <text x="820" y="140" font-size="11.5" opacity=".72">execute</text>
    <text x="820" y="158" font-size="11.5" opacity=".72">retry</text>
    <text x="820" y="176" font-size="11.5" opacity=".72">compensate</text>

    <path d="M820 226 V266" fill="none" stroke="currentColor" stroke-width="1.5" opacity=".55" marker-end="url(#a2)"/>
    <rect x="740" y="270" width="160" height="56" rx="7" fill="none" stroke="currentColor" stroke-width="1.5" opacity=".7"/>
    <text x="820" y="294" font-weight="600">Monitoring</text>
    <text x="820" y="312" font-size="11.5" opacity=".72">traces · audit · provenance</text>

    <path d="M740 298 H420 V250" fill="none" stroke="currentColor" stroke-width="1.2"
          stroke-dasharray="5 4" opacity=".45" marker-end="url(#a2)"/>
    <text x="560" y="345" font-size="11" opacity=".6">an execution trace resolves back to the document and design revision that produced it</text>
  </g>
</svg>
</div>

## The five stages

### 1. Authoring — the visual graph

Where the design lives. Rumima Enterprise Studio is VisML's commercial designer
and the flagship authoring environment, but nothing about the architecture
depends on it: the graph could come from another editor, from a generator, or
from a program emitting documents directly.

The requirement that matters is the **round trip**. A document generated from a
graph must reopen as that graph, without loss. If the round trip is lossy, teams
start editing the generated file by hand, and the graph stops being authoritative
— which collapses the whole model.

See [Visual Authoring](/visual-authoring/) for what an editor must preserve, why
layout is deliberately absent from the document, and how a host format embeds a
harness.

### 2. The internal object model

The in-memory shape a tool holds: nodes, typed edges, resources, artifacts,
metadata. The specification defines this object model normatively so that
independent implementations agree on *what a document means* before they argue
about how to run it.

The object model is where a designer's conveniences get resolved away. Layout
coordinates, colours and grouping are presentation, not semantics — they do not
belong in the executable document and do not affect execution.

### 3. HarnessXML — the interchange point

The serialised document. **This is the only part that has to be identical across
vendors**, which is why it is the only part the specification pins down
completely. Everything upstream and downstream is an implementation.

A document is:

- **complete** — sufficient to execute, not a sketch requiring code to fill in;
- **portable** — no vendor-specific semantics outside a declared extension namespace;
- **diffable** — a change to a threshold is one line in a pull request;
- **signable** — canonical enough to sign and verify years later.

### 4. Validation

A two-layer check, deliberately.

**Layer one: the XSD.** Structure, types, enumerations, and referential integrity
between edges, nodes, resources and artifacts — expressed as `xs:key` /
`xs:keyref`, so a *plain schema-validating parser in any language* already
rejects a dangling edge. No HarnessXML-aware tooling required.

**Layer two: specification rules.** What XSD 1.0 cannot express — acyclicity of
control flow, reachability, expression well-formedness, type compatibility across
a data edge, a retry policy on a non-idempotent node. Each rule carries an
`HX-nnnn` code, and each code has a conformance fixture that must be rejected
with exactly that code.

Validation is a gate, not advice. A conforming runtime **must not** execute an
invalid document.

### 5. Runtime and monitoring

A conforming runtime loads a validated document, resolves resources and
artifacts, and executes the graph according to the specified semantics: the node
lifecycle state machine, edge-type-driven scheduling, deterministic decisions,
bounded loops, declared retry policies, and compensation on failure.

Runtimes are expected to differ enormously in everything else — distribution,
persistence, crash recovery, scale, latency. That is where implementations should
compete. What they may not differ on is *what the document means*.

Monitoring closes the loop. Because the document carries provenance — the
generator, the source design and its digest, artifact digests — an execution
trace resolves back to the exact design revision that authorised it. That is the
difference between logs and an audit trail.

## What anyone may build

Everything except the specification itself:

| component | what it does |
|---|---|
| **Editors** | author graphs, emit HarnessXML |
| **Parsers** | read documents into an object model |
| **Validators** | enforce the XSD and the `HX-nnnn` rules |
| **Runtimes** | execute documents per the specification |
| **Compilers** | lower HarnessXML to another execution substrate |
| **Importers / exporters** | convert to and from BPMN, DAG YAML, framework graphs |
| **SDKs** | language bindings for building and inspecting documents |

No permission is required, no royalty is due, and there is no agreement to sign.
Check your implementation against the [conformance suite](/conformance/) rather
than against anyone's opinion — including VisML's.

## Where the reference implementation fits

`reference-runtime/` is an Apache-2.0 Rust implementation of the parser,
validator and execution model. Its job is to be **unambiguous, not fast**: it
exists so that every normative rule has running code and a test behind it, and so
that a disagreement about what the specification means can be settled by reading
an implementation instead of by arguing about prose.

It is explicitly **not** Rumima. If the reference runtime and Rumima disagree,
the specification decides, and at most one of them is right.
