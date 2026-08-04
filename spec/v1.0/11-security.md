---
title: Security Model
description: Principals, permissions, data classification, isolation and credential handling in HarnessXML — what the specification enforces and what it deliberately leaves to the runtime.
section: specification
order: 12
status: draft
---

# 11. Security Model

## 11.1 What this chapter does and does not do

HarnessXML's security model is **declarative**. It defines how a workflow states
what identity a step runs as, what it may touch, and how sensitive its data is.

It does **not** define enforcement. Authentication, authorisation, sandboxing and
key management belong to the runtime and to the organisation deploying it.

That division is deliberate and worth being explicit about, because a
specification that pretended to enforce would be claiming a guarantee it cannot
make. What the format *can* do is ensure the intent is **written down, travels
with the workflow, and is visible in a diff** — so that a runtime is able to
enforce and an auditor is able to check.

## 11.2 The `<security>` element

Appears at document level and on any node. Node-level settings override
document-level ones.

```xml
<harness …>
  <security classification="confidential" principal="ap-automation"/>

  <nodes>
    <node id="pay" type="task" impl="payments.release">
      <security classification="restricted" principal="ap-payments" isolation="container">
        <permission action="execute" resource="payments:sepa_transfer"/>
      </security>
    </node>
  </nodes>
</harness>
```

| attribute | meaning |
|---|---|
| `principal` | the identity this node runs as |
| `classification` | `public`, `internal`, `confidential`, `restricted` |
| `isolation` | requested execution isolation, e.g. `process`, `container`, `vm` |

## 11.3 Principals

`principal` is an **opaque** string the runtime maps onto its own identity system
— a service account, a role ARN, a Kubernetes service account, an LDAP entry.

The specification does not define the format, because every deployment has one
already and inventing a thirteenth would help nobody.

What it does define: **a runtime that cannot resolve a declared principal MUST
fail the node rather than fall back to a default identity** (`HX-4201`). Falling
back to "whatever the runtime runs as" is how a step meant to run with narrow
permissions ends up running with broad ones — silently, and only discovered
afterwards.

Declaring different principals on different nodes is how **separation of duties**
is expressed. The node that posts to the ledger and the node that releases
payment can run as different identities, and the document says so where a
reviewer sees it.

## 11.4 Permissions

```xml
<security>
  <permission action="read"    resource="erp:purchase_orders"/>
  <permission action="write"   resource="ledger:accounts_payable"/>
  <permission action="execute" resource="payments:sepa_transfer"/>
</security>
```

A **declaration of intent**: what this node needs in order to work. Both `action`
and `resource` are opaque strings the runtime interprets.

Two things this buys, neither of which requires the runtime to enforce anything:

1. **Review.** A pull request that adds `execute payments:sepa_transfer` to a node
   is conspicuous in a way that the same capability appearing inside an `impl` is
   not.
2. **Least privilege by construction.** A runtime that *can* enforce has a
   declared, per-node list to enforce against, instead of granting the whole
   workflow the union of everything any step might need.

A runtime **SHOULD** deny actions not declared. A runtime that enforces **MUST**
report a denial as a node failure with a distinguishable error class, never as a
generic failure — "permission denied on `ledger:accounts_payable`" is actionable
and "task failed" is not.

## 11.5 Classification

`classification` labels how sensitive the data flowing through a node or artifact
is.

The label **travels with the data**. A runtime **SHOULD** warn when a node
consumes an artifact classified more highly than itself, and **MUST NOT**
silently downgrade a classification.

This is not access control. It is the thing that makes access control possible
later, and it is what an auditor reads when asking whether restricted data ever
reached a node that could export it.

## 11.6 Isolation

`isolation` requests an execution boundary: `process`, `container`, `vm`, or a
runtime-defined value.

**A runtime that cannot honour a declared isolation level MUST fail rather than
downgrade** (`HX-4201`).

Silently running an untrusted step in-process because containers were
unavailable is exactly the failure the declaration was written to prevent. The
correct behaviour is to refuse and say why.

## 11.7 Credentials

Restating the rule from [chapter 9](/spec/v1.0/resources-artifacts/) because it
is the one most likely to be violated by accident:

```xml
<credential ref="ANTHROPIC_API_KEY" store="vault"/>
```

**A document containing a literal credential is invalid (`HX-3501`).**

A validator **MUST** reject a credential-shaped value in a `property` or port
`value`, and **SHOULD** warn rather than error when detection is uncertain —
detection is necessarily heuristic (high-entropy strings, known key prefixes,
private-key headers).

A runtime **MUST NOT** write a resolved credential into an execution trace, a log
or an error message, including when a node fails. A stack trace containing the
key it failed to authenticate with is a leak with a long half-life.

## 11.8 Signing

A document may carry a signature over its own content:

```xml
<metadata>
  <provenance>
    <signature algorithm="ed25519" keyId="visml-release-2026" value="…"/>
  </provenance>
</metadata>
```

The specification does not mandate a signing scheme in 1.0. What it states:

- the signature covers the **canonical form** of the document with the
  `<signature>` element itself excluded;
- a runtime configured to require signatures **MUST** refuse to execute an
  unsigned or invalid-signature document;
- a runtime **MUST NOT** treat a signature it cannot verify as absent — an
  unverifiable signature is a failure, not a missing feature.

That last point is the whole value. A signature that degrades to "no signature"
when the verifier does not recognise the algorithm provides no security at all,
while appearing to.

## 11.9 The threat this model actually addresses

Named plainly, so nobody assumes more:

**Addressed.** A workflow that silently gains capability without review. Steps
running with more privilege than they need. Sensitive data reaching an unlabelled
step. Credentials committed to version control. A modified workflow executing as
if it were the reviewed one.

**Not addressed.** A malicious `impl`. A compromised runtime. A compromised
secret store. An authorised operator who chooses to do something harmful.

HarnessXML makes intent explicit and reviewable. It cannot make an untrusted
executor safe, and does not claim to.
