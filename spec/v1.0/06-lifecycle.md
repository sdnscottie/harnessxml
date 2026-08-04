---
title: Execution Lifecycle
description: The HarnessXML node lifecycle state machine — every state a node passes through, every legal transition, and what each terminal state means.
section: specification
order: 7
status: draft
---

# 6. Execution Lifecycle

Every node in an execution instance is in exactly one state. The state machine is
normative: two conforming runtimes must agree on which state a node reached, and
that agreement is what makes execution traces comparable across implementations.

## 6.1 The states

<div class="diagram">
<svg viewBox="0 0 880 300" xmlns="http://www.w3.org/2000/svg" role="img"
     aria-labelledby="lctitle lcdesc">
  <title id="lctitle">Node lifecycle state machine</title>
  <desc id="lcdesc">A node starts PENDING. When its join condition is satisfied it
    becomes READY, then RUNNING. From RUNNING it reaches SUCCEEDED, or FAILED. A
    failure with retries remaining goes to RETRYING and back to READY. A node whose
    guard evaluates false goes from READY to SKIPPED. A node cancelled before it
    finished goes to CANCELLED; a node no path ever reached simply stays PENDING.
    SUCCEEDED, SKIPPED, FAILED, CANCELLED and COMPENSATED are terminal.</desc>
  <defs>
    <marker id="a3" viewBox="0 0 10 10" refX="9" refY="5"
            markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M0 0 L10 5 L0 10 z" fill="currentColor"/>
    </marker>
  </defs>
  <g fill="none" stroke="currentColor" stroke-width="1.5" opacity=".6" marker-end="url(#a3)">
    <path d="M104 130 H164"/>
    <path d="M264 130 H324"/>
    <path d="M424 130 H484"/>
    <path d="M424 148 L484 200"/>
    <path d="M544 214 L470 250 L374 250 L374 158"/>
    <path d="M214 148 L214 250 L300 250"/>
    <path d="M64 152 L64 250 L150 250"/>
  </g>
  <g font-size="12.5" text-anchor="middle" fill="currentColor">
    <rect x="14" y="110" width="90" height="40" rx="20" fill="none" stroke="currentColor" stroke-width="1.5" opacity=".8"/>
    <text x="59" y="135">PENDING</text>

    <rect x="164" y="110" width="90" height="40" rx="20" fill="none" stroke="currentColor" stroke-width="1.5" opacity=".8"/>
    <text x="209" y="135">READY</text>

    <rect x="324" y="110" width="100" height="40" rx="20" fill="none" stroke="currentColor" stroke-width="2"/>
    <text x="374" y="135">RUNNING</text>

    <rect x="484" y="110" width="110" height="40" rx="20" fill="currentColor" opacity=".13"/>
    <rect x="484" y="110" width="110" height="40" rx="20" fill="none" stroke="currentColor" stroke-width="1.5"/>
    <text x="539" y="135">SUCCEEDED</text>

    <rect x="484" y="192" width="110" height="40" rx="20" fill="currentColor" opacity=".13"/>
    <rect x="484" y="192" width="110" height="40" rx="20" fill="none" stroke="currentColor" stroke-width="1.5"/>
    <text x="539" y="217">FAILED</text>

    <rect x="300" y="230" width="100" height="40" rx="20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-dasharray="4 3" opacity=".8"/>
    <text x="350" y="255">RETRYING</text>

    <rect x="150" y="230" width="100" height="40" rx="20" fill="currentColor" opacity=".13"/>
    <rect x="150" y="230" width="100" height="40" rx="20" fill="none" stroke="currentColor" stroke-width="1.5"/>
    <text x="200" y="255">SKIPPED</text>

    <rect x="640" y="110" width="115" height="40" rx="20" fill="currentColor" opacity=".13"/>
    <rect x="640" y="110" width="115" height="40" rx="20" fill="none" stroke="currentColor" stroke-width="1.5"/>
    <text x="697" y="135">CANCELLED</text>

    <rect x="640" y="192" width="130" height="40" rx="20" fill="currentColor" opacity=".13"/>
    <rect x="640" y="192" width="130" height="40" rx="20" fill="none" stroke="currentColor" stroke-width="1.5"/>
    <text x="705" y="217">COMPENSATED</text>

    <text x="440" y="30" font-size="11.5" opacity=".65">shaded = terminal</text>
  </g>
</svg>
</div>

| state | meaning | terminal |
|---|---|---|
| `PENDING` | created; join condition not yet satisfied | no |
| `READY` | join condition satisfied; may be scheduled | no |
| `RUNNING` | executing | no |
| `RETRYING` | an attempt failed; retries remain; waiting out the backoff | no |
| `SUCCEEDED` | completed successfully | **yes** |
| `SKIPPED` | **reached**, and its guard evaluated false — a successful outcome | **yes** |
| `FAILED` | failed with retries exhausted | **yes** |
| `CANCELLED` | terminated before completion | **yes** |
| `COMPENSATED` | had succeeded, then was rolled back | **yes** |

## 6.2 Legal transitions

A conforming runtime **MUST NOT** perform any transition not listed here.

| from | to | when |
|---|---|---|
| `PENDING` | `READY` | join condition satisfied |
| `PENDING` | `CANCELLED` | instance cancelled while the node was still waiting |
| `READY` | `RUNNING` | scheduled, guard true |
| `READY` | `SKIPPED` | guard evaluated false |
| `READY` | `CANCELLED` | instance cancelled, or a sibling satisfied an `any` join |
| `RUNNING` | `SUCCEEDED` | execution completed |
| `RUNNING` | `FAILED` | execution failed with no retries remaining |
| `RUNNING` | `RETRYING` | execution failed with retries remaining |
| `RUNNING` | `CANCELLED` | cancelled — **only if `idempotent="true"`** |
| `RETRYING` | `READY` | backoff elapsed |
| `RETRYING` | `FAILED` | retry budget exhausted, or a non-retryable error class |
| `RETRYING` | `CANCELLED` | instance cancelled |
| `SUCCEEDED` | `COMPENSATED` | compensation ran during unwinding |

Everything else is a runtime defect.

### 6.2.1 Cancelling a `RUNNING` node

A runtime **MUST NOT** cancel a `RUNNING` node declared `idempotent="false"`. It
**MUST** let the attempt finish and then discard the result.

Interrupting a non-idempotent action mid-flight leaves the world in a state
nobody can describe: was the payment sent? did the arm complete the grasp? A
result that is discarded is at least a known outcome.

## 6.3 `SKIPPED` is a success

The state most often implemented wrongly.

`SKIPPED` is **terminal and successful**. A node whose guard evaluated false did
exactly what the document asked. So:

- outgoing `control` edges **are** satisfied;
- outgoing `data` edges are satisfied for scheduling, but the values are
  **unavailable** (see [§5.6](/spec/v1.0/execution-semantics/#5-6-data-availability));
- outgoing `error` edges are **not** taken — nothing failed.

Treating skip as failure means any optional step halts everything after it, which
is not what a guard means.

## 6.4 `PENDING` at completion is normal

A node that was never reached stays `PENDING` when the instance completes.

This is the expected outcome for the branch a decision did not take. It is not an
error, and a runtime **MUST NOT** report the instance as failed because nodes
remain `PENDING`.

Distinguish it from the two states it is most often confused with. All three
are ordinary outcomes, and an incident review needs to tell them apart:

| state | means | successors |
|---|---|---|
| `PENDING` at completion | **never reached** — no path arrived | also not reached |
| `SKIPPED` | **reached**, and its guard was false | control successors still run |
| `CANCELLED` | **reached and started**, then stopped | not reached |

Conflating `PENDING` with `SKIPPED` makes the untaken branch of every decision
report as a success that ran.

## 6.5 Loop iterations

A `loop` node has its own lifecycle, and so does **each iteration** of its body.
Iteration states are scoped to the iteration and do not overwrite one another —
iteration 3 failing does not put the body node in `FAILED` for iterations 4
onward.

The loop node's own outcome follows `onItemFailure`:

| `onItemFailure` | loop node reaches |
|---|---|
| `fail` (default) | `FAILED` on the first iteration failure |
| `continue` | `SUCCEEDED` if any iteration succeeded; `FAILED` if all failed |
| `break` | `SUCCEEDED`, stopping at the first failure |

## 6.6 Traces

A runtime **SHOULD** emit a trace of state transitions. A trace entry **SHOULD**
carry the node id, both states, a timestamp, the attempt number, and for
`FAILED`, the error class and message.

Conformance at Executing and Full level compares the **normalised sequence of
transitions**, not timing and not the interleaving of independent branches — two
runtimes may schedule unrelated work in different orders. What they may not do is
disagree about whether a node ran, was skipped, retried, failed or was
compensated. See [conformance](/conformance/).
