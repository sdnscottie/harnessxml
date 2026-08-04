//! Diagnostics.
//!
//! Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0
//!
//! Specification §13.5 and §14.4: every finding carries a code, a location, and
//! a message naming the specific offending value. The code is part of
//! conformance — two validators that reject the same document for
//! differently-stated reasons give their users incompatible diagnostics.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        })
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub line: usize,
    pub message: String,
}

impl Diagnostic {
    pub fn error(code: &'static str, line: usize, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Error,
            line,
            message: message.into(),
        }
    }

    pub fn warning(code: &'static str, line: usize, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Warning,
            line,
            message: message.into(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn push(&mut self, d: Diagnostic) {
        self.items.push(d);
    }

    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.items.iter().filter(|d| d.severity == Severity::Error)
    }

    pub fn has_errors(&self) -> bool {
        self.errors().next().is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Sorted for stable output: errors before warnings, then by line. A
    /// validator whose output order varies between runs is one nobody can diff.
    pub fn sorted(&self) -> Vec<&Diagnostic> {
        let mut v: Vec<&Diagnostic> = self.items.iter().collect();
        v.sort_by_key(|d| (d.severity == Severity::Warning, d.line, d.code));
        v
    }

    pub fn report(&self, path: &str) -> String {
        let mut out = String::new();
        for d in self.sorted() {
            out.push_str(&format!(
                "{}  {}:{}  {}\n         {}\n",
                d.code, path, d.line, d.severity, d.message
            ));
        }
        out
    }
}
