---
title: Why HarnessXML
description: Why an open specification for executable workflows is needed, what already exists, and where each existing option stops short.
section: introduction
order: 4
status: stable
---

# Why HarnessXML

## The problem, concretely

Take a workflow every team building on models has written some version of:

> Receive a document. Extract its text. Classify it with a model. If the model is
> confident, file it. If it is not, send it to a human. If the model call fails,
> retry with backoff — but not forever, and not if the failure was a bad request.
> Page someone if it still fails. Record what happened for audit.

Nothing there is exotic. Now ask four questions about your implementation of it:

1. **Where is the confidence threshold?** Not "which service" — which *line*.
   And who can change it without a deploy?
2. **What is the retry policy on the model call?** Is it the same as the one on
   the database write two steps later? Should it be?
3. **If the human review step times out after three days, what happens?** Is that
   behaviour written down, or is it whatever the queue library does by default?
4. **Can you show an auditor that a specific document went down the human path,
   and what the design was on the day it did?**

On most teams the answers are: scattered, no and no, nobody has checked, and not
without archaeology. Not because the team is careless — because the workflow is
not an object anywhere. It is an emergent property of several files.

## What already exists, and where each stops

The space is not empty. It is full of tools that solve *part* of this and are
excellent at what they do.

| | what it does well | where it stops for this problem |
|---|---|---|
| **Code-first orchestrators** (Airflow, Prefect, Temporal, Dagster) | Expressive, testable, real programming languages, mature schedulers | The workflow *is* the code, so it is only portable to that runtime, only readable by people who read that language, and the diagram is generated documentation that drifts |
| **BPMN** | A genuine open standard with real vendor diversity, decades of tooling | Designed for human business processes. No first-class model resources, artifacts with digests, idempotence declarations or bounded-loop safety. Extending it into ML and robotics means fighting its assumptions |
| **CI/CD workflow YAML** (GitHub Actions, GitLab CI) | Ubiquitous, easy to read, genuinely declarative | Deliberately vendor-specific, and scoped to build pipelines. No compensation, no typed data edges, no execution semantics you could implement independently |
| **Agent frameworks** (LangGraph, CrewAI, and the rest) | Fast to build with, close to how the field actually thinks right now | Framework-shaped, moving quickly by design, and the graph is a Python object — not an artifact you can sign, diff, archive or hand to another vendor |
| **GraphML / DOT** | Genuinely open, portable, excellent for graph structure | Describe *a graph*, not an *executable* one. No execution semantics at all — no retries, no conditions, no lifecycle |

Read down that last column and the same shape appears in every row. Either the
workflow is executable but not portable, or portable but not executable.

**HarnessXML is aimed at exactly that intersection: executable *and* portable,
specified precisely enough that independent implementations agree.**

## Why a specification rather than a better tool

A tool solves the problem for its users. A specification solves it for people who
will never use your tool — which is the only way it stays solved after your tool
is gone.

This matters more than usual here, because of how fast this field moves. The
model providers, the frameworks and the deployment shapes have all turned over
repeatedly in the last few years, and will again. A workflow captured in this
year's framework is captured in something with a short half-life.

The parts that *don't* turn over are the shape of the workflow and the semantics
of executing it: this step depends on that one, this one may be retried and that
one absolutely may not, this branch is taken when confidence is low, this failure
compensates by undoing that write. Those have been stable for decades and will
outlive every framework currently in use.

HarnessXML tries to be precise about the durable part, and deliberately silent
about the volatile part.

## Why XML

The honest answer, because it is a fair question and JSON or YAML would be the
default choice today.

**Schema validation that is genuinely normative.** XSD lets structural rules —
including referential integrity between edges, nodes, resources and artifacts —
be enforced by a plain schema-validating parser in any language, before any
HarnessXML-aware tool runs. JSON Schema is capable but weaker at cross-references;
YAML has no comparable standard in practice.

**Namespaced extension.** Vendors must be able to add capabilities without
forking the core, and readers must be able to tell instantly what is core and
what is vendor. XML namespaces do this natively and unambiguously. In JSON it is
a naming convention everyone implements differently.

**Signing and canonicalisation.** A workflow document that authorises payments or
drives machinery is something you may need to sign and verify years later. XML
has mature, boring, widely implemented answers here.

**Comments survive.** A normative document benefits from explaining itself
in place. JSON has no comments at all.

**Not a fashion argument.** XML is unfashionable, and that is close to
irrelevant for a format whose entire proposition is being readable in 2036.
Documents are mostly *generated from a visual editor and read in a diff*, not
hand-written — which is precisely the case where verbosity costs least and
strictness pays most.

Where JSON genuinely is the better fit — an HTTP API returning an execution
trace, say — the specification defines a JSON projection rather than pretending
the question does not arise.

## What you get

- A workflow that is **one file**, reviewable in a pull request.
- Retry, timeout, escalation and rollback that are **executable, not documentation**.
- A validator that catches a dangling reference, an unbounded loop, or a retry
  policy on a non-idempotent step **before** it reaches production.
- **Provenance** tying an execution back to the design revision that authorised it.
- **No lock-in**: implement the specification yourself, commercially, without
  permission — and check your implementation against a
  [published conformance suite](/conformance/) rather than against a vendor's opinion.
