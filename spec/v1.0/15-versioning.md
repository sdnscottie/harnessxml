---
title: Versioning and Compatibility
description: What may change between HarnessXML versions, what may never change, and how a runtime handles a document from a different version.
section: specification
order: 16
status: draft
---

# 15. Versioning and Compatibility

## 15.1 Version numbers

Specification versions are `MAJOR.MINOR`. There is no patch component — a
released version is immutable, so there is nothing for a patch number to
describe. Corrections are dated errata.

| change | version |
|---|---|
| additive only | MINOR — `1.0 → 1.1` |
| removal or redefinition | MAJOR — `1.x → 2.0` |
| typo or clarification with no behavioural change | erratum, appended to the released version |

## 15.2 Immutability

**A released version is frozen at a permanent URL, forever.**

```
https://harnessxml.com/spec/v1.0/
https://harnessxml.com/schema/v1.0/harnessxml-1.0.xsd
```

Released text is never edited in place — not for typos. A document that validated
against v1.0 in 2026 validates against the v1.0 served in 2036, byte for byte.

This is what makes the specification safe to cite in a contract, a certification
or an audit finding. A specification that can be edited after release is one
where "we complied with v1.0" is not a checkable statement.

Errata are published as a dated, appended list on the version's own page.

## 15.3 What a minor version may do

**May:**

- add optional elements and attributes
- add node types, edge types, error codes and error classes
- add enumeration values to attributes documented as open-ended
- add built-in functions
- deprecate a construct — marking it, warning on it, and **keeping it working**

**Must never:**

- remove an element, attribute or enumeration value
- narrow the value space of an existing attribute
- change the runtime meaning of an existing construct
- make an optional attribute required
- change a default value

Changing a default is listed under "never" deliberately, because it is the one
that looks harmless. A workflow that relied on a default silently changes
behaviour when the default moves, with no diff anywhere to show why.

## 15.4 Namespaces

The namespace changes only at a **major** version.

```
https://harnessxml.com/spec/1.0     used by 1.0, 1.1, 1.2, …
https://harnessxml.com/spec/2.0     used by 2.0, 2.1, …
```

A namespace change breaks every existing document, which is exactly what a minor
version promises not to do. `specVersion` carries the minor version; the
namespace carries the major.

## 15.5 How a runtime handles another version

Given a document declaring `specVersion="X.Y"`, a runtime implementing `X.Z`:

| case | behaviour |
|---|---|
| same major, `Y <= Z` | execute normally |
| same major, `Y > Z` | attempt it — **but** `HX-1003` on any construct not recognised |
| different major | **MUST** refuse (`HX-1002`) |

The middle row is where forward compatibility lives, and it works precisely
because of the fail-loudly rule: a 1.0 runtime running a 1.2 document succeeds if
that document happens to use no 1.2 construct, and refuses the moment it meets
one. It never runs a partial version of the workflow.

A runtime **MUST NOT** attempt a document from a different major version. Major
means constructs may have been redefined, so a familiar-looking element may mean
something else.

## 15.6 Deprecation

A construct may be deprecated in a minor version. Deprecation:

- marks it in the specification with the version and the replacement;
- makes conforming validators emit a **warning**;
- **keeps it working**, unchanged.

**A deprecated construct runs for at least one full major version.** Deprecated
in 1.3 means working through every 1.x, removable only in 2.0.

## 15.7 Major versions

A major version may remove or redefine. The bar for shipping one:

1. a **migration guide** covering every breaking change;
2. a **mechanical migration tool** in the reference implementation that converts
   a valid `1.x` document to `2.0` — or reports precisely what it cannot convert
   and why;
3. the previous major version stays served, forever, at its own URLs.

Without the tool, the major version does not ship. Requiring users to hand-migrate
is how a format loses the documents that had already been written in it.

## 15.8 Document versions are not spec versions

```xml
<harness specVersion="1.0">          <!-- version of HarnessXML -->
  <metadata>
    <documentVersion>3</documentVersion>   <!-- version of THIS workflow -->
  </metadata>
```

`specVersion` is which HarnessXML this document is written against.
`documentVersion` is the author's own revision of the workflow, opaque to the
specification and to every runtime.

They are separate attributes rather than one overloaded field because they change
for entirely unrelated reasons, and conflating them makes it impossible to answer
either "which spec does this need" or "which revision of our process ran".

## 15.9 The extension escape hatch

Extensions ([§2.8](/spec/v1.0/document-structure/#2-8-unknown-constructs)) are
what make these guarantees affordable.

A vendor that needs a capability the current version lacks does not have to wait
for a minor version, and does not have to fork. It ships a namespaced extension,
declaring whether it is `required`. If the capability proves broadly useful, that
is the normal route to an HXEP for core inclusion — with real implementation
experience behind it rather than a proposal argued in the abstract.

Without that escape hatch, the pressure to break compatibility would come from
vendors who had no other option, and the guarantees above would not survive
contact with commercial reality.
