"""HarnessXML SDK for Python.

Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0

An implementation of the HarnessXML open specification — https://harnessxml.com/

Standard library only. A specification that promises its released versions stay
reachable forever should not need a dependency tree to read them.

    import harnessxml

    h = harnessxml.load("workflow.hxml")          # parse + validate, or raise
    print(h.id, len(h.nodes), "nodes")

    diags = harnessxml.check("workflow.hxml")     # never raises; inspect findings
    for d in diags.sorted():
        print(d.code, d.message)

Conformance level: **Core** — parse and validate. This SDK does not execute
workflows; the reference executor in `reference-runtime/` does.
"""

from __future__ import annotations

from pathlib import Path

from .build import Builder, NodeBuilder
from .diag import Diagnostic, Diagnostics, HarnessXMLError, Severity
from .model import (
    NS,
    Artifact,
    Cases,
    Edge,
    Harness,
    Loop,
    Metadata,
    Node,
    Port,
    Resource,
    Retry,
    Timeout,
    Wait,
)
from .parse import parse
from .validate import validate

__version__ = "0.1.0"
__spec_version__ = "1.0"

__all__ = [
    "NS", "Artifact", "Builder", "Cases", "Diagnostic", "Diagnostics", "Edge",
    "Harness", "HarnessXMLError", "Loop", "Metadata", "Node", "NodeBuilder",
    "Port", "Resource", "Retry", "Severity", "Timeout", "Wait",
    "check", "check_text", "load", "loads", "parse", "validate",
]


def loads(text: str, *, path: str = "<string>") -> Harness:
    """Parse and validate a document. Raises HarnessXMLError if invalid."""
    d = Diagnostics()
    h = parse(text, d)
    if h is not None:
        validate(h, d)
    if d.has_errors() or h is None:
        raise HarnessXMLError(d, path)
    return h


def load(path: str | Path) -> Harness:
    """Parse and validate a file. Raises HarnessXMLError if invalid."""
    p = Path(path)
    return loads(p.read_text(encoding="utf-8"), path=str(p))


def check_text(text: str) -> Diagnostics:
    """Validate without raising. Returns every finding, errors and warnings.

    A validator SHOULD report ALL findings rather than stopping at the first
    (§14.6) — fixing one error per build cycle is an experience implementations
    have no reason to inflict.
    """
    d = Diagnostics()
    h = parse(text, d)
    if h is not None:
        validate(h, d)
    return d


def check(path: str | Path) -> Diagnostics:
    """Validate a file without raising."""
    return check_text(Path(path).read_text(encoding="utf-8"))
