---
title: Vision
description: What HarnessXML is trying to become, and the specific problem it exists to solve — the gap between the workflow a team designed and the orchestration code that actually runs.
section: introduction
order: 2
status: stable
---

# Vision

## The gap this exists to close

Every organisation building intelligent systems ends up with two representations
of the same workflow, and they are never the same thing.

The first is **the design**: a diagram. It shows the steps, what feeds what,
where the branch is, what happens when the model is unsure. Everyone — including
the people who cannot read the code — agrees this is what the system does.

The second is **the implementation**: orchestration code. Retry logic in a
decorator. A branch buried in an `if`. A timeout in a config file three
repositories away. A rollback that exists as a paragraph in a runbook.

The diagram is out of date within weeks, and everybody knows it. So the design
stops being consulted, review moves entirely into code review, and the people
who understood the *system* stop being able to see it. The failure is not that
teams are careless. It is that **the two representations have no mechanical
relationship**, so drift is the default and alignment is unpaid manual work.

HarnessXML closes the gap by making the design executable. Not a picture of the
system — the system.

## What success looks like

**A workflow is reviewed the way code is reviewed.** A change to a retry policy,
an approval threshold or an escalation path arrives as a diff, in a pull request,
with a validator running in CI. The reviewer sees the change to the *design*,
because the design is the file.

**A workflow outlives the tool that drew it.** A document written in one editor
opens in another, and runs on a runtime neither vendor wrote. That is the
difference between a file format and a specification, and it is the difference
between a workflow being an asset and being a lock-in.

**A workflow is auditable years later.** A regulator, an incident review, or a
new engineer can take an execution trace and follow it back to the exact
document, the exact design revision, and the exact artifact digests that produced
it — because provenance is in the format rather than in whatever logging someone
remembered to add.

**A workflow does not silently do less than it says.** A runtime meeting a
construct it does not understand fails loudly instead of skipping it. This one
rule is the difference between a format you can trust and a format you have to
verify by hand.

## What HarnessXML deliberately does not do

A specification's value is as much in what it refuses as in what it covers.

**It does not define what a step does.** A node carries an `impl` handle the
runtime resolves; whether that is a Python function, a container image, a
NETCONF call or a robot trajectory is out of scope. HarnessXML describes the
*workflow*, and stops at the boundary of the work. This is why it does not go
stale when the underlying technology turns over — and the underlying technology
in this field turns over constantly.

**It is not a programming language.** There are no user-defined functions, no
classes, no general recursion. The expression language exists to read outputs and
make decisions, and it is kept small on purpose: a workflow you cannot statically
analyse is a workflow you cannot validate, and validation is the point.

**It is not a scheduler, a queue, or a runtime.** It specifies the *semantics* a
runtime must implement. How the runtime distributes work, persists state or
recovers from a crash is an implementation decision, and implementations should
compete on it.

**It does not try to be minimal for its own sake.** Retries, timeouts,
compensation, idempotence, bounded loops and provenance are in the core because
leaving them out does not make them go away — it makes every implementation
invent them differently, which is exactly the interoperability failure a
specification exists to prevent.

## The longer arc

The formats that lasted — HTML, GraphML, OpenAPI — share a shape. Each took
something people were already doing incompatibly and wrote it down precisely
enough that independent implementations could agree, then held still long enough
to be trusted.

HarnessXML is aimed at the same shape, for executable intelligent system
workflows. That means the boring commitments matter more than the features:
released versions frozen forever, breaking changes only at a major version,
proposals argued in public, rejected proposals kept published, and a conformance
suite that decides who is compatible rather than a vendor's opinion.

Those commitments are written down in the [governance model](/governance/),
including the conflict of interest VisML carries as both steward and commercial
vendor — and the structural limits placed on it.
