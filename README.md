# HarnessXML

**The Open Specification for Executable Intelligent System Workflows.**

[![pipeline](https://gitlab.com/visml/harnessxml/badges/main/pipeline.svg)](https://gitlab.com/visml/harnessxml/-/pipelines)
[![Spec text: CC BY 4.0](https://img.shields.io/badge/spec-CC%20BY%204.0-blue)](LICENSE-SPEC)
[![Code: Apache 2.0](https://img.shields.io/badge/code-Apache%202.0-blue)](LICENSE-CODE)
[![Specification](https://img.shields.io/badge/spec-v1.0%20draft-orange)](https://harnessxml.com/spec/v1.0/)

[harnessxml.com](https://harnessxml.com/) · Specification text
[CC BY 4.0](LICENSE-SPEC) · Code [Apache 2.0](LICENSE-CODE)

An intelligent system is rarely one model or one program. It is a *workflow*: a
graph of steps that call models, move data, drive machines, wait on people, fail,
retry and compensate. On most teams that graph exists only as a diagram on a wall
and a pile of orchestration code that has drifted away from it.

HarnessXML makes the graph the artifact — one document, complete enough to
execute, validate, version, sign and audit, and portable across the tools and
runtimes that read it.

```xml
<harness xmlns="https://harnessxml.com/spec/1.0" id="triage" specVersion="1.0">
  <nodes>
    <node id="classify" type="inference">
      <resourceRef ref="model" role="model"/>
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
    <edge from="classify" to="route" type="control"/>
  </edges>
</harness>
```

The retry policy, the timeout, the confidence threshold and the escalation path
are **executable**, and all of them are reviewable in a diff before they reach
production.

## Status

**v1.0 is a draft and is not frozen.** Every documentation page states its own
status, and the [roadmap](site/content/roadmap.md) states the release gate. An
open specification that overstates its readiness burns the credibility it needs,
so the labels are deliberately unflattering.

| piece | state |
|---|---|
| Language design, object model, XSD | drafted; schema compiles and enforces referential integrity |
| Specification chapters 1–16 | written, marked draft |
| Reference examples (AI, robotics, networking, enterprise, training) | complete, schema-valid, used as fixtures |
| Reference validator (Rust) | working — parser + all layer-1/2 validation rules |
| Reference **executor** (Rust) | **working** — lifecycle, joins, decisions, loops, retries, compensation, traces |
| Expression language | working — full evaluator, 11 tests |
| Conformance corpus | working runner, incomplete corpus |
| Python SDK (`sdk/python/`) | **working** — parser, validator, builder, 25 tests, Core conformance |
| Go SDK (`sdk/go/`) | **working** — parser, validator, CLI, Core conformance |
| SDKs beyond Rust/Python/Go | not started |

## Repository layout

```
spec/v1.0/              the specification chapters (Markdown)
schema/v1.0/            harnessxml-1.0.xsd
examples/               reference documents, doubling as conformance fixtures
reference-runtime/      Rust parser + validator + `harnessxml` CLI
conformance/            the suite third parties run to prove compatibility
sdk/python/             Python SDK — stdlib only, parser + validator + builder
sdk/go/                 Go SDK — stdlib only, parser + validator + CLI
site/                   harnessxml.com — stdlib-only static site generator
deploy/                 GCP deployment (GCS + Cloud CDN behind the shared LB)
GOVERNANCE.md           stewardship, HXEP process, compatibility policy
```

## Quick start

```bash
# validate a document
cd reference-runtime && cargo build --release
./target/release/harnessxml validate ../examples/ai/document-triage.hxml

# see what each node waits for
./target/release/harnessxml explain ../examples/robotics/pick-and-place.hxml

# EXECUTE it, and print the resulting node states
./target/release/harnessxml run ../examples/ai/document-triage.hxml

# execute a scripted scenario and emit the trace as JSON
./target/release/harnessxml run ../examples/enterprise/invoice-approval.hxml \
  --scenario ../conformance/scenarios/payment-fails.txt --trace

# run the conformance suite
python3 conformance/validate.py --cmd "reference-runtime/target/release/harnessxml validate"

# build the website (standard library only — no pip install)
python3 site/build.py --check --serve
```

## File extensions

| name | layer | open? |
|---|---|---|
| **`.hxml`** | **interchange** — HarnessXML, specified here | **open, vendor-neutral** |
| `.visml` | markup standard used *inside* Rumima documents | vendor format |
| `.rmmx` | the file a Rumima document is saved as | vendor format |

```
Rumima Enterprise Studio  →  .rmmx  →  .hxml  →  any conforming runtime  →  execution
```

A `.rmmx` document contains `.visml` markup, which **embeds a complete HarnessXML
document**. Export lifts the element out; import wraps it. Only `.hxml` crosses
the boundary between tools.

**The dependency runs one way only.** HarnessXML must be fully definable,
validatable and executable without reference to `.visml`, `.rmmx` or any other
host format — it is *not* a subset, profile or extension of any of them, exactly
as SVG is not a subset of HTML merely because an HTML page can contain one.

## Openness

| | |
|---|---|
| Implementing it | no permission, no royalty, no agreement to sign |
| Specification text | [CC BY 4.0](LICENSE-SPEC) |
| Code, schemas, examples, tests | [Apache 2.0](LICENSE-CODE), incl. patent grant |
| Released versions | frozen at a permanent URL forever; corrections are dated errata |
| Changes | public proposals, 30-day minimum review, rejected ones stay published |
| Conformance | a published test suite, not agreement with any implementation |
| Trademarks | reserved — the one lever that makes a conformance claim mean something |

VisML created and stewards HarnessXML **and** sells Rumima Enterprise Studio, the
flagship commercial authoring environment. That is a real conflict of interest,
and [GOVERNANCE.md §1](GOVERNANCE.md) names it explicitly along with the
structural limits placed on it — no privileged extension point, a reference
runtime that is not Rumima, and conformance defined by tests rather than by
agreement with any product.

## Contributing

Ambiguity is the highest-value bug report. If two competent engineers can read a
normative sentence and build incompatible implementations, that sentence is
broken — please [tell us](CONTRIBUTING.md), and say what you thought it meant and
what the other reading is.

There is no CLA. See [CONTRIBUTING.md](CONTRIBUTING.md) and
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
