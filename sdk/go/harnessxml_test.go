package harnessxml_test

// Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	harnessxml "gitlab.com/visml/harnessxml/sdk/go"
)

const minimal = `<?xml version="1.0"?>
<harness xmlns="https://harnessxml.com/spec/1.0" id="m" specVersion="1.0">
  <nodes><node id="only" type="task" impl="noop"/></nodes>
</harness>`

func codes(t *testing.T, doc string) []string {
	t.Helper()
	_, d := harnessxml.Check([]byte(doc))
	var out []string
	for _, e := range d.Errors() {
		out = append(out, e.Code)
	}
	return out
}

func has(list []string, want string) bool {
	for _, x := range list {
		if x == want {
			return true
		}
	}
	return false
}

func mustHave(t *testing.T, doc, code string) {
	t.Helper()
	got := codes(t, doc)
	if !has(got, code) {
		t.Fatalf("expected %s, got %v", code, got)
	}
}

func repoRoot(t *testing.T) string {
	t.Helper()
	// sdk/go -> repository root
	abs, err := filepath.Abs("../..")
	if err != nil {
		t.Fatal(err)
	}
	return abs
}

func TestMinimalIsValid(t *testing.T) {
	h, err := harnessxml.LoadBytes([]byte(minimal), "<test>")
	if err != nil {
		t.Fatal(err)
	}
	if h.ID != "m" || len(h.Nodes) != 1 {
		t.Fatalf("unexpected parse: %+v", h)
	}
}

func TestPrefixedNamespaceIsAccepted(t *testing.T) {
	// §2.6 — match on namespace URI and local name, never on prefix.
	doc := `<?xml version="1.0"?>
<hx:harness xmlns:hx="https://harnessxml.com/spec/1.0" id="p" specVersion="1.0">
  <hx:nodes><hx:node id="a" type="task"/></hx:nodes>
</hx:harness>`
	if _, err := harnessxml.LoadBytes([]byte(doc), "<test>"); err != nil {
		t.Fatal(err)
	}
}

func TestDiagnosticsCarryALineNumber(t *testing.T) {
	_, d := harnessxml.Check([]byte(strings.Replace(minimal, `type="task"`, `type="quantum"`, 1)))
	for _, e := range d.Errors() {
		if e.Line <= 0 {
			t.Fatalf("finding with no location is a rule number, not a diagnostic: %+v", e)
		}
	}
}

func TestNotWellFormedIsHX1001(t *testing.T) { mustHave(t, "<harness", "HX-1001") }

func TestMissingSpecVersionIsHX1002(t *testing.T) {
	mustHave(t, strings.Replace(minimal, ` specVersion="1.0"`, "", 1), "HX-1002")
}

func TestUnknownNodeTypeIsHX1003(t *testing.T) {
	mustHave(t, strings.Replace(minimal, `type="task"`, `type="quantum"`, 1), "HX-1003")
}

func TestUnknownElementIsHX1003NotIgnored(t *testing.T) {
	mustHave(t, strings.Replace(minimal, "<nodes>", "<nodes><wibble/>", 1), "HX-1003")
}

func TestDuplicateNodeIDIsHX1101(t *testing.T) {
	doc := strings.Replace(minimal,
		`<node id="only" type="task" impl="noop"/>`,
		`<node id="a" type="task"/><node id="a" type="task"/>`, 1)
	mustHave(t, doc, "HX-1101")
}

func TestDanglingEdgeIsHX2001(t *testing.T) {
	mustHave(t, `<harness xmlns="https://harnessxml.com/spec/1.0" id="d" specVersion="1.0">
  <nodes><node id="a" type="task"/></nodes>
  <edges><edge from="a" to="ghost" type="control"/></edges>
</harness>`, "HX-2001")
}

func TestRetryOnNonIdempotentIsHX3301(t *testing.T) {
	doc := strings.Replace(minimal,
		`<node id="only" type="task" impl="noop"/>`,
		`<node id="p" type="task" idempotent="false"><retry maxAttempts="3"/></node>`, 1)
	mustHave(t, doc, "HX-3301")
}

func TestCycleIsHX3003(t *testing.T) {
	mustHave(t, `<harness xmlns="https://harnessxml.com/spec/1.0" id="c" specVersion="1.0">
  <nodes><node id="a" type="task"/><node id="b" type="task"/></nodes>
  <edges>
    <edge from="a" to="b" type="control"/>
    <edge from="b" to="a" type="control"/>
  </edges>
</harness>`, "HX-3003")
}

func TestErrorEdgesDoNotCreateACycle(t *testing.T) {
	doc := `<harness xmlns="https://harnessxml.com/spec/1.0" id="e" specVersion="1.0">
  <nodes><node id="a" type="task"/><node id="b" type="task"/></nodes>
  <edges>
    <edge from="a" to="b" type="control"/>
    <edge from="b" to="a" type="error"/>
  </edges>
</harness>`
	if has(codes(t, doc), "HX-3003") {
		t.Fatal("error edges must be excluded from the acyclicity check")
	}
}

func TestUnboundedLoopIsRejected(t *testing.T) {
	mustHave(t, `<harness xmlns="https://harnessxml.com/spec/1.0" id="u" specVersion="1.0">
  <nodes>
    <node id="l" type="loop"><loop kind="while" while="${true}"><body ref="w"/></loop></node>
    <node id="w" type="task"/>
  </nodes>
</harness>`, "HX-1001")
}

func TestLiteralCredentialIsHX3501(t *testing.T) {
	mustHave(t, `<harness xmlns="https://harnessxml.com/spec/1.0" id="l" specVersion="1.0">
  <resources>
    <resource id="m" type="model">
      <property name="apiKey" value="sk-ant-api03-AAAAAAAAAAAAAAAAAAAAAAAA"/>
    </resource>
  </resources>
  <nodes><node id="a" type="inference"><resourceRef ref="m"/></node></nodes>
</harness>`, "HX-3501")
}

func TestCredentialReferenceIsFine(t *testing.T) {
	doc := `<harness xmlns="https://harnessxml.com/spec/1.0" id="ok" specVersion="1.0">
  <resources>
    <resource id="m" type="model"><credential ref="ANTHROPIC_API_KEY" store="vault"/></resource>
  </resources>
  <nodes><node id="a" type="inference"><resourceRef ref="m"/></node></nodes>
</harness>`
	if has(codes(t, doc), "HX-3501") {
		t.Fatal("a credential REFERENCE is exactly what the format wants")
	}
}

func TestDataEdgeNeedsBothPorts(t *testing.T) {
	mustHave(t, `<harness xmlns="https://harnessxml.com/spec/1.0" id="dp" specVersion="1.0">
  <nodes>
    <node id="a" type="source"><outputs><output name="x"/></outputs></node>
    <node id="b" type="task"><inputs><input name="y"/></inputs></node>
  </nodes>
  <edges><edge from="a" to="b" type="data"/></edges>
</harness>`, "HX-2301")
}

func TestEveryShippedExampleValidates(t *testing.T) {
	root := repoRoot(t)
	var found int
	err := filepath.Walk(filepath.Join(root, "examples"), func(p string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() || !strings.HasSuffix(p, ".hxml") {
			return err
		}
		found++
		if _, e := harnessxml.Load(p); e != nil {
			t.Errorf("%s should be valid:\n%v", filepath.Base(p), e)
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if found < 5 {
		t.Fatalf("expected the shipped examples, found %d", found)
	}
}

func TestEveryInvalidFixtureIsRejectedWithItsCode(t *testing.T) {
	root := repoRoot(t)
	dir := filepath.Join(root, "conformance", "invalid")
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Skip("no conformance fixtures")
	}
	for _, e := range entries {
		if !strings.HasSuffix(e.Name(), ".hxml") {
			continue
		}
		expectedFile := filepath.Join(dir, strings.TrimSuffix(e.Name(), ".hxml")+".expected")
		want, err := os.ReadFile(expectedFile)
		if err != nil {
			continue
		}
		src, err := os.ReadFile(filepath.Join(dir, e.Name()))
		if err != nil {
			t.Fatal(err)
		}
		got := codes(t, string(src))
		code := strings.TrimSpace(string(want))
		if !has(got, code) {
			t.Errorf("%s: expected %s, got %v", e.Name(), code, got)
		}
	}
}
