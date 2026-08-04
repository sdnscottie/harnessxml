---
title: Expressions
description: The HarnessXML expression language — deliberately small, statically analysable, and side-effect free.
section: specification
order: 11
status: draft
---

# 10. Expressions

## 10.1 Scope of the language

Expressions appear in `guard/@when`, `case/@when`, `edge/@condition`,
`loop/@over`, `loop/@while`, `wait/@until` and port `@value`.

The language is **deliberately small**. It reads values, compares them, and
combines the results. It has:

- no user-defined functions
- no recursion
- no assignment, and no side effects
- no loops (loops are a node type)
- no access to the environment, filesystem or network

**Rationale.** The entire value proposition is validating a workflow *before* it
runs. Every expressiveness win that costs static analysability is a bad trade
here. When real logic is needed, that is what a `transform` node is for — it is
the honest place to put code, where it is visible in the graph rather than hidden
in an attribute.

## 10.2 Interpolation

An expression is written inside `${ }`:

```xml
<case when="${classify.confidence >= 0.90}" to="auto_file"/>
```

Where an attribute takes a value rather than a condition, text and expressions
may be mixed:

```xml
<input name="prompt" value="Classify this ${extract.pageCount}-page document"/>
```

A literal `${` is escaped as `$${`.

An expression that is not well-formed is invalid (`HX-3101`), reported at
validation time — never deferred to execution.

## 10.3 References

| form | resolves to |
|---|---|
| `node.port` | an output port of a node |
| `item`, `index` | the current loop iteration variable and position (names configurable) |
| `artifact('id')` | an artifact declaration |
| `resource('id')` | a resource declaration |
| `config.name` | a `<config>` property of the current node |

```xml
${classify.confidence}
${grasp}
${artifact('taxonomy').digest}
${config.threshold}
```

A reference to a node, port, artifact or resource that does not exist is invalid
(`HX-3103`). A reference to a node that **cannot have completed** before this
expression is evaluated is invalid (`HX-3104`) — reading a value from a node that
has not run is a design error the validator can catch statically, and it is a
common one.

## 10.4 Operators

| category | operators |
|---|---|
| comparison | `==` `!=` `<` `<=` `>` `>=` |
| logical | `and` `or` `not` (and `&&` `\|\|` `!`) |
| arithmetic | `+` `-` `*` `/` `%` |
| membership | `in` |
| null | `??` (coalesce) |
| grouping | `( )` |

Precedence follows the conventional order: grouping, unary, arithmetic,
comparison, `and`, `or`.

> In XML, `<` and `&` must be escaped as `&lt;` and `&amp;` inside attribute
> values. The word forms `and`, `or`, `not` are provided because
> `${a &amp;&amp; b}` is unreadable, and normative text should not require a
> reader to decode entities to see what a threshold is.

```xml
<case when="${classify.confidence &gt;= 0.9 and not review.forced}" to="auto"/>
<case when="${status in ['approved','auto_approved']}" to="pay"/>
<input name="limit" value="${config.limit ?? 100}"/>
```

## 10.5 Literals and types

| type | literals |
|---|---|
| string | `'text'` or `"text"` |
| number | `42`, `3.14`, `-1` |
| boolean | `true`, `false` |
| null | `null` |
| array | `[1, 2, 3]` |

Values are compared **without type coercion**. `'1' == 1` is `false`. Coercion
between a string and a number is the classic source of a threshold silently never
matching — and in this format that threshold might be an approval limit.

Comparison of incompatible types is a runtime error (`HX-4106`), not a `false`
result. A comparison that cannot be meaningfully performed is a defect, and
returning `false` would hide it behind a branch that simply never fires.

## 10.6 Built-in functions

A minimal set, all pure:

| function | returns |
|---|---|
| `len(x)` | length of a string, array or map |
| `has(x, k)` | whether a map or object has key `k` |
| `empty(x)` | whether a string, array or map is empty |
| `lower(s)`, `upper(s)` | case conversion |
| `contains(s, t)` | substring or element containment |
| `abs(n)`, `min(a,b)`, `max(a,b)`, `round(n)` | arithmetic |
| `artifact(id)`, `resource(id)` | declaration lookup |

The set is **closed** in 1.0. An unknown function is invalid (`HX-3105`). A
runtime **MUST NOT** offer extra functions in the core namespace — a document
that runs on one runtime and fails validation on another is exactly the
interoperability failure this specification exists to prevent.

Deliberately absent: anything reading wall-clock time, random numbers, the
environment, or the filesystem. All of them make an expression's value depend on
something outside the document, which makes the workflow irreproducible and its
validation meaningless.

## 10.7 Null handling

An unresolved reference evaluates to `null` rather than raising, so `null` checks
work:

```xml
<guard when="${extract.poNumber != null}"/>
```

But:

- `null` in an arithmetic or comparison operator other than `==` / `!=` is a
  runtime error (`HX-4106`);
- a `guard`, `case` or `condition` whose expression evaluates to `null` rather
  than a boolean is a runtime error (`HX-4107`).

The second rule matters. Treating `null` as `false` would silently take the
`otherwise` branch whenever an upstream value was missing — which looks exactly
like a deliberate routing decision in every log, and is not one.

## 10.8 Evaluation timing

| expression | evaluated |
|---|---|
| `guard/@when` | when the node becomes `READY`, before inputs resolve |
| `case/@when` | when the decision node executes, **in document order** |
| `edge/@condition` | when the source node reaches a terminal state |
| `loop/@over` | once, when the loop node executes |
| `loop/@while` | before each iteration (`while`) or after each (`until`) |
| port `@value` | when the owning node's inputs are resolved |

Expressions are **pure**, so a runtime MAY evaluate one more than once or cache
it. Any observable difference between those two choices is a defect in the
runtime, not a decision the document gets to make.
