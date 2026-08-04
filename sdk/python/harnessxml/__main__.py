"""`python -m harnessxml` — the SDK's command line interface.

Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0

Exit codes follow specification §14.7, so this is usable in CI:
    0  valid; warnings may have been reported
    1  invalid; at least one error
    2  the tool itself failed
"""

from __future__ import annotations

import sys
from pathlib import Path

from . import __version__, check
from .diag import Severity

USAGE = """\
harnessxml — Python SDK for the HarnessXML open specification
https://harnessxml.com/

USAGE:
    python -m harnessxml validate FILE...
    python -m harnessxml graph    FILE
    python -m harnessxml --version

EXIT CODES (specification §14.7):
    0    valid; warnings may have been reported
    1    invalid; at least one error
    2    the tool itself failed
"""


def _graph(path: str) -> int:
    from . import load
    from .diag import HarnessXMLError

    try:
        h = load(path)
    except HarnessXMLError as e:
        print(e, file=sys.stderr)
        return 1
    print(f"harness {h.id} (specVersion {h.spec_version})")
    print(f"  {len(h.nodes)} node(s), {len(h.edges)} edge(s)")
    if h.resources:
        print("\nresources")
        for r in h.resources:
            print(f"  {r.id:<20} {r.type}")
    print("\nnodes")
    for n in h.nodes:
        flags = []
        if not n.idempotent:
            flags.append("NOT-IDEMPOTENT")
        if n.retry:
            flags.append("retry")
        if n.guard:
            flags.append("guard")
        suffix = f"   [{' '.join(flags)}]" if flags else ""
        print(f"  {n.id:<22} {n.type:<12}{suffix}")
    print("\nedges")
    for e in h.edges:
        ports = f"  ({e.from_port} -> {e.to_port})" if e.from_port and e.to_port else ""
        print(f"  {e.from_:<22} --{e.type:^13}--> {e.to}{ports}")
    return 0


def main(argv: list[str] | None = None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    if not args or args[0] in ("-h", "--help"):
        print(USAGE)
        return 0 if args else 2
    if args[0] == "--version":
        print(f"harnessxml {__version__} (specification 1.0)")
        return 0

    command, files = args[0], args[1:]
    if not files:
        print(f"harnessxml: {command} needs a file", file=sys.stderr)
        return 2

    if command == "graph":
        return _graph(files[0])

    if command != "validate":
        print(f"harnessxml: unknown command '{command}'\n", file=sys.stderr)
        print(USAGE, file=sys.stderr)
        return 2

    worst = 0
    for f in files:
        try:
            diags = check(f)
        except OSError as e:
            # Exit 2, not 1: "the validator is broken" and "the workflow is
            # wrong" need different responses in CI.
            print(f"harnessxml: cannot read {f}: {e}", file=sys.stderr)
            return 2

        if diags.items:
            print(diags.report(str(Path(f))))
        errors = len(diags.errors)
        warnings = len(diags.warnings)
        if errors:
            print(f"{f}: {errors} error(s)")
            worst = max(worst, 1)
        elif warnings:
            print(f"{f}: valid ({warnings} warning(s))")
        else:
            print(f"{f}: valid")
    # A build that fails on advisory findings trains people to suppress
    # warnings, which loses the errors too (§14.7).
    return worst


if __name__ == "__main__":
    sys.exit(main())
