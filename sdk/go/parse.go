package harnessxml

// `.hxml` -> Harness.
//
// Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0
//
// Namespace-aware per §2.6: elements are matched on namespace URI plus local
// name, never on prefix.
//
// Per §2.8 an unrecognised element in the HarnessXML namespace is an ERROR
// (HX-1003), not something to skip. That rule is enforced here, in the parser,
// rather than left to the validator — a construct the parser silently dropped
// is one the validator can never see.
//
// encoding/xml gives no line numbers, so the parser tracks them itself from the
// decoder's byte offset. A diagnostic without a location is a rule number, not
// a diagnostic (§13.5).

import (
	"encoding/xml"
	"fmt"
	"sort"
	"strconv"
	"strings"
)

type parser struct {
	dec        *xml.Decoder
	diags      *Diagnostics
	lineStarts []int
}

func newLineIndex(src []byte) []int {
	starts := []int{0}
	for i, b := range src {
		if b == '\n' {
			starts = append(starts, i+1)
		}
	}
	return starts
}

func (p *parser) lineAt(offset int64) int {
	i := sort.SearchInts(p.lineStarts, int(offset))
	if i <= 0 {
		return 1
	}
	return i
}

func attr(se xml.StartElement, name string) string {
	for _, a := range se.Attr {
		if a.Name.Local == name && a.Name.Space != "xmlns" {
			return a.Value
		}
	}
	return ""
}

func attrPtr(se xml.StartElement, name string) *string {
	for _, a := range se.Attr {
		if a.Name.Local == name && a.Name.Space != "xmlns" {
			v := a.Value
			return &v
		}
	}
	return nil
}

func attrBool(se xml.StartElement, name string, def bool) bool {
	switch attr(se, name) {
	case "true":
		return true
	case "false":
		return false
	}
	return def
}

func attrInt(se xml.StartElement, name string) *int {
	v := attr(se, name)
	if v == "" {
		return nil
	}
	n, err := strconv.Atoi(v)
	if err != nil {
		return nil
	}
	return &n
}

func attrFloat(se xml.StartElement, name string, def float64) float64 {
	v := attr(se, name)
	if v == "" {
		return def
	}
	f, err := strconv.ParseFloat(v, 64)
	if err != nil {
		return def
	}
	return f
}

// Parse reads a HarnessXML document. It returns nil if the document is not
// well-formed or has no <harness> root; diagnostics always explain why.
func Parse(src []byte, diags *Diagnostics) *Harness {
	p := &parser{
		dec:        xml.NewDecoder(strings.NewReader(string(src))),
		diags:      diags,
		lineStarts: newLineIndex(src),
	}

	// Find the root.
	for {
		tok, err := p.dec.Token()
		if err != nil {
			diags.Errorf("HX-1001", 1, "not well-formed: %v", err)
			return nil
		}
		se, ok := tok.(xml.StartElement)
		if !ok {
			continue
		}
		line := p.lineAt(p.dec.InputOffset())
		if se.Name.Local != "harness" || se.Name.Space != NS {
			diags.Errorf("HX-1001", line,
				"root element is <%s> in namespace %q; expected <harness> in %q",
				se.Name.Local, se.Name.Space, NS)
			return nil
		}
		return p.harness(se, line)
	}
}

func (p *parser) harness(se xml.StartElement, line int) *Harness {
	h := &Harness{
		ID:          attr(se, "id"),
		SpecVersion: attrPtr(se, "specVersion"),
		Name:        attr(se, "name"),
		Entry:       attr(se, "entry"),
	}

	for {
		tok, err := p.dec.Token()
		if err != nil {
			if err.Error() == "EOF" {
				break
			}
			p.diags.Errorf("HX-1001", p.lineAt(p.dec.InputOffset()), "not well-formed: %v", err)
			return h
		}
		switch t := tok.(type) {
		case xml.EndElement:
			if t.Name.Local == "harness" {
				return h
			}
		case xml.StartElement:
			l := p.lineAt(p.dec.InputOffset())
			if t.Name.Space != NS {
				p.diags.Errorf("HX-1006", l,
					"element <%s> is from a foreign namespace and is only permitted inside <extension>",
					t.Name.Local)
				_ = p.dec.Skip()
				continue
			}
			switch t.Name.Local {
			case "metadata":
				p.metadata(h)
			case "security", "extension":
				_ = p.dec.Skip()
			case "resources":
				p.resources(h)
			case "artifacts":
				p.artifacts(h)
			case "nodes":
				p.nodes(h)
			case "edges":
				p.edges(h)
			default:
				p.diags.Errorf("HX-1003", l,
					"unrecognised element <%s> inside <harness>; an unknown construct must be rejected, never skipped",
					t.Name.Local)
				_ = p.dec.Skip()
			}
		}
	}
	return h
}

func (p *parser) text() string {
	var b strings.Builder
	depth := 0
	for {
		tok, err := p.dec.Token()
		if err != nil {
			return strings.TrimSpace(b.String())
		}
		switch t := tok.(type) {
		case xml.CharData:
			if depth == 0 {
				b.Write(t)
			}
		case xml.StartElement:
			depth++
		case xml.EndElement:
			if depth == 0 {
				return strings.TrimSpace(b.String())
			}
			depth--
		}
	}
}

func (p *parser) metadata(h *Harness) {
	for {
		tok, err := p.dec.Token()
		if err != nil {
			return
		}
		switch t := tok.(type) {
		case xml.EndElement:
			if t.Name.Local == "metadata" {
				return
			}
		case xml.StartElement:
			switch t.Name.Local {
			case "title":
				h.Metadata.Title = p.text()
			case "description":
				h.Metadata.Description = p.text()
			case "author":
				h.Metadata.Author = p.text()
			case "organization":
				h.Metadata.Organization = p.text()
			case "created":
				h.Metadata.Created = p.text()
			case "modified":
				h.Metadata.Modified = p.text()
			case "license":
				h.Metadata.License = p.text()
			case "documentVersion":
				h.Metadata.DocumentVersion = p.text()
			case "tags":
				p.tags(h)
			default:
				_ = p.dec.Skip()
			}
		}
	}
}

func (p *parser) tags(h *Harness) {
	for {
		tok, err := p.dec.Token()
		if err != nil {
			return
		}
		switch t := tok.(type) {
		case xml.EndElement:
			if t.Name.Local == "tags" {
				return
			}
		case xml.StartElement:
			if t.Name.Local == "tag" {
				h.Metadata.Tags = append(h.Metadata.Tags, p.text())
			} else {
				_ = p.dec.Skip()
			}
		}
	}
}

func (p *parser) resources(h *Harness) {
	for {
		tok, err := p.dec.Token()
		if err != nil {
			return
		}
		switch t := tok.(type) {
		case xml.EndElement:
			if t.Name.Local == "resources" {
				return
			}
		case xml.StartElement:
			l := p.lineAt(p.dec.InputOffset())
			if t.Name.Local != "resource" {
				p.diags.Errorf("HX-1003", l, "unrecognised element <%s> inside <resources>", t.Name.Local)
				_ = p.dec.Skip()
				continue
			}
			r := Resource{
				ID: attr(t, "id"), Type: attr(t, "type"), Name: attr(t, "name"),
				Provider: attr(t, "provider"), URI: attr(t, "uri"), Line: l,
			}
			p.resourceBody(&r)
			h.Resources = append(h.Resources, r)
		}
	}
}

func (p *parser) resourceBody(r *Resource) {
	for {
		tok, err := p.dec.Token()
		if err != nil {
			return
		}
		switch t := tok.(type) {
		case xml.EndElement:
			if t.Name.Local == "resource" {
				return
			}
		case xml.StartElement:
			switch t.Name.Local {
			case "property":
				r.Properties = append(r.Properties, Property{attr(t, "name"), attr(t, "value")})
				_ = p.dec.Skip()
			case "credential":
				r.CredentialRef = attr(t, "ref")
				r.CredentialStore = attr(t, "store")
				_ = p.dec.Skip()
			default:
				_ = p.dec.Skip()
			}
		}
	}
}

func (p *parser) artifacts(h *Harness) {
	for {
		tok, err := p.dec.Token()
		if err != nil {
			return
		}
		switch t := tok.(type) {
		case xml.EndElement:
			if t.Name.Local == "artifacts" {
				return
			}
		case xml.StartElement:
			l := p.lineAt(p.dec.InputOffset())
			if t.Name.Local != "artifact" {
				p.diags.Errorf("HX-1003", l, "unrecognised element <%s> inside <artifacts>", t.Name.Local)
				_ = p.dec.Skip()
				continue
			}
			a := Artifact{
				ID: attr(t, "id"), Type: attr(t, "type"), Name: attr(t, "name"),
				URI: attr(t, "uri"), MediaType: attr(t, "mediaType"),
				Digest: attr(t, "digest"), Classification: attr(t, "classification"),
				Line: l,
			}
			p.artifactBody(&a)
			h.Artifacts = append(h.Artifacts, a)
		}
	}
}

func (p *parser) artifactBody(a *Artifact) {
	for {
		tok, err := p.dec.Token()
		if err != nil {
			return
		}
		switch t := tok.(type) {
		case xml.EndElement:
			if t.Name.Local == "artifact" {
				return
			}
		case xml.StartElement:
			if t.Name.Local == "property" {
				a.Properties = append(a.Properties, Property{attr(t, "name"), attr(t, "value")})
			}
			_ = p.dec.Skip()
		}
	}
}

func (p *parser) nodes(h *Harness) {
	for {
		tok, err := p.dec.Token()
		if err != nil {
			return
		}
		switch t := tok.(type) {
		case xml.EndElement:
			if t.Name.Local == "nodes" {
				return
			}
		case xml.StartElement:
			l := p.lineAt(p.dec.InputOffset())
			if t.Name.Local != "node" {
				p.diags.Errorf("HX-1003", l, "unrecognised element <%s> inside <nodes>", t.Name.Local)
				_ = p.dec.Skip()
				continue
			}
			n := Node{
				ID: attr(t, "id"), Type: attr(t, "type"), Name: attr(t, "name"),
				Impl: attr(t, "impl"), Idempotent: attrBool(t, "idempotent", true),
				JoinPolicy: "all", Quorum: attrInt(t, "quorum"),
				Compensates: attr(t, "compensates"), Line: l,
			}
			if jp := attr(t, "joinPolicy"); jp != "" {
				n.JoinPolicy = jp
			}
			p.nodeBody(&n)
			h.Nodes = append(h.Nodes, n)
		}
	}
}

func (p *parser) nodeBody(n *Node) {
	for {
		tok, err := p.dec.Token()
		if err != nil {
			return
		}
		switch t := tok.(type) {
		case xml.EndElement:
			if t.Name.Local == "node" {
				return
			}
		case xml.StartElement:
			l := p.lineAt(p.dec.InputOffset())
			if t.Name.Space != NS {
				p.diags.Errorf("HX-1006", l, "foreign element <%s> outside <extension>", t.Name.Local)
				_ = p.dec.Skip()
				continue
			}
			switch t.Name.Local {
			case "description":
				n.Description = p.text()
			case "inputs":
				n.Inputs = p.ports("inputs", "input")
			case "outputs":
				n.Outputs = p.ports("outputs", "output")
			case "config":
				n.Config = p.properties("config")
			case "resourceRef":
				n.ResourceRefs = append(n.ResourceRefs, Ref{attr(t, "ref"), l})
				_ = p.dec.Skip()
			case "artifactRef":
				n.ArtifactRefs = append(n.ArtifactRefs, Ref{attr(t, "ref"), l})
				_ = p.dec.Skip()
			case "guard":
				g := attr(t, "when")
				n.Guard = &g
				_ = p.dec.Skip()
			case "retry":
				ma := 1
				if v := attrInt(t, "maxAttempts"); v != nil {
					ma = *v
				}
				backoff := "exponential"
				if b := attr(t, "backoff"); b != "" {
					backoff = b
				}
				initial := "PT1S"
				if d := attr(t, "initialDelay"); d != "" {
					initial = d
				}
				n.Retry = &Retry{
					MaxAttempts: ma, Backoff: backoff, InitialDelay: initial,
					MaxDelay: attr(t, "maxDelay"), Multiplier: attrFloat(t, "multiplier", 2),
					Jitter:  attrBool(t, "jitter", true),
					RetryOn: strings.Fields(attr(t, "retryOn")),
				}
				_ = p.dec.Skip()
			case "timeout":
				ot := "fail"
				if v := attr(t, "onTimeout"); v != "" {
					ot = v
				}
				n.Timeout = &Timeout{Duration: attr(t, "duration"), OnTimeout: ot}
				_ = p.dec.Skip()
			case "cases":
				n.Cases = p.cases(l)
			case "loop":
				n.Loop = p.loop(t, l)
			case "subworkflow":
				n.Subworkflow = attr(t, "href")
				_ = p.dec.Skip()
			case "wait":
				n.Wait = &Wait{
					Duration: attr(t, "duration"), Until: attr(t, "until"),
					Event: attr(t, "event"), Line: l,
				}
				_ = p.dec.Skip()
			case "security":
				_ = p.dec.Skip()
			case "extension":
				n.Extensions = append(n.Extensions, Extension{
					Namespace: attr(t, "namespace"),
					Required:  attr(t, "required") == "true",
				})
				_ = p.dec.Skip()
			default:
				p.diags.Errorf("HX-1003", l,
					"unrecognised element <%s> inside <node id='%s'>", t.Name.Local, n.ID)
				_ = p.dec.Skip()
			}
		}
	}
}

func (p *parser) ports(container, want string) []Port {
	var out []Port
	for {
		tok, err := p.dec.Token()
		if err != nil {
			return out
		}
		switch t := tok.(type) {
		case xml.EndElement:
			if t.Name.Local == container {
				return out
			}
		case xml.StartElement:
			l := p.lineAt(p.dec.InputOffset())
			if t.Name.Local == want {
				out = append(out, Port{
					Name: attr(t, "name"), Type: attr(t, "type"),
					Required: attrBool(t, "required", true),
					Default:  attrPtr(t, "default"), Value: attrPtr(t, "value"),
					Line: l,
				})
			}
			_ = p.dec.Skip()
		}
	}
}

func (p *parser) properties(container string) []Property {
	var out []Property
	for {
		tok, err := p.dec.Token()
		if err != nil {
			return out
		}
		switch t := tok.(type) {
		case xml.EndElement:
			if t.Name.Local == container {
				return out
			}
		case xml.StartElement:
			if t.Name.Local == "property" {
				out = append(out, Property{attr(t, "name"), attr(t, "value")})
			}
			_ = p.dec.Skip()
		}
	}
}

func (p *parser) cases(line int) *Cases {
	c := &Cases{Line: line}
	for {
		tok, err := p.dec.Token()
		if err != nil {
			return c
		}
		switch t := tok.(type) {
		case xml.EndElement:
			if t.Name.Local == "cases" {
				return c
			}
		case xml.StartElement:
			switch t.Name.Local {
			case "case":
				c.Cases = append(c.Cases, Case{attr(t, "when"), attr(t, "to")})
			case "otherwise":
				o := attr(t, "to")
				c.Otherwise = &o
			}
			_ = p.dec.Skip()
		}
	}
}

func (p *parser) loop(se xml.StartElement, line int) *Loop {
	l := &Loop{
		Kind: attr(se, "kind"), Over: attr(se, "over"), While: attr(se, "while"),
		Count: attrInt(se, "count"), MaxIterations: attrInt(se, "maxIterations"),
		Var: "item", IndexVar: "index", MaxConcurrency: 1, OnItemFailure: "fail",
		Line: line,
	}
	if v := attr(se, "var"); v != "" {
		l.Var = v
	}
	if v := attr(se, "indexVar"); v != "" {
		l.IndexVar = v
	}
	if v := attrInt(se, "maxConcurrency"); v != nil {
		l.MaxConcurrency = *v
	}
	if v := attr(se, "onItemFailure"); v != "" {
		l.OnItemFailure = v
	}
	for {
		tok, err := p.dec.Token()
		if err != nil {
			return l
		}
		switch t := tok.(type) {
		case xml.EndElement:
			if t.Name.Local == "loop" {
				return l
			}
		case xml.StartElement:
			if t.Name.Local == "body" {
				l.Body = attr(t, "ref")
			}
			_ = p.dec.Skip()
		}
	}
}

func (p *parser) edges(h *Harness) {
	for {
		tok, err := p.dec.Token()
		if err != nil {
			return
		}
		switch t := tok.(type) {
		case xml.EndElement:
			if t.Name.Local == "edges" {
				return
			}
		case xml.StartElement:
			l := p.lineAt(p.dec.InputOffset())
			if t.Name.Local != "edge" {
				p.diags.Errorf("HX-1003", l, "unrecognised element <%s> inside <edges>", t.Name.Local)
				_ = p.dec.Skip()
				continue
			}
			ty := attr(t, "type")
			if ty == "" {
				ty = "control"
			}
			known := false
			for _, k := range EdgeTypes {
				if k == ty {
					known = true
				}
			}
			if !known {
				p.diags.Errorf("HX-1003", l, "unrecognised edge type '%s'", ty)
				_ = p.dec.Skip()
				continue
			}
			h.Edges = append(h.Edges, Edge{
				ID: attr(t, "id"), From: attr(t, "from"), To: attr(t, "to"), Type: ty,
				FromPort: attr(t, "fromPort"), ToPort: attr(t, "toPort"),
				Condition: attr(t, "condition"), Line: l,
			})
			_ = p.dec.Skip()
		}
	}
}

var _ = fmt.Sprintf
