# harnessxml

Agrarobotics sub-project — **HarnessXML**, the open specification for executable
intelligent system workflows, and **harnessxml.com**, its official public home.

> ⚠️ **The name is misleading and cost a false start.** "Harness" here is an
> *execution* harness — the thing that runs a workflow, as a test harness runs
> tests. It is **not** a wiring loom. A first pass scaffolded an electrical
> wiring-harness format under that reading and was deleted.

- **Domain:** `harnessxml.com` — owned, GCP-hosted alongside visml.com
- **Repo:** `gitlab.com/visml/harnessxml` (public)
- **Steward:** VisML · **Flagship designer:** Rumima Enterprise Studio
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
Rumima Visual Graph → Internal Object Model → HarnessXML
                    → Validator → Harness Runtime → Execution → Monitoring
```

## The commercial/open split — the part to get right

VisML is both steward and vendor, which is a real conflict of interest.
`GOVERNANCE.md §1` names it rather than glossing it, and the structural answers
are: no privileged extension point (Rumima uses the same public mechanism as any
third party), a reference runtime that is **not** Rumima, and conformance defined
by a published test suite rather than by agreement with any product.

**File extensions — THREE layers, and the rule that keeps the positioning honest:**

| name | layer | open? |
|---|---|---|
| **`.hxml`** | **interchange** — HarnessXML, the open specification | **open, vendor-neutral** |
| `.visml` | the markup standard used *inside* Rumima documents | vendor format |
| `.rmmx` | the file a Rumima document is **saved as** | vendor format |

```
.rmmx                          the Rumima document — the file on disk
  └── .visml markup            the markup standard inside it
        └── <harness>          a complete HarnessXML document, embedded

Rumima Enterprise Studio → .rmmx → .hxml → any runtime → execution
```

**Only `.hxml` crosses the boundary between tools.** A runtime is handed the
export and never sees `.rmmx` or `.visml` — which is what makes the runtime
replaceable.

**THE ONE-WAY RULE (normative, spec §2.9.1).** HarnessXML must be fully
definable, validatable and executable without reference to `.visml`, `.rmmx` or
any host format. It is *not* a subset, profile or extension of any of them — it
is an independent specification a host document happens to contain, exactly as
an HTML page may contain SVG. A host may depend on HarnessXML; HarnessXML must
never depend on a host. Reverse that and "open and vendor-neutral" becomes a
claim the format's own definition contradicts.

> `.vxml` was rejected: it already belongs to **VoiceXML** (W3C). `.rxml` was an
> earlier plan, superseded.

**The dynamic layer.** `.visml` does finetuning and attribute injection at
authoring time. That is useful and sits in tension with static analysability, so
the site states where flexibility belongs — resolve at export, or declare it in
`<config>`, port `value` expressions and `<extension>` — and the line it must
not cross: a running instance mutated in place with no new document identity is
**not conformant**, because nothing can say afterwards what actually executed.

## Naming decision — Rumima, not RuMima

Brand casing is **Rumima**. Corrected across 52 occurrences in 17 files.

The VisML feature page is **`visml.com/harnessxml/`**, never `/harness/`:
"harness" alone is overloaded (wiring, test, safety, climbing), and that
ambiguity is not hypothetical — it caused a false start on this very project.
The vendor extension namespace in the training example moved with it. A
namespace URI is an *identifier*, not a link, so changing one changes what a
document means: free while nothing depends on it, not free later.

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
├── examples/{ai,robotics,networking,enterprise,training}/
├── examples/rmmx/                  ·   REAL Rumima maps + screenshots of the live app
├── reference-runtime/              ·   Rust: parser + validator + EXECUTOR + CLI
├── sdk/python/                     ·   Python SDK — stdlib only, incl. document builder
├── sdk/go/                         ·   Go SDK — stdlib only (encoding/xml)
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
./target/release/harnessxml run     ../examples/ai/document-triage.hxml --trace

# the SAME conformance suite, against all three implementations
python3 conformance/validate.py --cmd "reference-runtime/target/release/harnessxml validate"
python3 conformance/validate.py --cmd "sdk/python/harnessxml-validate"
cd sdk/go && go build -o /tmp/hx ./cmd/harnessxml && cd ../.. \
  && python3 conformance/validate.py --cmd "/tmp/hx validate"
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
| 5 reference examples | AI, robotics, networking, enterprise, training — schema-valid, double as fixtures |
| 2 Rumima `.rmmx` maps + screenshots | built in the REAL app over its REST API, not mock-ups |
| Reference **validator** (Rust) | working — all layer-1/2 rules |
| Reference **executor** (Rust) | working — lifecycle, joins, decisions, loops, retries, compensation, JSON traces |
| Expression language (ch.10) | working — full evaluator |
| **Python SDK** | working — parser, validator, document builder, CLI, 25 tests |
| **Go SDK** | working — parser, validator, CLI; `go.mod` has no requires |
| Rust tests | **43** passing, clippy `-D warnings` + fmt clean |
| Conformance corpus | 12 fixtures + 3 frozen execution traces; **corpus still incomplete** |
| Site | 32 pages, link-checked, visml.com house style |
| CI | GitLab + GitHub both green, 7 jobs each |
| SDKs beyond Rust/Python/Go | **not started** |
| **Independent** implementation | **none** — see below |

**THE HEADLINE RESULT.** All three implementations pass the same conformance
suite with the **same error codes**:

```
Rust reference   passed 12   failed 0
Python SDK       passed 12   failed 0
Go SDK           passed 12   failed 0
```

Three codebases in three languages reaching identical verdicts is the best
evidence available that the specification is precise enough to implement from.
**But the same author wrote all three**, so they are ADDITIONAL implementations,
not INDEPENDENT ones. The "independent implementation" release gate stays open
until someone outside VisML implements it. The roadmap says so in those words.

## Building the executor found THREE contradictions in the specification

The strongest argument for a reference implementation, demonstrated three times:

1. **§2.5 and §5.2 contradicted their own next sentence.** "Entry set = every
   node with no incoming control/data/dependency edge" made an ERROR HANDLER —
   which has only an incoming error edge — into a START. The executor
   cheerfully ran a failure handler on a workflow where nothing had failed.
   Entry set is now "no incoming edge of any type".

2. **§5.3.1 said an all-negative join yields SKIPPED; §6.4 said unreached nodes
   stay PENDING.** Flatly contradictory. §6.4 is right: SKIPPED means *reached,
   guard false* and is a SUCCESS whose successors run; PENDING means *never
   reached*. Conflating them made the untaken branch of every decision report as
   a success that ran, then fail its consumers with HX-4101 for a value nobody
   intended to produce.

3. **NEW §5.3.3 — unreachability PROPAGATES.** Nothing said an unreachable
   source makes its outgoing edges resolved-negative, so a downstream `all` join
   waited forever on a branch never taken and the instance DEADLOCKED.

Two examples were wrong too: robotics had a dependency edge expressing a
loop-BACK (deadlock — repetition must be a loop node), and enterprise had
`reverse_entry` as both an error target and a compensation target, so the error
edge HANDLED the failure and the compensation the example exists to demonstrate
never ran.

## CI caught two real bugs

Neither was noise:

- **gitleaks exit 1** — correct detection. The repo contains strings shaped
  exactly like Anthropic API keys, in the HX-3501 fixtures. They are FAKE and
  deliberate: the only way to prove a validator enforces "no literal
  credentials" is to feed it one. `.gitleaks.toml` allowlists them NARROWLY, by
  literal value, so a real leak in those same files is still caught.
- **cargo audit exit 1** — a real vulnerability. quick-xml 0.37.5 carried
  **RUSTSEC-2026-0195**, unbounded namespace allocation in `NsReader` → memory
  exhaustion DoS. `parse.rs` uses `NsReader`, and a validator whose job is
  parsing UNTRUSTED documents is the worst place to carry a parser DoS. Upgraded
  to 0.41.0; handled the `unescape_value` → `normalized_value(XmlVersion)` API
  change.

## Repos, sites and CI

| | |
|---|---|
| source of truth | **gitlab.com/visml/harnessxml** (public) |
| mirror | **github.com/sdnscottie/harnessxml** (public) |
| spec site | **harnessxml.com** — GCS bucket `harnessxml-web` + Cloud CDN |
| vendor page | **visml.com/harnessxml/** — GCS bucket `visml-web`, same shared LB |

> ⚠️ GitLab pipelines fail INSTANTLY with **zero jobs and `yaml_errors: null`**
> until the account completes **identity verification** (card or phone). That is
> "never scheduled", not "broken config" — every new GitLab account hits it once.
> `visml` is verified; pipelines went green immediately after.

> ⚠️ `gh` on this box has TWO accounts. `scottsoft` is a *profile name*; the
> username is **`scottie-svb`**, and the repo lives under **`sdnscottie`**.
> Pushing `.github/workflows/**` needs the **`workflow` token scope** —
> `gh auth refresh -h github.com -s workflow`. Use HTTPS, not SSH: the SSH key
> here belongs to `sdnscottie` and GitHub refuses one key on two accounts.

## Deploy auth — Workload Identity Federation, no keys

**The org enforces `constraints/iam.disableServiceAccountKeyCreation`**, so a
service-account JSON key CANNOT be created. That settles the design: GitLab CI
authenticates by **WIF over OIDC** and no long-lived credential exists anywhere.

OIDC and WIF are two halves of one handshake, not alternatives: GitLab mints a
short-lived signed JWT proving the job runs in `visml/harnessxml`; GCP's WIF
trusts that issuer and exchanges it for a short-lived access token.

| piece | value |
|---|---|
| pool | `gitlab-pool` (global) |
| provider | `gitlab` — issuer `https://gitlab.com`, **attribute-condition pins `project_path == 'visml/harnessxml'`** |
| service account | `harnessxml-deploy@agrarobotics-licensing.iam.gserviceaccount.com` |
| project number | `96423820656` |

The attribute-condition is the security boundary: without it, ANY GitLab project
on gitlab.com could federate into this service account.

GitLab CI/CD variables are `GCP_PROJECT_ID`, `GCP_SERVICE_ACCOUNT`,
`GCP_WIF_PROVIDER` — **none masked, because none are secrets.** They are
identifiers. Having nothing secret to store is the point.

Least privilege, deliberately: `roles/storage.objectAdmin` scoped to
**gs://harnessxml-web only** (not the project), plus a **custom role**
`harnessxmlCdnInvalidator` carrying just `compute.urlMaps.invalidateCache`.
`roles/compute.loadBalancerAdmin` would have let CI reconfigure the shared load
balancer that also serves visml.com, rumima and both collab hosts.

**⚠️ INCOMPLETE — needs `gcloud auth login`, then:** the custom role, the two
role bindings, and the `roles/iam.workloadIdentityUser` binding for
`principalSet://…/attribute.project_path/visml/harnessxml`. Pool, provider and
service account already exist.

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
