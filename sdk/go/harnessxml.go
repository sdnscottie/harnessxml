package harnessxml

// Public convenience API.
//
// Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0

import "os"

// Load reads, parses and validates a document. It returns *ValidationError if
// the document is invalid.
func Load(path string) (*Harness, error) {
	src, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	return LoadBytes(src, path)
}

// LoadBytes parses and validates a document already in memory.
func LoadBytes(src []byte, path string) (*Harness, error) {
	d := &Diagnostics{}
	h := Parse(src, d)
	if h != nil {
		Validate(h, d)
	}
	if h == nil || d.HasErrors() {
		return nil, &ValidationError{Path: path, Diagnostics: d}
	}
	return h, nil
}

// Check validates without returning an error, so callers can inspect every
// finding including warnings.
//
// A validator SHOULD report ALL findings rather than stopping at the first
// (§14.6) — fixing one error per build cycle is an experience implementations
// have no reason to inflict.
func Check(src []byte) (*Harness, *Diagnostics) {
	d := &Diagnostics{}
	h := Parse(src, d)
	if h != nil {
		Validate(h, d)
	}
	return h, d
}

// CheckFile validates a file without returning an error for invalidity.
func CheckFile(path string) (*Harness, *Diagnostics, error) {
	src, err := os.ReadFile(path)
	if err != nil {
		return nil, nil, err
	}
	h, d := Check(src)
	return h, d, nil
}
