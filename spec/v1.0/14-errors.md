---
title: Error Reporting
description: The HarnessXML HX-nnnn code space, runtime error classes, and what a conforming implementation must report.
section: specification
order: 15
status: draft
---

# 14. Error Reporting

## 14.1 The code space

```
HX-1xxx   document and structure     validation time
HX-2xxx   references and interfaces  validation time
HX-3xxx   semantics                  validation time
HX-4xxx   runtime                    execution time
```

Codes are **permanent**. Once assigned, a code is never reused for a different
rule and never removed — a removed code would break every implementation's
diagnostics and every conformance fixture asserting it. A rule that is withdrawn
leaves its code retired.

New codes may be added in a minor version. An implementation meeting an
unrecognised code MUST treat it as an error of its class (by leading digit) and
report it verbatim, rather than discarding it.

Validation codes `1xxx`–`3xxx` are catalogued in
[chapter 13](/spec/v1.0/validation/). This chapter covers `4xxx` and the
reporting requirements that apply to all of them.

## 14.2 Runtime errors — `HX-4xxx`

| code | condition |
|---|---|
| `HX-4101` | A required input has no value at execution time — typically because its producer was `SKIPPED` and the input declares no `default`. |
| `HX-4102` | A referenced resource could not be resolved, or its credential could not be retrieved. |
| `HX-4103` | A `decision` node matched no case and declared no `<otherwise>`. |
| `HX-4104` | A loop exceeded `@maxIterations`. |
| `HX-4105` | An artifact's content digest did not match its declaration. |
| `HX-4106` | An expression performed an invalid operation — incompatible types, or `null` in an operator other than `==` / `!=`. |
| `HX-4107` | A `guard`, `case` or `condition` expression evaluated to a non-boolean. |
| `HX-4108` | A node exceeded its `<timeout>` and `onTimeout="fail"`. |
| `HX-4109` | A subworkflow could not be loaded, or its digest did not match. |
| `HX-4110` | Compensation failed; the instance completes as **failed**, not compensated. |
| `HX-4201` | A declared `principal` or `isolation` level could not be honoured. |
| `HX-4202` | A declared permission was denied by the runtime. |

### 14.2.1 Errors that must never degrade

Four of these have a tempting "graceful" alternative that the specification
forbids, because each one converts a visible failure into a silent wrong answer:

| code | the tempting behaviour | why it is forbidden |
|---|---|---|
| `HX-4101` | pass `null` downstream | the workflow proceeds using data nobody produced |
| `HX-4104` | stop quietly at the limit | reports success having processed part of its input |
| `HX-4105` | proceed on digest mismatch | a substituted input is indistinguishable from a correct run in every log |
| `HX-4201` | fall back to a default identity or weaker isolation | the step runs with privilege the document said it must not have |

## 14.3 Error classes

Runtime failures originating in a step — as opposed to in the workflow structure
— are additionally assigned an **error class**, which is what `retryOn` matches
([§8.1.2](/spec/v1.0/failure/#8-1-2-error-classes)):

`transient` · `rate_limit` · `timeout` · `unavailable` · `invalid_input` ·
`unauthorized` · `not_found` · `internal`

The list is **open-ended**; a runtime MAY define additional classes. A runtime
**MUST** document how it maps its failures onto these, because a `retryOn` list
means nothing to a reader who cannot see the mapping.

A runtime **SHOULD NOT** classify a failure as `transient` when it has no
evidence of transience. Defaulting to `transient` means every permanent failure
is retried to exhaustion — turning a two-second error into a two-minute one and
burning the retry budget that a genuinely transient failure needed.

## 14.4 What a report must contain

Every error report, at validation or at runtime, MUST include:

1. the **code**;
2. the **location** — file, line and column for validation; node id and attempt
   number for runtime;
3. a **message** naming the specific offending value, not just the rule.

```
HX-2303  workflow.hxml:47:5
         edge 'e_hr_cat': toPort 'suggestedCategry' is not an input on node
         'human_review' (did you mean 'suggestedCategory'?)

HX-4102  node 'classify', attempt 1
         resource 'classifier': credential 'ANTHROPIC_API_KEY' not found in
         store 'vault'
```

A message naming the offending value is the difference between a diagnostic and a
rule number. `HX-2303` alone tells an author which rule they broke; the message
tells them where.

## 14.5 Credentials in errors

A runtime **MUST NOT** include a resolved credential in any error message, log
line or trace entry — including stack traces, and including when the failure was
an authentication failure.

The failure mode is specific and common: the exception carrying the key it could
not authenticate with, written to a log that is shipped, indexed and retained.
Report the credential's **reference** (`ANTHROPIC_API_KEY`) and its store, never
its value.

## 14.6 Reporting all findings

A validator **SHOULD** report every finding rather than stopping at the first.

Fixing one error per build cycle across five cycles is an experience that
implementations have no reason to inflict, and it is the single most common
complaint about strict validators. Where one error makes later analysis
impossible — a malformed document that will not parse — reporting only the first
is correct, and the implementation **SHOULD** say that analysis stopped rather
than implying the document had one problem.

## 14.7 Exit codes

For a command-line validator:

| exit code | meaning |
|---|---|
| `0` | valid; warnings may have been reported |
| `1` | invalid; at least one error |
| `2` | the tool itself failed — could not read the file, could not load the schema |

Separating `1` from `2` matters in CI. "This workflow is wrong" and "the
validator is broken" require different responses, and collapsing them into one
non-zero exit means a broken toolchain looks like a broken workflow.

A validator **MUST NOT** exit non-zero for warnings alone. A build that fails on
advisory findings trains people to suppress warnings, which loses the errors too.
