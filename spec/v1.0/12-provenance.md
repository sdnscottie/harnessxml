---
title: Metadata and Provenance
description: Recording what a HarnessXML document is, where it came from, and how an execution trace resolves back to the design that authorised it.
section: specification
order: 13
status: draft
---

# 12. Metadata and Provenance

## 12.1 Metadata

```xml
<metadata>
  <title>Invoice intake, approval and payment</title>
  <description>Extracts invoice fields, matches the purchase order, routes for
    approval by value, and releases payment.</description>
  <author>VisML</author>
  <organization>VisML</organization>
  <created>2026-08-04T09:00:00Z</created>
  <modified>2026-08-04T14:20:00Z</modified>
  <license>Apache-2.0</license>
  <documentVersion>3</documentVersion>
  <tags><tag>finance</tag><tag>approval</tag></tags>
  <provenance>…</provenance>
</metadata>
```

All elements are OPTIONAL and carry no execution semantics. A runtime **MUST NOT**
change behaviour based on metadata.

`documentVersion` is the **author's** version of this workflow — distinct from
`specVersion`, which is the version of HarnessXML it is written against. The two
are confused often enough to be worth naming separately rather than overloading
one attribute.

## 12.2 Provenance

```xml
<provenance>
  <generator name="RuMima Enterprise Studio" version="1.0" vendor="VisML"/>
  <source uri="visml://finance/invoice-approval.visml"
          type="visual-graph"
          digest="sha256:1a2b3c…"/>
  <signature algorithm="ed25519" keyId="visml-release-2026" value="…"/>
</provenance>
```

| element | records |
|---|---|
| `generator` | the tool that produced this document |
| `source` | what it was produced *from*, ideally with a digest |
| `signature` | a signature over the canonical document |

### 12.2.1 Why this is in the core

Because of the [philosophy](/philosophy/): the visual graph is the authoritative
design, and HarnessXML is its portable representation. That claim is only
checkable if a document can point back at the design it came from.

Without provenance, an execution trace says "workflow `invoice_approval` ran".
With it, the trace resolves to the exact document, the exact source design, and
the exact digests of both — which is what an incident review, an audit or a
regulator actually asks for.

`source/@digest` is where most of the value sits. A URI alone identifies a file
that has since been edited by someone who did not record when. A digest
identifies the bytes.

## 12.3 Artifact provenance

Artifact digests ([chapter 9](/spec/v1.0/resources-artifacts/)) extend the same
chain to data:

```xml
<artifact id="taxonomy" type="config"
          uri="gs://visml-config/taxonomy-v3.json"
          digest="sha256:9f2c1b…"/>
```

Together with document provenance this gives a complete chain for any execution:

```
design (.visml, digest)
  └── document (.hxml, digest, signed)
        └── inputs (artifacts, digests)
              └── execution trace (node states, timings, model identities)
```

Every link is content-identified. That is the difference between logs and an
audit trail: logs tell you what a system reported, and an audit trail lets you
verify it.

## 12.4 Execution trace requirements

A runtime **SHOULD** emit a trace. When it does, the trace **SHOULD** record:

| | |
|---|---|
| document identity | harness `@id`, `documentVersion`, document digest |
| provenance | `generator`, `source` uri and digest |
| instance | a unique execution identity, start and end time |
| per node | id, every state transition with timestamp, attempt number |
| per failure | error class, message, and whether it was retried or compensated |
| resources | resolved resource identity — **including which model actually answered** |
| artifacts | resolved uri and verified digest for every artifact read or written |

A runtime **MUST NOT** write resolved credentials into a trace, including inside
error messages ([§11.7](/spec/v1.0/security/#11-7-credentials)).

> Recording the **resolved model identity** matters more than it looks. "Which
> model produced this classification" is the first question asked after a bad
> output, and a provider alias that silently pointed at a different model version
> last Tuesday is the answer often enough to be worth capturing every time.

## 12.5 What provenance does not do

It records claims. It does not verify them.

A `generator` element saying a document came from a particular tool is an
assertion by whatever wrote the file. A `source` digest is only as trustworthy as
the pipeline that computed it.

**Signatures are what turn a claim into evidence.** Provenance without a
signature is documentation; provenance with a verified signature is an audit
record. The specification supports both because most workflows need the first and
only some need the second — but it should be clear which one a given document
has.
