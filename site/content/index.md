---
title: HarnessXML
description: HarnessXML is the Open Specification for Executable Intelligent System Workflows — an open, vendor-neutral language and execution model for AI orchestration, agent systems, robotics, network and industrial automation.
section: introduction
order: 1
status: stable
---

# HarnessXML

<p class="mission">HarnessXML is the Open Specification for Executable Intelligent
System Workflows.</p>

An intelligent system is rarely one model or one program. It is a *workflow*: a
graph of steps that call models, move data, drive machines, wait on people, fail,
retry and compensate. That graph is the real design — and on most teams it exists
only as a diagram on a wall and a pile of orchestration code that has drifted
away from it.

HarnessXML makes the graph the artifact. One document describes the workflow
completely enough to execute, validate, version, sign and audit — and it is
portable across the tools and runtimes that read it.

<div class="cta-row">
  <a class="cta cta-primary" href="/spec/v1.0/">Read the specification</a>
  <a class="cta cta-secondary" href="/spec/v1.0/concepts/">Start with the concepts</a>
  <a class="cta cta-secondary" href="/downloads/">Download schemas &amp; tools</a>
</div>

## The three ideas

<div class="card-grid">
  <div class="card">
    <h3>The visual graph is the authoritative design</h3>
    <p>People reason about workflows as pictures. The diagram is not documentation
       of the system — it <em>is</em> the system's design, and it should stay that way.</p>
  </div>
  <div class="card">
    <h3>HarnessXML is the portable representation</h3>
    <p>A machine-readable form of that design: complete, diffable, reviewable in a
       pull request, and readable by any tool that implements the specification.</p>
  </div>
  <div class="card">
    <h3>The runtime executes the specification</h3>
    <p>The same document produces the same execution semantics on any conforming
       runtime. Behaviour is defined by the specification, not by one vendor's engine.</p>
  </div>
</div>

## How the pieces fit

<div class="diagram">
<svg viewBox="0 0 940 210" xmlns="http://www.w3.org/2000/svg" role="img"
     aria-labelledby="archtitle archdesc">
  <title id="archtitle">HarnessXML architecture pipeline</title>
  <desc id="archdesc">A visual graph authored in RuMima becomes an internal object
    model, is serialised to a HarnessXML document, checked by a validator, executed
    by a harness runtime, and observed through monitoring. The HarnessXML document
    is the open interchange point that any tool may produce or consume.</desc>
  <defs>
    <marker id="arw" viewBox="0 0 10 10" refX="9" refY="5"
            markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M0 0 L10 5 L0 10 z" fill="currentColor"/>
    </marker>
  </defs>
  <g fill="none" stroke="currentColor" stroke-width="1.5" opacity=".55" marker-end="url(#arw)">
    <path d="M132 62 H172"/>
    <path d="M292 62 H332"/>
    <path d="M452 62 H492"/>
    <path d="M612 62 H652"/>
    <path d="M772 62 H812"/>
  </g>

  <g font-size="12.5" text-anchor="middle">
    <g>
      <rect x="12" y="34" width="120" height="56" rx="6"
            fill="none" stroke="currentColor" stroke-width="1.5" opacity=".75"/>
      <text x="72" y="57" fill="currentColor" font-weight="600">RuMima</text>
      <text x="72" y="74" fill="currentColor" opacity=".7">Visual Graph</text>
    </g>
    <g>
      <rect x="172" y="34" width="120" height="56" rx="6"
            fill="none" stroke="currentColor" stroke-width="1.5" opacity=".75"/>
      <text x="232" y="57" fill="currentColor" font-weight="600">Object Model</text>
      <text x="232" y="74" fill="currentColor" opacity=".7">in-memory</text>
    </g>
    <g>
      <rect x="332" y="26" width="120" height="72" rx="6"
            fill="currentColor" opacity=".12"/>
      <rect x="332" y="26" width="120" height="72" rx="6"
            fill="none" stroke="currentColor" stroke-width="2.5"/>
      <text x="392" y="53" fill="currentColor" font-weight="700">HarnessXML</text>
      <text x="392" y="70" fill="currentColor" opacity=".75">the document</text>
      <text x="392" y="86" fill="currentColor" opacity=".75" font-size="10.5">open · vendor-neutral</text>
    </g>
    <g>
      <rect x="492" y="34" width="120" height="56" rx="6"
            fill="none" stroke="currentColor" stroke-width="1.5" opacity=".75"/>
      <text x="552" y="57" fill="currentColor" font-weight="600">Validator</text>
      <text x="552" y="74" fill="currentColor" opacity=".7">HX-nnnn codes</text>
    </g>
    <g>
      <rect x="652" y="34" width="120" height="56" rx="6"
            fill="none" stroke="currentColor" stroke-width="1.5" opacity=".75"/>
      <text x="712" y="57" fill="currentColor" font-weight="600">Harness Runtime</text>
      <text x="712" y="74" fill="currentColor" opacity=".7">execution</text>
    </g>
    <g>
      <rect x="812" y="34" width="116" height="56" rx="6"
            fill="none" stroke="currentColor" stroke-width="1.5" opacity=".75"/>
      <text x="870" y="57" fill="currentColor" font-weight="600">Monitoring</text>
      <text x="870" y="74" fill="currentColor" opacity=".7">traces · audit</text>
    </g>
  </g>

  <g stroke="currentColor" stroke-width="1.2" stroke-dasharray="4 4" opacity=".45" fill="none">
    <path d="M392 98 V140"/>
    <path d="M120 140 H820"/>
    <path d="M120 140 V126"/>
    <path d="M820 140 V126"/>
  </g>
  <text x="470" y="164" text-anchor="middle" font-size="12" fill="currentColor" opacity=".75">
    anyone may build an editor, parser, validator, runtime, compiler or SDK against the specification
  </text>
  <text x="470" y="184" text-anchor="middle" font-size="11.5" fill="currentColor" opacity=".55">
    RuMima Enterprise Studio is VisML's commercial authoring environment — one implementation, not the definition
  </text>
</svg>
</div>

**RuMima is the flagship commercial visual designer for HarnessXML. HarnessXML is
not RuMima's file format.** The specification is open and vendor-neutral: build
your own editor, parser, validator, runtime, compiler, importer, exporter or SDK
against it, commercially, without asking permission. If the specification and
RuMima ever disagree, the specification is right and RuMima has a bug.

## What a document looks like

```xml
<harness xmlns="https://harnessxml.com/spec/1.0"
         id="document_triage" specVersion="1.0" entry="receive">

  <resources>
    <resource id="classifier" type="model" provider="anthropic">
      <property name="model" value="claude-opus-5"/>
      <credential ref="ANTHROPIC_API_KEY" store="vault"/>
    </resource>
  </resources>

  <nodes>
    <node id="receive" type="source" impl="intake.receive">
      <outputs><output name="document" type="binary"/></outputs>
    </node>

    <node id="classify" type="inference">
      <inputs><input name="document" type="binary"/></inputs>
      <outputs>
        <output name="category"   type="string"/>
        <output name="confidence" type="number"/>
      </outputs>
      <resourceRef ref="classifier" role="model"/>
      <retry maxAttempts="4" backoff="exponential" retryOn="rate_limit transient"/>
      <timeout duration="PT3M" onTimeout="retry"/>
    </node>

    <node id="route" type="decision">
      <cases>
        <case when="${classify.confidence >= 0.90}" to="auto_file"/>
        <otherwise to="human_review"/>
      </cases>
    </node>
  </nodes>

  <edges>
    <edge from="receive"  to="classify" type="data" fromPort="document" toPort="document"/>
    <edge from="classify" to="route"    type="control"/>
  </edges>
</harness>
```

Nothing above is a hint to a human reader. The retry policy, the timeout, the
confidence threshold and the escalation path are all **executable** — and all
reviewable in a diff before they reach production.

## What it is for

<div class="card-grid">
  <div class="card"><h3>AI orchestration &amp; agent systems</h3>
    <p>Model calls, tool use, confidence gating, human escalation, and the retry
       and rate-limit behaviour that decides whether any of it survives contact
       with a real API.</p></div>
  <div class="card"><h3>Machine learning pipelines</h3>
    <p>Datasets and models as first-class, digest-identified artifacts, so a run
       is reproducible and a result is traceable to the inputs that produced it.</p></div>
  <div class="card"><h3>Robotics</h3>
    <p>Bounded loops, non-idempotent physical actions that must never be blindly
       retried, and compensation paths that put the part back.</p></div>
  <div class="card"><h3>Network &amp; industrial automation</h3>
    <p>Staged rollouts, quorum gates, and a declared rollback for every push —
       so the undo path is in the design instead of in a runbook at 02:00.</p></div>
  <div class="card"><h3>Business process automation</h3>
    <p>Approval routing, separation of duties, and an audit trail that ties an
       execution back to the exact design revision that authorised it.</p></div>
  <div class="card"><h3>Whatever comes next</h3>
    <p>The specification describes the workflow and stops at the boundary of what
       a step does — which is why it does not go stale when the steps change.</p></div>
</div>

## Openness is structural, not a slogan

| | |
|---|---|
| **Specification text** | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) — copy, translate, quote, build on it, commercially, with attribution |
| **Reference code, schemas, examples** | [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0), including an express patent grant |
| **Implementing it** | No permission, no royalty, no notification, no agreement to sign |
| **Released versions** | Frozen at a permanent URL, forever. Corrections are dated errata, never edits |
| **Changes** | Public proposals, 30-day minimum review, rejected proposals stay published |
| **Conformance** | Defined by a published test suite, not by agreement with any implementation |

Read the [governance model](/governance/) — including the conflict of interest
VisML has, and the structural limits placed on it.
