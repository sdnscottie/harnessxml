---
title: Downloads
description: Download HarnessXML schemas, example documents, the reference runtime, the CLI and SDKs. Every released schema stays at a permanent URL forever.
section: implementing
order: 1
status: stable
---

# Download Center

Everything here is permanent. A released schema URL keeps serving the same bytes
forever — see [versioning and immutability](/governance/#4-versioning-and-immutability).

## Schemas

| version | schema | namespace | status |
|---|---|---|---|
| **1.0** | [`harnessxml-1.0.xsd`](/schema/v1.0/harnessxml-1.0.xsd) | `https://harnessxml.com/spec/1.0` | draft |

Point your editor at the schema for completion and inline validation:

```xml
<harness xmlns="https://harnessxml.com/spec/1.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="https://harnessxml.com/spec/1.0
                             https://harnessxml.com/schema/v1.0/harnessxml-1.0.xsd"
         id="my_workflow" specVersion="1.0">
```

Validate from the command line with any XSD-capable tool:

```bash
# xmllint
xmllint --noout --schema harnessxml-1.0.xsd my_workflow.hxml

# python, standard tooling
python3 -c "from lxml import etree; \
  etree.XMLSchema(etree.parse('harnessxml-1.0.xsd')).assertValid(etree.parse('my_workflow.hxml'))"
```

> The XSD is normative for **structure** only. Rules XSD 1.0 cannot express —
> acyclicity, reachability, expression well-formedness, retry-on-non-idempotent —
> are normative in the specification text and carry `HX-nnnn` codes. A document
> that passes the schema is not necessarily valid HarnessXML.

## File extension and media type

| | |
|---|---|
| extension | **`.hxml`** |
| media type | `application/harnessxml+xml` |

Neither affects validity — an implementation **must** recognise a HarnessXML
document from its root element and namespace, never from its filename. The
extension exists so editors, diff tools and operators recognise a workflow at a
glance.

Three names appear around HarnessXML, at three layers. Only `.hxml` crosses the
boundary between tools:

| name | layer | open? |
|---|---|---|
| **`.hxml`** | **interchange** — HarnessXML, specified here | **open, vendor-neutral** |
| `.visml` | markup standard used *inside* Rumima documents | vendor format |
| `.rmmx` | the file a Rumima document is saved as | vendor format |

```
Rumima Enterprise Studio  →  .rmmx  →  .hxml  →  any conforming runtime  →  execution
```

A `.rmmx` document contains `.visml` markup, which **embeds a complete
HarnessXML document**. Export lifts that element out; import wraps it. Nothing
is translated, so nothing is lost in translation.

> **The dependency runs one way only.** HarnessXML must be fully definable,
> validatable and executable without reference to `.visml`, `.rmmx` or any other
> host format. It is **not a subset of, profile of, or extension of** any of
> them — it is an independent specification that a host document happens to
> contain, exactly as an HTML page may contain an SVG document. **No conforming
> implementation is ever expected to read `.rmmx` or `.visml`.** See
> [§2.9.1](/spec/v1.0/document-structure/#2-9-1-hxml-visml-and-rmmx-embedding-not-subsetting).

## Example documents

Real, complete, schema-valid documents. Each is also a conformance fixture, so
they cannot rot without CI noticing.

| example | domain | what it demonstrates |
|---|---|---|
| [`document-triage.hxml`](/examples-src/ai/document-triage.hxml) | AI orchestration | model inference, confidence-gated routing, human escalation, error edges |
| [`pick-and-place.hxml`](/examples-src/robotics/pick-and-place.hxml) | Robotics | bounded loops, non-idempotent physical actions, compensation |
| [`config-rollout.hxml`](/examples-src/networking/config-rollout.hxml) | Network automation | canary staging, quorum barrier, declared rollback per device |
| [`invoice-approval.hxml`](/examples-src/enterprise/invoice-approval.hxml) | Business process | approval routing by value, guards vs decisions, security and provenance |
| [`weight-training.hxml`](/examples-src/training/weight-training.hxml) | Adaptive training | domain customisation with zero new core constructs — a barbell, a sensor and a human instead of a server |

### Authored as visual maps

The same harnesses as **Rumima `.rmmx` documents**, with screenshots of the
actual application rendering them:

| map | demonstrates |
|---|---|
| [`document-triage.rmmx`](/examples-src/rmmx/document-triage.rmmx) | the five typed edge relationships, confidence-gated routing, a non-idempotent human step |
| [`weighted-model-router.rmmx`](/examples-src/rmmx/weighted-model-router.rmmx) | weighted model selection, LoRA finetuning as a declared resource property, quality-gated escalation from a local qwen3 to a frontier model |

Opening them needs [Rumima](https://rumima.visml.com/); reading them does not —
every node's description carries its HarnessXML fragment. The `.hxml` these
export to is vendor-neutral and runs on any conforming runtime.

## Reference implementation

Apache-2.0. Parser, validator and execution model in Rust, plus the `harnessxml`
CLI.

```bash
git clone https://gitlab.com/visml/harnessxml
cd harnessxml/reference-runtime
cargo build --release
```

```bash
harnessxml validate workflow.hxml                    # schema + specification rules
harnessxml graph    workflow.hxml                    # the resolved execution graph
harnessxml explain  workflow.hxml                    # what each node waits for
harnessxml run      workflow.hxml                    # EXECUTE it
harnessxml run      workflow.hxml --scenario s.txt --trace
```

The executor implements the full model of chapters 5–8: the lifecycle state
machine, join policies, guards, decisions in document order, the four loop kinds
with `maxIterations` enforced, retry policies with backoff, and compensation
unwinding in reverse completion order. Because `impl` is opaque, node outcomes
come from a **scenario script**, which is what makes a run — and therefore a
conformance trace — reproducible.

Its job is to be unambiguous rather than fast — every normative rule has running
code and a test behind it.

## SDKs

Language bindings for building, reading and validating documents.

| language | package | status | conformance |
|---|---|---|---|
| Rust | `harnessxml` (`reference-runtime/`) | **available** — parser, validator **and executor** | Executing |
| Python | `harnessxml` (`sdk/python/`) | **available** — parser, validator, document builder | Core |
| Go | `gitlab.com/visml/harnessxml/sdk/go` | **available** — parser, validator, CLI | Core |
| Java | `com.visml:harnessxml` | planned | — |
| C# | `HarnessXml` | planned | — |
| JavaScript / TypeScript | `@harnessxml/core` | planned | — |

> **Planned means not written.** Those rows are listed so the intended surface
> is public and nobody duplicates work by accident — not to suggest something
> exists that you can install today.
>
> An SDK is marked **available** only when it passes the
> [conformance suite](/conformance/). All three that are, do: Rust, Python and Go
> each accept every valid fixture and reject every invalid one **with the same
> `HX-nnnn` code**. Three independent codebases in three languages reaching
> identical verdicts is the best evidence available that the specification is
> precise enough to implement from.

## Conformance suite

The fixtures that define conformance: documents that must be accepted, documents
that must be rejected **with a specified error code**, and execution traces that
must match.

```bash
git clone https://gitlab.com/visml/harnessxml
python3 harnessxml/conformance/validate.py
```

Self-certification is the default — run it, publish your results, state your
level. See [conformance](/conformance/).

## Licensing

| what | licence |
|---|---|
| Specification text, this website | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) |
| Schemas, examples, reference code, conformance suite | [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0) |
| "HarnessXML", "VisML", "Rumima" | trademarks, not licensed by either |

See [licensing](/licensing/) for what that means in practice.
