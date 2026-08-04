#!/usr/bin/env python3
"""HarnessXML conformance runner.

Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0

Runs the conformance corpus against a validator. Two modes:

  * default — validate with the bundled schema check (Core level, structural
    layer only). Needs `lxml`.
  * --cmd   — shell out to any validator, in any language. The command is given
    a document path and must exit 0 for valid, non-zero for invalid, and print
    the HX-nnnn code for an invalid document.

    python3 conformance/validate.py
    python3 conformance/validate.py --cmd "harnessxml validate"
    python3 conformance/validate.py --cmd "my-validator --quiet"

Fixture layout:

    conformance/valid/*.hxml          MUST be accepted
    conformance/invalid/*.hxml        MUST be rejected
    conformance/invalid/*.expected    the required HX-nnnn code
    examples/**/*.hxml                also treated as valid fixtures

Exit codes follow the specification's own convention (chapter 14 §14.7):
    0  all fixtures behaved as required
    1  at least one fixture did not
    2  the runner itself could not run
"""

from __future__ import annotations

import argparse
import re
import shlex
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CONFORMANCE = ROOT / "conformance"
SCHEMA = ROOT / "schema" / "v1.0" / "harnessxml-1.0.xsd"

GREEN, RED, YELLOW, DIM, RESET = "\033[32m", "\033[31m", "\033[33m", "\033[2m", "\033[0m"


def collect_valid() -> list[Path]:
    files = sorted((CONFORMANCE / "valid").glob("*.hxml")) if (CONFORMANCE / "valid").exists() else []
    files += sorted((ROOT / "examples").rglob("*.hxml"))
    return files


def collect_invalid() -> list[tuple[Path, str | None]]:
    out: list[tuple[Path, str | None]] = []
    d = CONFORMANCE / "invalid"
    if not d.exists():
        return out
    for f in sorted(d.glob("*.hxml")):
        expected_file = f.with_suffix(".expected")
        expected = expected_file.read_text(encoding="utf-8").strip() if expected_file.exists() else None
        out.append((f, expected))
    return out


class SchemaValidator:
    """The bundled structural validator — schema layer only.

    This is deliberately NOT a full Core-conformant validator: it implements
    layer 1 of chapter 13 and none of layer 2. Fixtures asserting a semantic
    code (HX-3xxx) are reported as SKIP rather than silently passing, because a
    runner that quietly counts unimplemented rules as successes is worse than no
    runner at all.
    """

    implemented_prefixes = ("HX-1", "HX-2")

    def __init__(self) -> None:
        try:
            from lxml import etree
        except ImportError:
            print("error: the bundled validator needs lxml "
                  "(pip install lxml), or use --cmd", file=sys.stderr)
            raise SystemExit(2)
        self.etree = etree
        if not SCHEMA.exists():
            print(f"error: schema not found at {SCHEMA}", file=sys.stderr)
            raise SystemExit(2)
        self.schema = etree.XMLSchema(etree.parse(str(SCHEMA)))

    # Map an XSD engine message onto the specification's own code, so that even
    # the structural layer reports the code a fixture asserts rather than a
    # generic "schema failed". Order matters — first match wins.
    # lxml qualifies the constraint name, e.g. keyref '{https://…/spec/1.0}edgeToRef'
    _KEYREF = r"keyref ['\"](?:\{[^}]*\})?"
    MESSAGE_CODES = [
        (re.compile(_KEYREF + r"(?:edgeFromRef|edgeToRef|caseToRef|otherwiseToRef|loopBodyRef)"), "HX-2001"),
        (re.compile(_KEYREF + r"resourceRefRef"), "HX-2002"),
        (re.compile(_KEYREF + r"artifactRefRef"), "HX-2003"),
        (re.compile(r"Duplicate key-sequence"), "HX-1101"),
        (re.compile(r"attribute ['\"]specVersion['\"] is required"), "HX-1002"),
    ]

    def check(self, path: Path) -> tuple[bool, str]:
        try:
            doc = self.etree.parse(str(path))
        except Exception as e:
            return False, f"HX-1001 not well-formed: {e}"
        if self.schema.validate(doc):
            return True, ""
        err = self.schema.error_log[0] if len(self.schema.error_log) else None
        message = err.message if err else "schema validation failed"
        code = "HX-1001"
        for pattern, mapped in self.MESSAGE_CODES:
            if pattern.search(message):
                code = mapped
                break
        return False, f"{code} {message}"


class CommandValidator:
    implemented_prefixes = ("HX-",)

    def __init__(self, cmd: str) -> None:
        self.cmd = shlex.split(cmd)

    def check(self, path: Path) -> tuple[bool, str]:
        try:
            p = subprocess.run(self.cmd + [str(path)], capture_output=True, text=True, timeout=120)
        except FileNotFoundError:
            print(f"error: validator not found: {self.cmd[0]}", file=sys.stderr)
            raise SystemExit(2)
        except subprocess.TimeoutExpired:
            return False, "validator timed out"
        if p.returncode == 2:
            print(f"error: validator reported a tool failure on {path}:\n{p.stderr}", file=sys.stderr)
            raise SystemExit(2)
        return p.returncode == 0, (p.stdout + p.stderr).strip()


CODE_RE = re.compile(r"\bHX-\d{4}\b")


def main() -> int:
    ap = argparse.ArgumentParser(description="Run the HarnessXML conformance suite")
    ap.add_argument("--cmd", help="validator command; the document path is appended")
    ap.add_argument("-v", "--verbose", action="store_true")
    args = ap.parse_args()

    validator = CommandValidator(args.cmd) if args.cmd else SchemaValidator()
    label = args.cmd if args.cmd else "bundled schema validator (structural layer only)"
    print(f"{DIM}validator: {label}{RESET}\n")

    passed = failed = skipped = 0

    print("valid fixtures — MUST be accepted")
    for f in collect_valid():
        ok, msg = validator.check(f)
        rel = f.relative_to(ROOT)
        if ok:
            passed += 1
            if args.verbose:
                print(f"  {GREEN}PASS{RESET} {rel}")
        else:
            failed += 1
            print(f"  {RED}FAIL{RESET} {rel}\n       rejected, but should be valid: {msg[:200]}")
    print(f"  {DIM}{len(collect_valid())} fixture(s){RESET}\n")

    invalid = collect_invalid()
    if invalid:
        print("invalid fixtures — MUST be rejected with the specified code")
        for f, expected in invalid:
            rel = f.relative_to(ROOT)
            if expected and not expected.startswith(validator.implemented_prefixes):
                skipped += 1
                print(f"  {YELLOW}SKIP{RESET} {rel} — {expected} is outside this validator's layer")
                continue
            ok, msg = validator.check(f)
            if ok:
                failed += 1
                print(f"  {RED}FAIL{RESET} {rel}\n       accepted, but should be rejected ({expected})")
                continue
            found = CODE_RE.findall(msg)
            if expected and expected not in found:
                failed += 1
                print(f"  {RED}FAIL{RESET} {rel}\n"
                      f"       rejected, but with {found or ['no code']} — expected {expected}")
            else:
                passed += 1
                if args.verbose:
                    print(f"  {GREEN}PASS{RESET} {rel} ({expected or 'rejected'})")
        print()
    else:
        print(f"{YELLOW}no invalid fixtures found — the corpus is incomplete{RESET}\n")

    print(f"{'-' * 52}")
    print(f"passed {passed}   failed {failed}   skipped {skipped}")
    if skipped:
        print(f"{DIM}skipped fixtures assert rules outside this validator's layer;{RESET}")
        print(f"{DIM}they are NOT successes. Use --cmd with a full validator.{RESET}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
