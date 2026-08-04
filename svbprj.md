# harnessxml

Agrarobotics sub-project — **HarnessXML**, the open specification for executable
intelligent system workflows, and **harnessxml.com**, its official public home.

> ⚠️ **The name is misleading and cost a false start.** "Harness" here is an
> *execution* harness — the thing that runs a workflow, as a test harness runs
> tests. It is **not** a wiring loom. A first pass scaffolded an electrical
> wiring-harness format under that reading and was deleted.

- **Domain:** `harnessxml.com` — owned, GCP-hosted alongside visml.com
- **Repo:** `gitlab.com/visml/harnessxml` (public)
- **Steward:** VisML · **Flagship designer:** RuMima Enterprise Studio
- **Status:** v1.0 **draft** — see §Status

## What it is

An open, vendor-neutral specification for describing workflows that call models,
move data, drive machines, wait on people, fail, retry and compensate. Positioned
as foundational infrastructure in the spirit of HTML, GraphML or OpenAPI — and
the site markets the **language and execution model**, never XML itself.

The three-line philosophy the homepage leads with:

1. The visual graph is the authoritative design.
2. HarnessXML is the portable machine-readable representation of it.
3. The runtime executes that specification consistently across platforms.

```
RuMima Visual Graph → Internal Object Model → HarnessXML
                    → Validator → Harness Runtime → Execution → Monitoring
```

## The commercial/open split — the part to get right

VisML is both steward and vendor, which is a real conflict of interest.
`GOVERNANCE.md §1` names it rather than glossing it, and the structural answers
are: no privileged extension point (RuMima uses the same public mechanism as any
third party), a reference runtime that is **not** RuMima, and conformance defined
by a published test suite rather than by agreement with any product.

**File extensions, and the rule that keeps the whole positioning honest:**

| ext | format | open? |
|---|---|---|
| **`.hxml`** | **HarnessXML** — this specification | **open, vendor-neutral** |
| `.visml` | VisML Markup Language — shared native format of VisML's products, incl. RuMima | vendor format |

A `.visml` document **embeds** a complete `<harness>` as a child element; export
lifts it out. **The dependency runs one way only** — HarnessXML must be fully
definable and executable without reference to `.visml`. It is *not* a subset or
profile of the vendor format, exactly as SVG is not a subset of HTML.

> `.visml` was chosen over `.vxml` because **`.vxml` already belongs to VoiceXML**
> (a W3C standard). With one extension carrying VisML's whole product surface,
> that collision would have been paid everywhere. `.rxml` was the earlier plan,
> superseded by the one-format-family decision.

## Layout

```
harnessxml/
├── svbprj.md                       ← this file
├── README.md                       ← the repo's public front page
├── LICENSE-SPEC                    ·   CC BY 4.0 — specification text
├── LICENSE-CODE                    ·   Apache 2.0 — all code (patent grant)
├── GOVERNANCE.md · CONTRIBUTING.md · CODE_OF_CONDUCT.md
├── spec/v1.0/                      ·   16 specification chapters (Markdown)
├── schema/v1.0/harnessxml-1.0.xsd  ·   normative structural schema
├── examples/{ai,robotics,networking,enterprise}/
├── reference-runtime/              ·   Rust parser + validator + `harnessxml` CLI
├── conformance/                    ·   the suite third parties run
├── site/                           ·   build.py (stdlib only) + content + assets
├── deploy/deploy_harnessxml.sh     ·   GCP deploy
└── .github/workflows/              ·   ci.yml, deploy.yml
```

`GOVERNANCE.md`, `CONTRIBUTING.md` and `CODE_OF_CONDUCT.md` are published as site
pages **directly from the repo root** — a governance policy that exists twice is
one that will eventually say two different things.

## Build & Run

```bash
python3 site/build.py --check --serve       # build site, verify links, serve :8000
cd reference-runtime && cargo test          # 14 tests
./target/release/harnessxml validate ../examples/ai/document-triage.hxml
./target/release/harnessxml explain  ../examples/robotics/pick-and-place.hxml
python3 conformance/validate.py --cmd "reference-runtime/target/release/harnessxml validate"
```

`site/build.py` is **standard library only, no network at build time** — a
documentation site that cannot be rebuilt in ten years is not an archive.

## Hosting — GCS bucket + Cloud CDN, NOT a VM

Verified against live GCP on 2026-08-04.

**There is no visml.com VM.** The whole VisML umbrella is serverless. What serves
the websites is a **GCS bucket behind a backend bucket with Cloud CDN**, attached
as a host rule to one shared classic global HTTPS load balancer. Cloud Run is used
only for the two *applications* (collab hub, licensing API).

| | |
|---|---|
| project | `agrarobotics-licensing` |
| LB IP | `34.49.81.67` (`rumima-lb-ip`) |
| url-map | `rumima-urlmap` · proxy `rumima-https-proxy` |
| existing hosts | `visml.com`+`www` → bucket `visml-web`; `collab.agrarobotics.com`, `collab.visml.com` → Cloud Run; default → bucket `agrarobotics-rumima-web` |
| harnessxml | bucket `harnessxml-web` → backend bucket `harnessxml-backend` (CDN on) → new path matcher on the **same** url-map |
| HTTP→HTTPS | `rumima-http-redirect` uses `defaultUrlRedirect` and enumerates no hosts, so a new host is covered automatically |

### Verified by experiment, not memory

**Directory-index resolution works on a backend bucket.** This decided the
architecture. The widely-documented claim that a backend bucket cannot resolve
`/dir/` → `/dir/index.html` is wrong when the bucket carries a website config.
Probe (`gs://visml-web/_probe/index.html`, since deleted):

```
https://visml.com/_probe/            -> 200   directory index RESOLVES
https://visml.com/_probe/index.html  -> 200
https://visml.com/_probe             -> 301   redirects to the slash form
```

An earlier test showing `/rumima/` → 404 was inconclusive: that bucket holds only
`index.html`, and the rumima site sidesteps the question entirely by being flat
`.html` files. So clean, citable URLs like `/spec/v1.0/execution-semantics/` work
on a plain bucket — **Cloud Run is not needed**, and an initial recommendation to
use it was reversed.

### ⚠️ Traps

**`--ssl-certificates` REPLACES the whole list; it does not append.** The proxy
carries four certs (`rumima-cert-2`, `collab-cert`, `visml-cert`,
`visml-apex-cert`). Passing only `harnessxml-cert` would instantly break TLS for
rumima, both collab hosts and visml.com. `deploy_harnessxml.sh` reads the current
list and appends. (`rumima-cert` is a 5th, unattached leftover — leave it.)

**Cloud Armor on a backend *bucket* supports only EDGE policies** — IP/geo/header
allow-deny at the CDN edge. The full backend policies (OWASP preset, rate
limiting, bot management) need a backend *service*. Acceptable here: the OWASP
ruleset defends an application, and this site is static, public, read-only, with
no forms, no auth and no server-side code.

**Deliberate divergence from visml.com:** `gs://visml-web` sets
`notFoundPage=index.html`, so a mistyped URL returns the homepage with HTTP 200.
Fine for one page, wrong for a docs site — it hides broken links from readers and
lets search engines index infinite duplicate homepages. `harnessxml-web` gets a
real `404.html`. Also, visml.com currently sends **no security headers at all**
and leaks `server: UploadServer`; harnessxml.com sets HSTS/CSP/etc. via
`--custom-response-header` from day one.

**Never delete or reorder the `collab.agrarobotics.com` host rule.**

## Status — deliberately unflattering

| piece | state |
|---|---|
| Object model, XSD, 16 spec chapters | drafted; schema compiles, enforces referential integrity via `xs:key`/`xs:keyref` |
| 4 reference examples | complete, schema-valid, double as conformance fixtures |
| Reference **validator** (Rust) | working — all layer-1/2 rules, 14 tests, clippy/fmt clean |
| Reference **executor** | **not started** |
| Conformance corpus | runner works; 11 fixtures; corpus incomplete |
| SDKs beyond Rust | **not started** |
| Site | 31 pages, builds clean, link-checked |
| GCP deploy | scripted, **not yet run** |

## The reference implementation earned its keep immediately

It rejected `examples/networking/config-rollout.hxml` under rule `HX-3004`
("a loop body must not be reachable by forward edges from outside the loop"). The
example was right and the **rule was wrong**: a loop body almost always needs a
loop-invariant input, and `data` edges are how that is bound. `HX-3004` now
covers `control` and `dependency` edges only, and spec §7.2.6 records why.

That is precisely what a reference implementation is for, and the episode is
worth keeping: normative prose that sounds airtight fails against real documents.

## Conventions

- Rust edition 2024, minimal dependencies (`quick-xml` only)
- Python: **standard library only** in `site/build.py` and `conformance/`
- Every normative rule carries an `HX-nnnn` code, and every code has a fixture
- Docs + diagrams in `docs_about__harnessxml/`
- Commit trailer: `Co-Contributed-By: CC Opus 4.8 <noreply@anthropic.com>`

## Sibling Projects

- `../rumima` — the flagship visual designer; authors `.visml`, exports `.hxml`
- `../visml` — the umbrella brand and steward
- `../erpnext` — a candidate real workflow to model in HarnessXML
- `../../agrarobotics__GCP_vms` — the shared LB and deploy notes this reuses
