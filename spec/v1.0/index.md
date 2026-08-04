---
title: Specification v1.0
description: The HarnessXML 1.0 specification — status, editors, conventions, and how to read it.
section: specification
order: 1
status: draft
---

# HarnessXML Specification, Version 1.0

**Status:** Draft · **Namespace:** `https://harnessxml.com/spec/1.0` ·
**Schema:** [`harnessxml-1.0.xsd`](/schema/v1.0/harnessxml-1.0.xsd) ·
**Steward:** [VisML](https://visml.com)

> **This version is a draft and is not yet frozen.** Constructs may change before
> release. Once v1.0 is released it becomes immutable — permanently available at
> this URL, corrected only by dated errata, never edited. See
> [governance §4](/governance/#4-versioning-and-immutability).

## Abstract

HarnessXML is an open, vendor-neutral specification for describing **executable
intelligent system workflows**: graphs of steps that invoke models, move data,
drive machinery, wait on people, fail, retry and compensate.

The specification defines an object model, an XML serialisation of it, a set of
typed relationships between nodes, and an execution model precise enough that
independent runtimes execute the same document identically.

It deliberately does **not** define what an individual step does. A node carries
an opaque implementation handle the runtime resolves. HarnessXML describes the
workflow and stops at the boundary of the work.

## Scope

**In scope:** the document structure; the object model; node and edge types;
execution semantics including dependency resolution, scheduling, conditional
execution, loops, retries, timeouts and compensation; the node lifecycle state
machine; resource and artifact declarations; metadata and provenance; the
security model; validation rules and their error codes; versioning and
compatibility guarantees; conformance levels.

**Out of scope:** step implementations; the wire protocol between a runtime and
the systems it calls; distribution, persistence and crash recovery within a
runtime; visual layout, colour and grouping (presentation, not semantics); the
authoring user interface.

## How to read this specification

Chapters are ordered so that each depends only on those before it.

| | chapter | what it covers |
|---|---|---|
| 1 | [Concepts and Object Model](/spec/v1.0/concepts/) | the vocabulary and the in-memory model everything else refers to |
| 2 | [Document Structure](/spec/v1.0/document-structure/) | the top-level document and its sections |
| 3 | [Nodes](/spec/v1.0/nodes/) | node types, ports, configuration |
| 4 | [Typed Relationships](/spec/v1.0/edges/) | edges, and what each type means to the scheduler |
| 5 | [Execution Semantics](/spec/v1.0/execution-semantics/) | dependency resolution, readiness, scheduling, joins |
| 6 | [Execution Lifecycle](/spec/v1.0/lifecycle/) | the node state machine |
| 7 | [Conditionals and Loops](/spec/v1.0/control-flow/) | guards, decisions, the four loop kinds |
| 8 | [Failure, Retries and Compensation](/spec/v1.0/failure/) | retry policy, timeouts, idempotence, rollback |
| 9 | [Resources and Artifacts](/spec/v1.0/resources-artifacts/) | external capabilities and identified data |
| 10 | [Expressions](/spec/v1.0/expressions/) | the expression language |
| 11 | [Security Model](/spec/v1.0/security/) | principals, permissions, classification, credentials |
| 12 | [Metadata and Provenance](/spec/v1.0/provenance/) | traceability back to the design |
| 13 | [Validation Rules](/spec/v1.0/validation/) | every normative rule, by code |
| 14 | [Error Reporting](/spec/v1.0/errors/) | the `HX-nnnn` code space and diagnostic requirements |
| 15 | [Versioning and Compatibility](/spec/v1.0/versioning/) | what may change, and when |
| 16 | [Glossary](/spec/v1.0/glossary/) | normative definitions |

Each chapter carries its own status. **Draft** means not yet normative.
**Planned** means not yet written — an outline of intent only, and nothing should
be implemented against it.

## Conventions

### Requirement keywords

**MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**,
**SHOULD NOT**, **RECOMMENDED**, **MAY** and **OPTIONAL** are to be interpreted as
described in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119) and
[RFC 8174](https://www.rfc-editor.org/rfc/rfc8174), and only when in capitals.

Lowercase uses of these words carry their ordinary English meaning and impose no
requirement.

### Error codes

Every normative rule that a validator or runtime can check carries an identifier
of the form `HX-nnnn`. A conforming implementation rejecting a document **MUST**
report the specified code — see [Error Reporting](/spec/v1.0/errors/). This is
part of conformance: two validators that reject the same document for different
stated reasons give their users incompatible diagnostics.

### Namespaces

The namespace URI for this major version is:

```
https://harnessxml.com/spec/1.0
```

Minor versions do **not** change it. A namespace change breaks every existing
document, which is exactly what a minor version promises not to do.

### Examples

Examples are **non-normative** unless the surrounding text says otherwise. Every
example in this specification is a complete, schema-valid document or a fragment
excerpted from one — never a sketch with an ellipsis standing in for something
that would not actually validate.

### Schema vs. text

The XSD is normative for **structure**: element order, cardinality, datatypes,
enumerations, and referential integrity between edges, nodes, resources and
artifacts.

Rules XSD 1.0 cannot express — acyclicity of control flow, reachability,
expression well-formedness, type compatibility across a data edge, a retry policy
on a non-idempotent node — are normative **in this text**.

A document that validates against the schema is therefore not necessarily valid
HarnessXML. Both layers apply.

## Editors

- VisML (Scottie von Bruchhausen)

Editors are accountable for the precision of normative text. Ambiguity is a bug —
if two competent engineers can read a normative sentence and build incompatible
implementations, [report it](/contributing/).

## Licence

This specification text is licensed under
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). The schema, examples
and reference implementation are licensed under
[Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0). See
[licensing](/licensing/).
