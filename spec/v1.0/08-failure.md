---
title: Failure, Retries and Compensation
description: HarnessXML retry policy, backoff, timeouts, idempotence and the compensation model — how a workflow undoes what it has already done.
section: specification
order: 9
status: draft
---

# 8. Failure, Retries and Compensation

## 8.1 Retry policy

```xml
<retry maxAttempts="4"
       backoff="exponential"
       initialDelay="PT1S"
       maxDelay="PT1M"
       multiplier="2"
       jitter="true"
       retryOn="rate_limit transient"/>
```

| attribute | default | meaning |
|---|---|---|
| `maxAttempts` | — | **required**; total attempts including the first, 1–1000 |
| `backoff` | `exponential` | `none`, `fixed`, `linear`, `exponential` |
| `initialDelay` | `PT1S` | delay before the second attempt |
| `maxDelay` | — | ceiling on any single delay |
| `multiplier` | `2` | growth factor for `linear` and `exponential` |
| `jitter` | `true` | randomise the delay |
| `retryOn` | — | space-separated error classes; absent means retry any failure |

Absent `<retry>` means **no retry**. One attempt.

`maxAttempts` counts attempts, not retries: `maxAttempts="4"` is one attempt plus
up to three retries. Naming it `maxAttempts` rather than `maxRetries` is
deliberate — the off-by-one between the two readings is one of the most common
misconfigurations in orchestration, and it is always discovered in production.

### 8.1.1 Delay calculation

For attempt *n* (2-based; the first attempt has no delay):

| backoff | delay |
|---|---|
| `none` | `0` |
| `fixed` | `initialDelay` |
| `linear` | `initialDelay × multiplier × (n − 1)` |
| `exponential` | `initialDelay × multiplier^(n − 2)` |

Then clamped to `maxDelay` if present.

With `jitter="true"` (the default) the runtime **MUST** apply randomisation,
**RECOMMENDED** as full jitter — a uniform random value in `[0, delay]`.

> Jitter defaults to on because the failure it prevents is systemic. When a
> shared dependency has a bad minute, every workflow retrying it on an identical
> schedule reconverges into a synchronised thundering herd and turns a blip into
> an outage.

### 8.1.2 Error classes

`retryOn` names error classes the runtime maps its failures onto. The
interoperable core:

| class | meaning | retry? |
|---|---|---|
| `transient` | a temporary condition | usually |
| `rate_limit` | throttled by a dependency | usually, with longer backoff |
| `timeout` | the attempt exceeded its bound | often |
| `unavailable` | the dependency is down | usually |
| `invalid_input` | the request was malformed | **no** — it will fail identically |
| `unauthorized` | credentials rejected | **no** |
| `not_found` | the target does not exist | **no** |
| `internal` | a defect in the step | **no** |

The list is open-ended; a runtime MAY define more. Omitting `retryOn` retries any
failure, which is convenient and usually wrong — retrying `invalid_input` four
times with exponential backoff wastes ninety seconds to reach the same answer.

## 8.2 Timeouts

```xml
<timeout duration="PT3M" onTimeout="retry"/>
```

`duration` bounds **a single attempt**, not the node's total lifetime. A node
with `maxAttempts="4"` and `PT3M` may occupy twelve minutes plus backoff.

| `onTimeout` | effect |
|---|---|
| `fail` (default) | the attempt fails; retry policy applies |
| `retry` | the attempt fails and is retried, even if `retryOn` excludes `timeout` |
| `skip` | the node becomes `SKIPPED` — a successful outcome |

`skip` is for genuinely optional work — an enrichment call that is nice to have.
Using it on anything else converts a stuck workflow into one that silently did
less, which is the failure mode this specification most wants to avoid.

Durations are ISO 8601 and **MUST NOT** use months or years (`HX-3401`). Their
length is not fixed, so a scheduler cannot resolve them deterministically.

## 8.3 Idempotence

A node declared `idempotent="false"`:

- **MUST NOT** be retried automatically by a runtime;
- **MUST NOT** be cancelled while `RUNNING` — the attempt finishes and the result
  is discarded ([§6.2.1](/spec/v1.0/lifecycle/#6-2-1-cancelling-a-running-node));
- combined with a `<retry>` policy, makes the document invalid (`HX-3301`).

The last rule is the point. The unsafe combination is **unrepresentable** rather
than documented as unwise, because "don't do that" in a specification is a rule
somebody will break at 3am under pressure.

Non-idempotent work is not rare: a payment, a ledger entry, a physical grasp, an
email, a work order. The correct pattern is to keep the retry policy on the
*idempotent* part and leave the non-idempotent step bare:

```xml
<!-- fine to retry: reserving is idempotent given the same reservation key -->
<node id="reserve" type="task" impl="pay.reserve" idempotent="true">
  <retry maxAttempts="4" backoff="exponential" retryOn="transient rate_limit"/>
</node>

<!-- NOT retried: capturing twice charges twice -->
<node id="capture" type="task" impl="pay.capture" idempotent="false"/>
```

## 8.4 Compensation

Retries handle a step that might work if tried again. Compensation handles a
workflow that **already changed the world** and must put it back.

```xml
<node id="post_ledger" type="task" impl="ledger.post" idempotent="false"/>

<node id="reverse_entry" type="task" impl="ledger.reverse"
      idempotent="true" compensates="post_ledger"/>

<edges>
  <edge from="post_ledger" to="reverse_entry" type="compensation"/>
</edges>
```

### 8.4.1 When compensation runs

When a node reaches `FAILED` with **no outgoing error edge**, the failure
propagates and the runtime begins **unwinding**:

1. Nodes still `RUNNING` or `READY` are cancelled, subject to the idempotence
   rule in §8.3.
2. Every node in `SUCCEEDED` that has a compensation edge is compensated, in
   **reverse completion order** — most recently completed first.
3. A compensated node transitions `SUCCEEDED` → `COMPENSATED`.
4. The instance completes as *compensated*.

Reverse completion order matters: undoing a shipment before undoing the
allocation that produced it leaves the system in a state neither step anticipated.

A node with an outgoing **error** edge is handled rather than propagating —
compensation does not begin ([§5.8](/spec/v1.0/execution-semantics/#5-8-failure-propagation)).

### 8.4.2 Requirements on compensating nodes

- The target **SHOULD** declare `compensates="<source id>"`; disagreement with the
  edge is invalid (`HX-2004`).
- The target **MUST NOT** be reachable by forward `control` or `data` edges
  (`HX-2005`) — a node that is both a normal step and a rollback will run at the
  wrong time.
- The target **SHOULD** be `idempotent="true"`. Compensation runs during failure
  handling, which is exactly when a runtime is most likely to try again.
- Compensation **SHOULD NOT** itself declare compensation. A rollback of a
  rollback is almost always a modelling error.

### 8.4.3 If compensation fails

A failed compensation is not something a specification can fix. The runtime
**MUST**:

1. continue compensating the remaining nodes — one failure must not abandon the
   rest of the unwind;
2. record the failure distinctly in the trace;
3. complete the instance as **failed**, not compensated.

The workflow is now in a state neither the author nor the runtime can describe,
and saying so loudly is the only correct behaviour. Reporting *compensated* when
the rollback did not happen would be a lie that an auditor eventually finds.

## 8.5 Choosing the mechanism

| situation | use |
|---|---|
| transient dependency failure | `<retry>` with `retryOn` |
| the step may hang | `<timeout>` |
| failure is expected and has a path | `error` edge |
| the step already changed the world | `compensation` edge |
| the step must never run twice | `idempotent="false"` |
| a whole branch is optional | `<guard>` |

The most common design error is reaching for `retry` where `error` or
`compensation` is meant. Retrying a step that failed because the *world* is in an
unexpected state does not change the world — it just takes longer to reach the
same failure.
