---
title: Licensing
description: HarnessXML's specification text is CC BY 4.0 and its code is Apache 2.0. What that permits, what it requires, and why trademarks are deliberately excluded.
section: project
order: 5
status: stable
---

# Licensing

A permissive split licence, published from the first day rather than added once
adoption made it awkward not to.

| what | licence | SPDX |
|---|---|---|
| Specification text, this website | Creative Commons Attribution 4.0 | `CC-BY-4.0` |
| Schemas, examples, reference implementation, conformance suite, SDKs | Apache License 2.0 | `Apache-2.0` |
| "HarnessXML", "VisML", "RuMima" | **not licensed** — trademarks | — |

## What you may do

Without asking, without paying, without telling anyone:

- **Implement HarnessXML.** Build an editor, parser, validator, runtime,
  compiler, importer, exporter or SDK. Sell it. Compete with RuMima.
- **Copy and redistribute the specification**, in whole or in part, in any medium.
- **Translate it.** Translations are explicitly welcome, and a good translation is
  a contribution to the specification's reach, not a derivative to be tolerated.
- **Quote it** in your own documentation, papers, RFCs or standards.
- **Build a derived specification or a profile** on top of it.
- **Use the reference code** in a commercial product, modified or not.

There is no implementer agreement, no royalty, no field-of-use restriction, and
no notification requirement.

## What you must do

**Attribute the specification text.** CC BY 4.0's one condition. Something like:

> HarnessXML Specification, created and stewarded by VisML, used under CC BY 4.0.
> <https://harnessxml.com/>

Indicate if you changed it. If you publish a modified or derived specification,
say so plainly and do not imply VisML endorses it.

**Keep the Apache-2.0 notices** on code you redistribute — the licence text, the
copyright notice, and a statement of significant changes. Standard Apache
obligations, nothing unusual.

Attribution in *code* is requested but not required beyond what Apache-2.0 says.
Attribution in *specification text* is required.

## Why these two, specifically

**CC BY 4.0 for the text.** A specification only becomes a standard if a
competitor can implement it without permission. CC BY is the weakest condition
under which authorship still has to be acknowledged: commercial use, derivatives
and redistribution are all permitted, and credit is the price. It is the posture
of the specifications HarnessXML means to stand alongside.

**Apache 2.0 for the code, rather than MIT.** Two reasons, and the first is the
real one:

1. **The express patent grant (section 3).** A specification that enterprises are
   asked to implement must not leave a patent question open. MIT is silent on
   patents; Apache-2.0 grants them explicitly and terminates the grant for anyone
   who sues over patents in the work. Enterprise legal review notices this.
2. **Familiarity.** Kubernetes, Terraform, OpenTelemetry. Apache-2.0 reads as
   normal and passes review without a conversation.

Note the asymmetry: **the CC BY licence on the specification text grants no
patent rights.** The patent grant that matters attaches to the reference
implementation under Apache-2.0.

## Why trademarks are excluded

Neither licence grants trademark rights, and that is deliberate rather than an
oversight.

Trademarks are the only lever left once everything else is given away. They are
what stops a divergent fork from calling itself HarnessXML, and what makes a
conformance claim mean something.

- You **may** say your software "implements HarnessXML" or "supports HarnessXML".
- You **may not** name or brand your product so as to imply it is the official
  HarnessXML implementation, or that VisML produced or endorsed it.
- Using the HarnessXML **conformance mark** requires published, reproducible
  results from a tagged release of the [conformance suite](/conformance/).

A false conformance claim is a trademark matter. That is the enforcement
mechanism, and it exists precisely so that everything else could be permissive.

## Contributions

By contributing you agree your contribution is licensed under CC BY 4.0 (text) or
Apache 2.0 (code), and that you have the right to contribute it. **There is no
CLA.** The Apache-2.0 inbound patent grant in section 5 covers what a CLA would
otherwise be needed for.

If your employer owns your work, get their permission first — that is the one
thing this project cannot check for you.

## Files

- [`LICENSE-SPEC`](https://github.com/sdnscottie/harnessxml/blob/main/LICENSE-SPEC) — CC BY 4.0 terms and scope
- [`LICENSE-CODE`](https://github.com/sdnscottie/harnessxml/blob/main/LICENSE-CODE) — full Apache 2.0 text
- [Governance](/governance/) — the commitments that surround the licences
