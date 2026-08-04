---
title: Validation Rules
description: Every normative HarnessXML validation rule, by error code — the two validation layers, and what a conforming validator must reject.
section: specification
order: 14
status: draft
---

# 13. Validation Rules

## 13.1 Two layers

**Layer 1 — the XML Schema.** Structure, element order, cardinality, datatypes,
enumerations, and referential integrity between edges, nodes, resources and
artifacts (expressed as `xs:key` / `xs:keyref`). Any plain schema-validating
parser in any language enforces this, with no HarnessXML-aware tooling.

**Layer 2 — this chapter.** Everything XSD 1.0 cannot express: graph properties,
expression analysis, and policy contradictions.

A document that passes layer 1 is **not** necessarily valid HarnessXML. Both
layers apply, and a conforming validator implements both.

**A runtime MUST NOT execute an invalid document.** Validation is a gate, not
advice.

## 13.2 Rule catalogue

Codes are grouped: `1xxx` document and structure, `2xxx` references and
interfaces, `3xxx` semantics, `4xxx` runtime
([chapter 14](/spec/v1.0/errors/)). Only `1xxx`–`3xxx` are validation-time.

### 13.2.1 Document and structure — `HX-1xxx`

| code | rule |
|---|---|
| `HX-1001` | The document MUST satisfy the XML Schema for its declared `specVersion`. |
| `HX-1002` | `specVersion` MUST be present on `<harness>`. |
| `HX-1003` | An element or attribute in the HarnessXML namespace that the implementation does not recognise MUST cause rejection. It MUST NOT be ignored. |
| `HX-1004` | An `<extension required="true">` whose namespace the implementation does not understand MUST cause rejection. |
| `HX-1005` | The document MUST be UTF-8 encoded. |
| `HX-1006` | An element from a foreign namespace MUST NOT appear outside `<extension>`. |
| `HX-1101` | Identifiers MUST be unique within their scope: node, edge, resource and artifact ids; port names per node per direction. |
| `HX-1102` | `<nodes>` MUST contain at least one `<node>`. |

### 13.2.2 References and interfaces — `HX-2xxx`

| code | rule |
|---|---|
| `HX-2001` | `edge/@from` and `edge/@to` MUST name declared nodes. *(schema-enforced)* |
| `HX-2002` | `resourceRef/@ref` MUST name a declared resource. *(schema-enforced)* |
| `HX-2003` | `artifactRef/@ref` MUST name a declared artifact. *(schema-enforced)* |
| `HX-2004` | If a compensation-edge target declares `@compensates`, it MUST name the edge's source node. |
| `HX-2005` | A compensation-edge target MUST NOT be reachable by forward `control` or `data` edges. |
| `HX-2101` | Every `required` input MUST be satisfied by an incoming data edge or a `@value`. |
| `HX-2102` | An input MUST NOT be satisfied by both a data edge and a `@value`. |
| `HX-2201` | `<cases>` MUST appear on `type="decision"` and MUST NOT appear elsewhere. |
| `HX-2202` | `<loop>` MUST appear on `type="loop"` and MUST NOT appear elsewhere. |
| `HX-2203` | `<subworkflow>` MUST appear on `type="subworkflow"` and MUST NOT appear elsewhere. |
| `HX-2204` | `<wait>` MUST appear on `type="wait"` and MUST NOT appear elsewhere. |
| `HX-2205` | `<wait>` MUST declare exactly one of `@duration`, `@until` or `@event`. |
| `HX-2206` | `<cases>` MUST contain at least one `<case>`. |
| `HX-2207` | A loop MUST declare the attribute its `@kind` requires — `@over` for `forEach`, `@while` for `while` and `until`, `@count` for `times`. |
| `HX-2208` | For `kind="times"`, `@count` MUST NOT exceed `@maxIterations`. |
| `HX-2301` | A `data` edge MUST declare both `@fromPort` and `@toPort`. |
| `HX-2302` | `@fromPort` MUST name an output on the source node. |
| `HX-2303` | `@toPort` MUST name an input on the target node. |
| `HX-2304` | At most one `data` edge MUST target a given input. |
| `HX-2401` | `@quorum` MUST be present when `joinPolicy="quorum"`. |
| `HX-2402` | `@quorum` MUST NOT exceed the number of incoming edges. |
| `HX-2501` | `type="inference"` MUST reference at least one resource of type `model`. |

### 13.2.3 Semantics — `HX-3xxx`

| code | rule |
|---|---|
| `HX-3001` | The entry set MUST NOT be empty. |
| `HX-3002` | Subworkflow references MUST NOT be recursive, directly or indirectly. |
| `HX-3003` | The graph formed by `control`, `data` and `dependency` edges MUST be acyclic. `error` and `compensation` edges are excluded. |
| `HX-3004` | A loop body node MUST NOT be the target of an incoming `control` or `dependency` edge from outside its loop. An incoming `data` edge **is** permitted — that is how a loop-invariant input is bound. |
| `HX-3005` | Every node SHOULD be reachable from the entry set along `control`, `data`, `dependency` or `error` edges. An unreachable node is a **warning**, not an error. |
| `HX-3101` | Every expression MUST be well-formed. |
| `HX-3102` | A loop iteration variable MUST NOT be referenced outside its loop body. |
| `HX-3103` | An expression MUST NOT reference an undeclared node, port, artifact or resource. |
| `HX-3104` | An expression MUST NOT reference a node that cannot have completed before the expression is evaluated. |
| `HX-3105` | An expression MUST NOT call an unknown function. |
| `HX-3201` | Where both ports of a `data` edge declare a `@type`, the types MUST be compatible. |
| `HX-3301` | A node declared `idempotent="false"` MUST NOT declare a `<retry>` policy. |
| `HX-3401` | An ISO 8601 duration MUST NOT use months or years. |
| `HX-3501` | A document MUST NOT contain a literal credential. |

## 13.3 Errors and warnings

Most rules are **errors**: the document is invalid and MUST be rejected.

A few are **warnings**: the document is valid, and a validator SHOULD report the
condition.

| warning | why it is not an error |
|---|---|
| `HX-3005` unreachable node | legitimate during authoring, and legitimate for a node reached only by a not-yet-wired branch |
| redundant `control` alongside `data` between the same pair | harmless, but misleads a reader into seeing two relationships |
| guarded producer feeding a required consumer | statically valid; fails at runtime with `HX-4101` if the guard is false |
| uncertain credential-shaped value | detection is heuristic; a false positive that blocks a build is worse than a warning |

A validator **MUST** distinguish the two in its output and **MUST NOT** exit
non-zero on warnings alone. A build that fails on advisory findings trains people
to pass `--no-warnings`, which loses the errors too.

## 13.4 Validation is offline

A validator **MUST NOT** require network access.

- Schemas MUST be resolved from a local copy for the declared `specVersion`,
  never fetched ([§2.7](/spec/v1.0/document-structure/#2-7-schema-association)).
- A `subworkflow/@href` MUST NOT be dereferenced during validation of the parent.
  `HX-3002` is checked over references the validator has been given, not over
  what it can reach.
- Artifact URIs MUST NOT be dereferenced.

A verdict that depends on network reachability is not reproducible, and a
workflow's validity must not change because a CDN had a bad day.

## 13.5 Diagnostics

A conforming validator MUST report, for every finding:

1. the **code** — `HX-nnnn`;
2. the **location** — line and column, and the id of the nearest identified
   element;
3. a **message** naming the specific offending value.

```
workflow.hxml:47:5  HX-2303  edge 'e_hr_cat': toPort 'suggestedCategry' is not
                             an input on node 'human_review'
                             (did you mean 'suggestedCategory'?)
```

The code is part of conformance. Two validators that reject the same document for
differently-stated reasons give their users incompatible diagnostics, and an
author cannot then move between tools — which is precisely the interoperability
failure a specification exists to prevent.

A validator **SHOULD** report **all** findings, not stop at the first. Fixing one
error at a time through five build cycles is a bad experience that implementations
have no reason to inflict.
