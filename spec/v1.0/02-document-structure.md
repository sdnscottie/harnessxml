---
title: Document Structure
description: The HarnessXML document — root element, section order, identifiers, and the namespace rules.
section: specification
order: 3
status: draft
---

# 2. Document Structure

## 2.1 The root element

A HarnessXML document has exactly one root element, `<harness>`, in the namespace
`https://harnessxml.com/spec/1.0`.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<harness xmlns="https://harnessxml.com/spec/1.0"
         id="document_triage"
         specVersion="1.0"
         name="Document triage"
         entry="receive">
  ...
</harness>
```

| attribute | required | meaning |
|---|---|---|
| `id` | **yes** | document-unique identifier for this harness |
| `specVersion` | **yes** | the specification version the document is written against |
| `name` | no | human-readable title; carries no semantics |
| `entry` | no | explicit entry node; see §2.5 |

A document **MUST** be UTF-8 encoded. A document **MUST** declare `specVersion`
(`HX-1002`) — a runtime cannot safely guess which semantics apply, and guessing
wrong is worse than refusing.

## 2.2 Section order

The child elements of `<harness>` appear in this order, and the order is
enforced by the schema:

```xml
<harness …>
  <metadata/>    <!-- optional, at most one -->
  <security/>    <!-- optional, at most one -->
  <resources/>   <!-- optional, at most one -->
  <artifacts/>   <!-- optional, at most one -->
  <nodes/>       <!-- REQUIRED, exactly one, at least one <node> -->
  <edges/>       <!-- optional, at most one -->
  <extension/>*  <!-- zero or more -->
</harness>
```

Fixed order rather than free order is a deliberate readability choice. Every
HarnessXML document a reviewer opens has declarations before graph, and graph
before wiring — so a diff always lands in a predictable place.

## 2.3 Identifiers

Identifiers — `@id` on harness, node, edge, resource and artifact, and port
`@name` — match:

```
[A-Za-z_][A-Za-z0-9_.\-]*
```

with a maximum length of 255 characters.

Constrained deliberately so an identifier is usable **unmodified** as an
identifier in every language an SDK might generate, and never needs escaping
inside an expression.

Identifiers are **case-sensitive**. `classify` and `Classify` are different
nodes; authoring tools SHOULD warn about identifiers differing only by case,
because a reader will not see the difference.

Uniqueness is scoped:

| identifier | must be unique among |
|---|---|
| node `@id` | all nodes in the harness |
| edge `@id` | all edges in the harness |
| resource `@id` | all resources |
| artifact `@id` | all artifacts |
| port `@name` | ports of the same direction on the same node |

A duplicate is invalid (`HX-1101`).

Node, resource and artifact identifiers occupy **separate** spaces — a node and a
resource may both be called `classifier`. References are unambiguous because each
reference site names exactly one kind: `resourceRef/@ref` can only mean a
resource.

## 2.4 References

Every cross-reference names an identifier declared elsewhere in the same document:

| from | to |
|---|---|
| `edge/@from`, `edge/@to` | node `@id` |
| `case/@to`, `otherwise/@to` | node `@id` |
| `loop/body/@ref` | node `@id` |
| `resourceRef/@ref` | resource `@id` |
| `artifactRef/@ref` | artifact `@id` |
| `node/@compensates` | node `@id` |

The first five are enforced **by the schema**, via `xs:key` / `xs:keyref`. A
dangling edge is therefore rejected by any plain schema-validating parser in any
language, before HarnessXML-aware tooling runs.

> `@compensates` is checked in the text layer rather than the schema (`HX-2004`),
> because its target is constrained by more than existence — see
> [chapter 8](/spec/v1.0/failure/).

## 2.5 Entry points

If `@entry` is present, it **MUST** name a node in the harness, and that node is
the sole entry point.

If `@entry` is absent, the entry set is **every node with no incoming edge of any
type**.

The earlier wording of this rule said "no incoming `control`, `data` or
`dependency` edge", which contradicted its own next sentence: an error handler
has *only* an incoming `error` edge, so it satisfied the rule and started as if
it were an entry point. The reference implementation ran a failure handler on a
workflow where nothing had failed, which is how the contradiction was found.

**A node reachable only by an `error` or `compensation` edge is a handler, not a
start**, and does not become ready until something has actually failed.

A harness whose entry set is empty is invalid (`HX-3001`). An empty entry set
means every node waits for another, which is a cycle: nothing can ever begin.

## 2.6 Namespace handling

The namespace is fixed per **major** version. Minor versions do not change it.

```xml
xmlns="https://harnessxml.com/spec/1.0"     <!-- 1.0, 1.1, 1.2, … -->
xmlns="https://harnessxml.com/spec/2.0"     <!-- 2.0, 2.1, … -->
```

Prefixed or default form are both acceptable; an implementation **MUST** match on
namespace URI and local name, never on the prefix:

```xml
<hx:harness xmlns:hx="https://harnessxml.com/spec/1.0" …>
```

Elements from **other** namespaces are permitted only inside `<extension>`. An
unrecognised element in the HarnessXML namespace is invalid (`HX-1003`) — see
§2.8.

## 2.7 Schema association

Associating the schema is OPTIONAL and does not affect validity, but it gives
editors completion and inline checking:

```xml
<harness xmlns="https://harnessxml.com/spec/1.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="https://harnessxml.com/spec/1.0
                             https://harnessxml.com/schema/v1.0/harnessxml-1.0.xsd"
         id="w" specVersion="1.0">
```

A validator **MUST NOT** fetch a schema over the network in order to validate. It
**MUST** use a local copy of the schema for the declared `specVersion`. A
validator whose verdict depends on network reachability is not reproducible, and
a workflow's validity must not change because a CDN had a bad day.

## 2.8 Unknown constructs

**A conforming implementation encountering an element or attribute in the
HarnessXML namespace that it does not recognise MUST reject the document
(`HX-1003`). It MUST NOT ignore it.**

This is the most important rule in the specification.

The tolerant alternative — skip what you do not understand — means a run
**reports success while having done less than it was told**. The skipped node
could have been the approval gate, the safety interlock or the rollback. A green
result that omitted the safety check is worse than a red one.

The cost is accepted deliberately: a 1.0 runtime cannot execute a 1.2 document
that uses a 1.2 node type. That is the correct outcome, and it should say so.

Vendor extension has its own mechanism precisely so that this rule can stay
absolute:

```xml
<node id="grasp" type="task">
  <extension namespace="https://acme.example/harness/1" required="false">
    <acme:forceLimit xmlns:acme="https://acme.example/harness/1">12.5</acme:forceLimit>
  </extension>
</node>
```

- `required="false"` — a runtime that does not understand it **MUST** ignore the
  extension and **MUST** still execute the node.
- `required="true"` — a runtime that does not understand it **MUST** refuse to
  execute the document (`HX-1004`).

`required="true"` is how a vendor says "without this, the workflow is wrong"
instead of producing subtly different behaviour without telling anyone.

## 2.9 File extension and media type

A HarnessXML document **SHOULD** use the file extension:

```
.hxml
```

and **SHOULD** be served with the media type:

```
application/harnessxml+xml
```

Neither affects validity — a conforming implementation **MUST** determine that a
document is HarnessXML from its root element and namespace, never from its
filename or from a `Content-Type` header. A workflow does not stop being valid
because someone renamed the file.

The extension exists so that editors, diff tools and operators can recognise a
workflow at a glance. `application/xml` and `text/xml` are acceptable fallbacks
where a server cannot be configured.

### 2.9.1 `.hxml` and `.visml` — embedding, not subsetting

VisML uses one vendor extension across its products, and this specification
defines a second, separate one. The relationship between them is **containment**,
and the direction matters more than anything else in this section.

| extension | format | what it is | open? |
|---|---|---|---|
| **`.hxml`** | **HarnessXML** | the specification defined by this document — executable workflows | **open, vendor-neutral** |
| `.visml` | VisML Markup Language | the shared native format of VisML's products, including Rumima | vendor format |

**Only `.hxml` is specified here, and only `.hxml` is an interchange format.**

#### A `.visml` document may embed a HarnessXML document

A VisML document carries what HarnessXML deliberately excludes — canvas layout,
colours, grouping, editor state, work that does not yet execute — and embeds the
executable graph as a child element **in the HarnessXML namespace**:

```xml
<visml xmlns="https://visml.com/schema/1.0" product="rumima">
  <rumima-document version="1">
    <canvas><!-- layout, colours, grouping: presentation, not semantics --></canvas>

    <harness xmlns="https://harnessxml.com/spec/1.0"
             id="document_triage" specVersion="1.0">
      <!-- a complete, independently valid HarnessXML document -->
    </harness>
  </rumima-document>
</visml>
```

Export is then **lifting the element out**, and import is wrapping it. Nothing is
translated, so nothing can be lost in translation.

#### The embedded element MUST be a complete HarnessXML document

Not a fragment, not a profile, not a dialect. Serialised on its own it is
byte-for-byte a valid `.hxml` document that validates against
[`harnessxml-1.0.xsd`](/schema/v1.0/harnessxml-1.0.xsd).

#### The dependency runs one way only

**This is the rule that keeps HarnessXML open, and it is normative:**

> HarnessXML **MUST** be fully definable, validatable and executable without
> reference to `.visml` or to any other host format. A conforming implementation
> **MUST NOT** be required to understand a host format in order to process an
> embedded HarnessXML document.

`.visml` may depend on HarnessXML. HarnessXML must never depend on `.visml`.

HarnessXML is therefore **not a subset of**, **not a profile of**, and **not an
extension of** the VisML markup language. It is an independent specification that
a VisML document happens to contain — exactly as an HTML page may contain an SVG
document without SVG becoming a subset of HTML.

If that direction were reversed — if HarnessXML were *defined as* a restricted
profile of a vendor format — then implementing it would require understanding
that vendor's format first, the normative namespace would belong to that vendor,
and "open and vendor-neutral" would be a claim the format's own definition
contradicted. The embedding relationship gives VisML everything it wants from
integration and costs the specification nothing.

#### The same applies to any host

Nothing here is specific to VisML. Any editor, document format or repository may
embed a HarnessXML document by the same rule: include a complete `<harness>` in
the HarnessXML namespace, and lift it out unchanged to export. A conforming
implementation is never expected to read `.visml` — if interoperating required a
vendor's native format, the open format would have failed at its only job.

> **Note on the extension.** `.visml` was chosen over the shorter `.vxml` because
> `.vxml` is already in common use for [VoiceXML](https://www.w3.org/TR/voicexml21/),
> a W3C standard for voice dialogue applications. Since a single extension now
> carries VisML's whole product surface, the cost of that collision — operating
> systems and editors with the association already claimed — would have been paid
> everywhere rather than in one place. `.hxml` and `.visml` have no comparable
> prior use.

## 2.10 A minimal document

The smallest valid harness — one node, no edges:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<harness xmlns="https://harnessxml.com/spec/1.0"
         id="minimal" specVersion="1.0">
  <nodes>
    <node id="only" type="task" impl="noop"/>
  </nodes>
</harness>
```

`<metadata>`, `<resources>`, `<artifacts>` and `<edges>` are all optional. What
cannot be omitted is `id`, `specVersion`, and at least one node.
