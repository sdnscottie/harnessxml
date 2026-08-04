# Contributing to HarnessXML

Thank you for considering it. HarnessXML is only worth calling a specification
if people outside VisML shape it.

## The short version

| you want to | do this |
|---|---|
| report an ambiguity in the spec | open an issue — ambiguity is a **bug**, and the most valuable kind |
| fix a typo or clarify wording | pull request, no proposal needed |
| add or change normative behaviour | write an **HXEP** — see `GOVERNANCE.md` §3 |
| report an implementation bug | issue against `reference-runtime/` |
| contribute a conformance test | pull request to `conformance/` — always welcome |
| add an example workflow | pull request to `examples/` |

## Ambiguity is the highest-value bug report

If two competent engineers can read a sentence in the specification and build
incompatible implementations, that sentence is broken — even if both readings
are reasonable. Please report it, and say what you thought it meant and what
the other reading is. This is worth more than a feature request.

## Licensing of contributions

By contributing you agree that:

- contributions to the **specification text** (`spec/`, `site/content/`) are
  licensed under **CC BY 4.0**;
- contributions to **code, schemas, examples and tests** are licensed under
  **Apache License 2.0**;
- you have the right to contribute the material.

There is no CLA to sign. The Apache-2.0 inbound licence includes the patent
grant in section 5, which is the protection a CLA would otherwise be needed
for. If your employer owns your work, make sure you have their permission
before contributing — that is the one thing this project cannot check for you.

## Coding standards

**Rust** (`reference-runtime/`)
- edition 2024; `cargo fmt` and `cargo clippy -- -D warnings` must be clean
- every validation rule carries its `HX-nnnn` error code as a constant, and the
  code appears in both the specification and the test fixture that triggers it
- no `unwrap()` on anything derived from a parsed document — a malformed input
  must produce a diagnostic, never a panic

**Python** (`site/build.py`)
- standard library only. The site must build on a bare `python3` with no
  `pip install`, because a documentation site that cannot be rebuilt in ten
  years is not an archive
- no network access at build time

**XML**
- schema changes require the examples to still validate: `python3 conformance/validate.py`
- two-space indent, attributes on one line until it hurts, then one per line

## Specification writing style

- **RFC 2119 keywords** (MUST, MUST NOT, SHOULD, MAY) in capitals, and only
  where a conformance requirement is genuinely meant
- every normative rule gets an error code, and every error code gets a test
- every construct gets a worked example that is a real, runnable document —
  not a fragment with an ellipsis in it
- prefer stating the failure a rule prevents over restating the rule
- no forward references to undefined terms; the glossary is normative

## Tests

```bash
python3 conformance/validate.py          # every example against the XSD
cd reference-runtime && cargo test       # parser + validator + lifecycle
python3 site/build.py --check            # build the site, verify internal links
```

A pull request that changes behaviour without a test will be asked for one
before review, not after.

## Review

Editorial fixes: reviewed by whoever is available. Normative changes: the HXEP
process in `GOVERNANCE.md`, including the 30-day public review minimum. Nobody
gets to skip that, VisML included.

## Conduct

See `CODE_OF_CONDUCT.md`. Short form: argue about the specification as hard as
you like, and never about the person.
