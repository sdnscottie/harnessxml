# HarnessXML

**The Open Specification for Executable Intelligent System Workflows.**

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
| Reference examples (AI, robotics, networking, enterprise) | complete, schema-valid, used as fixtures |
| Reference validator (Rust) | working — parser + all layer-1/2 validation rules |
| Reference **executor** | not started |
| Conformance corpus | working runner, incomplete corpus |
| SDKs beyond Rust | not started |

## Repository layout

```
spec/v1.0/              the specification chapters (Markdown)
schema/v1.0/            harnessxml-1.0.xsd
examples/               reference documents, doubling as conformance fixtures
reference-runtime/      Rust parser + validator + `harnessxml` CLI
conformance/            the suite third parties run to prove compatibility
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

# run the conformance suite
python3 conformance/validate.py --cmd "reference-runtime/target/release/harnessxml validate"

# build the website (standard library only — no pip install)
python3 site/build.py --check --serve
```

## File extensions

| | | |
|---|---|---|
| **`.hxml`** | **HarnessXML** — executable workflows, specified here | **open, vendor-neutral** |
| `.visml` | VisML Markup Language — the shared native format of VisML's products, including RuMima | vendor format |

A `.visml` document **embeds** a complete HarnessXML document as a child element,
alongside the canvas layout and editor state that HarnessXML excludes. Export
lifts the element out; import wraps it.

**The dependency runs one way only.** HarnessXML must be fully definable,
validatable and executable without reference to `.visml` — it is *not* a subset,
profile or extension of any vendor format, exactly as SVG is not a subset of HTML
merely because an HTML page can contain one.

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

VisML created and stewards HarnessXML **and** sells RuMima Enterprise Studio, the
flagship commercial authoring environment. That is a real conflict of interest,
and [GOVERNANCE.md §1](GOVERNANCE.md) names it explicitly along with the
structural limits placed on it — no privileged extension point, a reference
runtime that is not RuMima, and conformance defined by tests rather than by
agreement with any product.

## Contributing

Ambiguity is the highest-value bug report. If two competent engineers can read a
normative sentence and build incompatible implementations, that sentence is
broken — please [tell us](CONTRIBUTING.md), and say what you thought it meant and
what the other reading is.

There is no CLA. See [CONTRIBUTING.md](CONTRIBUTING.md) and
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
