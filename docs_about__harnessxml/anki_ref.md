---
project: harnessxml
anki_id: TBD_ASSIGN_ME    # YYYYMMDD_HHMMSS — assigned by Scottie
status: v1.0 draft — spec + validator working, executor not started
last_updated: 2026-08-04
---

# Anki Reference Card — harnessxml

Source of truth for the compact reference block in the upper-right of every
`.drawio` in this sub-project. Update here first, then re-render the diagrams.

## Concept

- **HarnessXML** — the **Open Specification for Executable Intelligent System Workflows**
- ⚠️ "Harness" = **execution** harness (runs a workflow), **NOT** a wiring loom
- Positioned like HTML / GraphML / OpenAPI. Markets the **language + execution
  model**, never XML itself
- Domains: AI orchestration · agent systems · ML pipelines · robotics · network,
  industrial and business-process automation

**Philosophy (3 lines):** the visual graph is the authoritative design →
HarnessXML is its portable machine-readable form → the runtime executes that
specification consistently across platforms.

**Pipeline:** Rumima Visual Graph → Object Model → **HarnessXML** → Validator →
Harness Runtime → Execution → Monitoring

## Extensions

| ext | format | open? |
|---|---|---|
| **`.hxml`** | HarnessXML — this spec | **open, vendor-neutral** |
| `.visml` | VisML Markup Language (Rumima + all VisML products) | vendor format |

`.visml` **embeds** a complete `<harness>`; export lifts it out. Dependency runs
**one way only** — HarnessXML never depends on `.visml`. Not a subset/profile.
(`.vxml` rejected: already VoiceXML. `.rxml` superseded.)

## The language, in one card

- **12 node types:** task · inference · transform · decision · loop · parallel ·
  barrier · subworkflow · source · sink · wait · human
- **5 edge types (semantics, not styling):** control · data · dependency · error ·
  compensation
- **9 lifecycle states:** PENDING → READY → RUNNING → SUCCEEDED | SKIPPED |
  FAILED | CANCELLED | COMPENSATED, plus RETRYING
- **Error codes:** `HX-1xxx` structure · `2xxx` references · `3xxx` semantics ·
  `4xxx` runtime
- **Namespace:** `https://harnessxml.com/spec/1.0` (major version only)

**The four rules that define its character**

1. **Fail loudly on the unknown** (`HX-1003`) — never skip an unrecognised
   construct; the skipped node could be the approval gate or the rollback
2. **`maxIterations` is required** — no unbounded loop, ever
3. **Idempotence is declared, never inferred** — retry on `idempotent="false"` is
   invalid (`HX-3301`), i.e. unrepresentable rather than discouraged
4. **Credentials are referenced, never contained** (`HX-3501`)

## Openness

- Spec text **CC BY 4.0** · code **Apache 2.0** (chosen over MIT for the patent
  grant) · **trademarks reserved** — the only enforcement lever
- No CLA, no royalty, no permission needed to implement commercially
- Released versions **frozen forever**; corrections are dated errata
- HXEP process: public, 30-day review, rejected proposals stay published
- Conflict of interest (VisML = steward + vendor) named in `GOVERNANCE.md §1`

## Hosting — verified live 2026-08-04

- **No VM, no Cloud Run.** GCS bucket `harnessxml-web` → backend bucket
  (Cloud CDN on) → host rule on the **existing shared** classic global HTTPS LB
- project `agrarobotics-licensing` · IP **34.49.81.67** · url-map `rumima-urlmap`
  · proxy `rumima-https-proxy`
- Same pattern as visml.com (`visml-backend` → `gs://visml-web`)
- ⚠️ `--ssl-certificates` **REPLACES** the list — 4 certs already attached; the
  deploy script reads-then-appends or TLS breaks for rumima/collab/visml
- ⚠️ Backend buckets support **edge-only** Cloud Armor policies
- Probe proved directory-index resolution works on a backend bucket → clean
  citable URLs, so Cloud Run was **not** needed

## Numbers

| | |
|---|---|
| Spec chapters | 16 + index |
| Site pages | 31, link-checked |
| Node / edge types | 12 / 5 |
| Reference examples | 4 (AI, robotics, networking, enterprise) |
| Conformance fixtures | 11 (4 valid + 7 invalid, all code-asserted) |
| Rust tests | 14 |

## Sibling projects

- `../rumima` — flagship designer, authors `.visml` → exports `.hxml`
- `../visml` — umbrella brand, steward of the specification
- `../../agrarobotics__GCP_vms` — the shared LB this reuses
