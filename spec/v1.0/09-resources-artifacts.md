---
title: Resources and Artifacts
description: Declaring the external capabilities a HarnessXML workflow needs, and the identified data it consumes and produces.
section: specification
order: 10
status: draft
---

# 9. Resources and Artifacts

Two declarations that look similar and answer different questions.

- A **resource** is a *capability* the workflow needs but does not contain — a
  model endpoint, a database, a robot arm.
- An **artifact** is *identified data* that exists independently of any one
  execution — a dataset, a model file, a document, a log.

## 9.1 Resources

```xml
<resources>
  <resource id="classifier" type="model" name="Document classifier" provider="anthropic">
    <description>Vision-capable model used for OCR and classification.</description>
    <property name="model" value="claude-opus-5"/>
    <property name="temperature" value="0"/>
    <credential ref="ANTHROPIC_API_KEY" store="gcp-secret-manager"/>
  </resource>
</resources>
```

| attribute | meaning |
|---|---|
| `id` | required, unique among resources |
| `type` | required — `model`, `compute`, `datastore`, `queue`, `device`, `service`, `secretstore` |
| `name` | human-readable |
| `provider` | who supplies it — advisory, not interpreted |
| `uri` | endpoint or connection target |

Nodes reference resources rather than embedding connection details:

```xml
<resourceRef ref="classifier" role="model"/>
```

`role` says *how* the node uses it, which matters when a node needs two resources
of the same type — a model to classify with and a model to critique with.

### 9.1.1 Why the indirection exists

So that **moving a workflow between environments is a change to the resource
block and nothing else**. No node changes. The diff shows exactly what varies
between development, staging and production — which is precisely the thing a
reviewer wants to see and normally cannot.

A node that embedded its own endpoint would have to be edited per environment,
and the design would differ between them. Then the graph is no longer
authoritative, because there are several of it.

### 9.1.2 Credentials are referenced, never contained

```xml
<credential ref="ANTHROPIC_API_KEY" store="vault"/>
```

`ref` names a secret in a store; `store` names the store. **A document containing
a literal credential is invalid (`HX-3501`).**

A validator **MUST** reject a document where a credential-shaped value appears in
a `property` or port `value`. Detection is necessarily heuristic — high-entropy
strings, known key prefixes, anything matching a private-key header — and a
validator **SHOULD** report a warning rather than an error when it is unsure.

> These documents are designed to be committed to git, diffed in pull requests
> and archived for audit. Those are three excellent ways to publish a key. The
> format refuses to make it convenient, and the validator argues about it.

### 9.1.3 Resource resolution

A runtime resolves each referenced resource **before** the node executes. Failure
to resolve is a runtime error (`HX-4102`) and the node fails without side effects.

Two nodes referencing the same resource **MAY** share a connection. The
specification does not require pooling and does not forbid it.

## 9.2 Artifacts

```xml
<artifacts>
  <artifact id="taxonomy" type="config" name="Classification taxonomy"
            uri="gs://visml-config/taxonomy-v3.json"
            mediaType="application/json"
            digest="sha256:9f2c1b7ae4d0c8135e6a0b4f7c2d9e18a3b5c7d9e1f2a4b6c8d0e2f4a6b8c0d2"
            classification="internal"/>
</artifacts>
```

| attribute | meaning |
|---|---|
| `id` | required, unique among artifacts |
| `type` | required — `dataset`, `model`, `document`, `image`, `binary`, `config`, `log`, `report` |
| `uri` | where it lives |
| `mediaType` | IANA media type |
| `digest` | content digest, e.g. `sha256:…` |
| `classification` | `public`, `internal`, `confidential`, `restricted` |

Nodes declare their relationship to an artifact:

```xml
<artifactRef ref="taxonomy" direction="in"/>
```

`direction` is `in`, `out` or `inout`.

### 9.2.1 Digests

A `digest` is OPTIONAL and **RECOMMENDED** for every input artifact.

It is what makes a run reproducible and an audit trail meaningful. A trace saying
"classified against taxonomy `sha256:9f2c…`" answers the question asked two years
later. A trace saying "classified against `taxonomy-v3.json`" does not, because
that file has been edited since and nobody recorded when.

A runtime **SHOULD** verify the digest when it can, and **MUST** fail
(`HX-4105`) rather than proceed on a mismatch. A silently substituted input is
indistinguishable from a correct run in every log.

### 9.2.2 Artifacts are not ports

The distinction that causes the most confusion:

| | port | artifact |
|---|---|---|
| lifetime | one execution | independent of any execution |
| identity | a name on a node | a document-level id, often a digest |
| moves via | a data edge | a URI the runtime resolves |
| after the run | gone | still there |

A document being classified right now is a **port value**. The taxonomy it is
classified against is an **artifact**. Modelling the taxonomy as a port value
means a re-run cannot find it; modelling the document as an artifact means every
execution needs a new document declaration.

## 9.3 Classification

Both artifacts and `<security>` blocks carry a `classification`: `public`,
`internal`, `confidential`, `restricted`.

Classification is **declarative**. The specification does not define enforcement —
that is a runtime and organisational matter. What it defines is that the label
travels with the data, so a runtime **can** enforce, and a trace **can** be
audited against policy.

A runtime **SHOULD** warn when a node without a matching classification consumes
a more highly classified artifact. It **MUST NOT** silently downgrade.
