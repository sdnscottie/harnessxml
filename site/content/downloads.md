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

There are two extensions in this ecosystem, and only one of them is specified here:

| extension | format | open? |
|---|---|---|
| **`.hxml`** | **HarnessXML** — executable workflows, specified here | **open, vendor-neutral** |
| `.visml` | VisML Markup Language — the shared native format of VisML's products, including RuMima | vendor format |

A `.visml` document **embeds** a complete HarnessXML document as a child element,
alongside the canvas layout, colours and editor state that HarnessXML
deliberately excludes. Export is lifting that element out; import is wrapping it.
Nothing is translated, so nothing is lost in translation.

> **The dependency runs one way only.** HarnessXML must be fully definable,
> validatable and executable without reference to `.visml` or any other host
> format. HarnessXML is **not a subset of, profile of, or extension of** the
> VisML markup language — it is an independent specification that a VisML
> document happens to contain, exactly as an HTML page may contain an SVG
> document. **No conforming implementation is ever expected to read `.visml`.**
> See [§2.9.1](/spec/v1.0/document-structure/#2-9-1-hxml-and-visml-embedding-not-subsetting).

## Example documents

Real, complete, schema-valid documents. Each is also a conformance fixture, so
they cannot rot without CI noticing.

| example | domain | what it demonstrates |
|---|---|---|
| [`document-triage.hxml`](/examples-src/ai/document-triage.hxml) | AI orchestration | model inference, confidence-gated routing, human escalation, error edges |
| [`pick-and-place.hxml`](/examples-src/robotics/pick-and-place.hxml) | Robotics | bounded loops, non-idempotent physical actions, compensation |
| [`config-rollout.hxml`](/examples-src/networking/config-rollout.hxml) | Network automation | canary staging, quorum barrier, declared rollback per device |
| [`invoice-approval.hxml`](/examples-src/enterprise/invoice-approval.hxml) | Business process | approval routing by value, guards vs decisions, security and provenance |

## Reference implementation

Apache-2.0. Parser, validator and execution model in Rust, plus the `harnessxml`
CLI.

```bash
git clone https://github.com/scottsoft/harnessxml
cd harnessxml/reference-runtime
cargo build --release
```

```bash
harnessxml validate  workflow.hxml     # schema + specification rules
harnessxml graph     workflow.hxml     # print the resolved execution graph
harnessxml explain   workflow.hxml     # per-node scheduling analysis
```

Its job is to be unambiguous rather than fast — every normative rule has running
code and a test behind it.

## SDKs

Language bindings for building, reading and validating documents.

| language | package | status |
|---|---|---|
| Rust | `harnessxml` | reference implementation — in progress |
| Python | `harnessxml` | planned |
| Go | `github.com/scottsoft/harnessxml-go` | planned |
| Java | `com.visml:harnessxml` | planned |
| C# | `HarnessXml` | planned |
| JavaScript / TypeScript | `@harnessxml/core` | planned |

> **Planned means not written.** These are listed so the intended surface is
> public and so nobody duplicates work by accident — not to suggest something
> exists that you can install today. An SDK appears in this table as *available*
> only when it passes the [conformance suite](/conformance/).

## Conformance suite

The fixtures that define conformance: documents that must be accepted, documents
that must be rejected **with a specified error code**, and execution traces that
must match.

```bash
git clone https://github.com/scottsoft/harnessxml
python3 harnessxml/conformance/validate.py
```

Self-certification is the default — run it, publish your results, state your
level. See [conformance](/conformance/).

## Licensing

| what | licence |
|---|---|
| Specification text, this website | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) |
| Schemas, examples, reference code, conformance suite | [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0) |
| "HarnessXML", "VisML", "RuMima" | trademarks, not licensed by either |

See [licensing](/licensing/) for what that means in practice.
