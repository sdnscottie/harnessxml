---
title: Conformance
description: The three HarnessXML conformance levels, how the test suite is structured, and how to publish a conformance claim.
section: implementing
order: 10
status: draft
---

# Conformance

Conformance is defined by a **published test suite**, not by agreement with any
implementation — including VisML's. If the reference runtime disagrees with the
suite, the reference runtime has a bug.

## The three levels

| level | an implementation must |
|---|---|
| **Core** | parse and validate — accept every valid fixture, and reject every invalid one **with the specified `HX-nnnn` code** |
| **Executing** | everything in Core, plus the execution model: edge-type scheduling, the node lifecycle, deterministic decisions, bounded loops, retries and timeouts |
| **Full** | everything in Executing, plus resources, artifacts, provenance, the security model, and compensation |

Levels are cumulative and are the only conformance claims recognised. A partial
claim ("Core plus loops") is not a level, because the value of a level is that it
tells a reader exactly what they can rely on.

## Why the error code is part of the test

Rejecting an invalid document is not enough. Two validators that both reject a
dangling edge but disagree about *why* will give their users incompatible
diagnostics, and a workflow author cannot move between tools.

So an invalid fixture asserts the code:

```
conformance/invalid/edge-dangling-target.hxml      -> HX-2001
conformance/invalid/loop-unbounded.hxml            -> HX-1001
conformance/invalid/retry-on-non-idempotent.hxml   -> HX-3301
conformance/invalid/credential-literal.hxml        -> HX-3501
```

An implementation that rejects the right documents for the wrong reasons is not
Core-conformant.

## Fixture layout

```
conformance/
├── validate.py             runs the suite against a validator command
├── valid/                  documents that MUST be accepted
├── invalid/                documents that MUST be rejected
│   └── <case>.expected     the required HX-nnnn code
└── traces/                 execution traces that MUST match  (Executing, Full)
    ├── <case>.hxml
    └── <case>.trace.json   normalised node state transitions
```

Traces are compared on the **normalised sequence of node state transitions**, not
on timing or on the ordering of independent branches — two runtimes are allowed
to schedule unrelated work in different orders. What they may not do is disagree
about whether a node ran, was skipped, retried, failed or was compensated.

## Running the suite

```bash
git clone https://github.com/sdnscottie/harnessxml
cd harnessxml
python3 conformance/validate.py                       # reference validator
python3 conformance/validate.py --cmd "my-validator"  # your implementation
```

The runner is standard-library Python and shells out to a validator command, so
it works against an implementation written in any language.

## Publishing a claim

**Self-certification is the default.** VisML does not gatekeep who may implement
HarnessXML or who may say they have.

To publish a claim:

1. Run the suite against a **tagged release** of your implementation.
2. Publish the full output, including any failures.
3. State the level, the suite version, and the implementation version.

Something like:

> `acme-harness 2.1.0` — HarnessXML **Executing** conformance,
> suite `v1.0-rc3`, results: `https://acme.example/conformance/2.1.0.txt`

## The conformance mark

The wordmark is the one thing not open-licensed, and this is why. Using the
HarnessXML conformance mark requires published, reproducible results for a tagged
release at the level claimed.

A false conformance claim is a trademark matter — the only enforcement lever the
[licences](/licensing/) leave, and the reason everything else could be
permissive. It exists to protect implementers from a competitor claiming
compatibility it does not have, not to control who may implement.

## Current state

> **The suite is incomplete.** The fixture format is settled and the runner
> works, but the corpus does not yet cover every normative rule. Completing that
> coverage is a release gate for v1.0 final — a rule the suite cannot check is a
> rule implementers will diverge on. See the [roadmap](/roadmap/).
