package harnessxml

// Diagnostics — specification §13.5 and §14.4.
//
// Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0
//
// Every finding carries a code, a location, and a message naming the specific
// offending value. The code is part of conformance: two validators that reject
// the same document for differently-stated reasons give their users
// incompatible diagnostics, and an author cannot then move between tools.

import (
	"fmt"
	"sort"
	"strings"
)

type Severity int

const (
	Error Severity = iota
	Warning
)

func (s Severity) String() string {
	if s == Warning {
		return "warning"
	}
	return "error"
}

type Diagnostic struct {
	Code     string
	Severity Severity
	Line     int
	Message  string
}

func (d Diagnostic) String() string {
	return fmt.Sprintf("%s  line %d  %s\n         %s", d.Code, d.Line, d.Severity, d.Message)
}

type Diagnostics struct {
	Items []Diagnostic
}

func (d *Diagnostics) Errorf(code string, line int, format string, args ...any) {
	d.Items = append(d.Items, Diagnostic{code, Error, line, fmt.Sprintf(format, args...)})
}

func (d *Diagnostics) Warnf(code string, line int, format string, args ...any) {
	d.Items = append(d.Items, Diagnostic{code, Warning, line, fmt.Sprintf(format, args...)})
}

func (d *Diagnostics) Errors() []Diagnostic {
	var out []Diagnostic
	for _, x := range d.Items {
		if x.Severity == Error {
			out = append(out, x)
		}
	}
	return out
}

func (d *Diagnostics) Warnings() []Diagnostic {
	var out []Diagnostic
	for _, x := range d.Items {
		if x.Severity == Warning {
			out = append(out, x)
		}
	}
	return out
}

func (d *Diagnostics) HasErrors() bool { return len(d.Errors()) > 0 }

// Sorted returns errors before warnings, then by line. A validator whose output
// order varies between runs is one nobody can diff.
func (d *Diagnostics) Sorted() []Diagnostic {
	out := make([]Diagnostic, len(d.Items))
	copy(out, d.Items)
	sort.SliceStable(out, func(i, j int) bool {
		if (out[i].Severity == Warning) != (out[j].Severity == Warning) {
			return out[i].Severity != Warning
		}
		if out[i].Line != out[j].Line {
			return out[i].Line < out[j].Line
		}
		return out[i].Code < out[j].Code
	})
	return out
}

func (d *Diagnostics) Report(path string) string {
	var b strings.Builder
	for _, x := range d.Sorted() {
		fmt.Fprintf(&b, "%s  %s:%d  %s\n         %s\n", x.Code, path, x.Line, x.Severity, x.Message)
	}
	return b.String()
}

// ValidationError is returned by Load and Parse when a document is invalid.
type ValidationError struct {
	Path        string
	Diagnostics *Diagnostics
}

func (e *ValidationError) Error() string {
	return fmt.Sprintf("%s: %d error(s)\n%s", e.Path, len(e.Diagnostics.Errors()),
		e.Diagnostics.Report(e.Path))
}
