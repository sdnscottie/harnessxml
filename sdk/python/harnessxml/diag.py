"""Diagnostics — specification §13.5 and §14.4.

Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0

Every finding carries a code, a location, and a message naming the specific
offending value. The code is part of conformance: two validators that reject the
same document for differently-stated reasons give their users incompatible
diagnostics, and an author cannot then move between tools.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum


class Severity(Enum):
    ERROR = "error"
    WARNING = "warning"


@dataclass
class Diagnostic:
    code: str
    severity: Severity
    line: int
    message: str

    def __str__(self) -> str:
        return f"{self.code}  line {self.line}  {self.severity.value}\n         {self.message}"


@dataclass
class Diagnostics:
    items: list[Diagnostic] = field(default_factory=list)

    def error(self, code: str, line: int, message: str) -> None:
        self.items.append(Diagnostic(code, Severity.ERROR, line, message))

    def warning(self, code: str, line: int, message: str) -> None:
        self.items.append(Diagnostic(code, Severity.WARNING, line, message))

    @property
    def errors(self) -> list[Diagnostic]:
        return [d for d in self.items if d.severity is Severity.ERROR]

    @property
    def warnings(self) -> list[Diagnostic]:
        return [d for d in self.items if d.severity is Severity.WARNING]

    def has_errors(self) -> bool:
        return bool(self.errors)

    def sorted(self) -> list[Diagnostic]:
        """Errors before warnings, then by line.

        A validator whose output order varies between runs is one nobody can
        diff.
        """
        return sorted(
            self.items,
            key=lambda d: (d.severity is Severity.WARNING, d.line, d.code),
        )

    def report(self, path: str) -> str:
        out = []
        for d in self.sorted():
            out.append(
                f"{d.code}  {path}:{d.line}  {d.severity.value}\n         {d.message}"
            )
        return "\n".join(out)


class HarnessXMLError(Exception):
    """Raised by the convenience API when a document is invalid."""

    def __init__(self, diagnostics: Diagnostics, path: str = "<document>") -> None:
        self.diagnostics = diagnostics
        self.path = path
        n = len(diagnostics.errors)
        super().__init__(f"{path}: {n} error(s)\n{diagnostics.report(path)}")
