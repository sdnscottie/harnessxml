// Package harnessxml is a Go SDK for the HarnessXML open specification —
// https://harnessxml.com/
//
// Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0
//
// Standard library only. A specification that promises its released versions
// stay reachable forever should not need a dependency tree to read them.
//
// Conformance level: Core — parse and validate. This SDK does not execute
// workflows; the reference executor does.
package harnessxml

// NS is the HarnessXML 1.0 namespace. It changes only at a MAJOR version: a
// namespace change breaks every existing document, which is exactly what a
// minor version promises not to do.
const NS = "https://harnessxml.com/spec/1.0"

// SpecVersion is the specification version this SDK implements.
const SpecVersion = "1.0"

// Version is the SDK's own version, distinct from SpecVersion.
const Version = "0.1.0"

// NodeTypes is a CLOSED enumeration. A runtime meeting an unrecognised type
// MUST reject the document (HX-1003) — never skip the node.
var NodeTypes = []string{
	"task", "inference", "transform", "decision", "loop", "parallel",
	"barrier", "subworkflow", "source", "sink", "wait", "human",
}

// EdgeTypes are the five typed relationships. The type determines what the
// scheduler does; these are semantics, not diagram styling.
var EdgeTypes = []string{"control", "data", "dependency", "error", "compensation"}

// IsForwardEdge reports whether an edge participates in acyclicity (HX-3003)
// and forward reachability. error and compensation do not: a handler may
// legitimately point backwards, and compensation points backwards by
// definition.
func IsForwardEdge(t string) bool {
	return t == "control" || t == "data" || t == "dependency"
}

// Port is a named, optionally typed input or output. Ports are matched BY NAME,
// never by position, so adding one does not silently rebind existing wiring.
type Port struct {
	Name     string
	Type     string
	Required bool
	Default  *string
	Value    *string
	Line     int
}

func (p Port) HasValue() bool   { return p.Value != nil }
func (p Port) HasDefault() bool { return p.Default != nil }

// Retry is a retry policy (§8.1). A nil *Retry means ONE attempt.
type Retry struct {
	MaxAttempts  int
	Backoff      string
	InitialDelay string
	MaxDelay     string
	Multiplier   float64
	Jitter       bool
	// RetryOn lists error classes to retry on. Empty means "retry any
	// failure", which is convenient and usually wrong (§8.1.2).
	RetryOn []string
}

// Timeout bounds a SINGLE ATTEMPT, not the node's total lifetime (§8.2).
type Timeout struct {
	Duration  string
	OnTimeout string
}

// Case is one branch of a decision. Cases are evaluated IN DOCUMENT ORDER and
// the first true one wins — the order is normative, so a decision is
// deterministic and explainable.
type Case struct {
	When string
	To   string
}

type Cases struct {
	Cases     []Case
	Otherwise *string
	Line      int
}

type Loop struct {
	Kind  string
	Over  string
	While string
	Count *int
	// MaxIterations is REQUIRED. There is no unbounded form — an unbounded
	// loop in a workflow that runs unattended is a defect, not a feature.
	MaxIterations  *int
	Body           string
	Var            string
	IndexVar       string
	MaxConcurrency int
	OnItemFailure  string
	Line           int
}

type Wait struct {
	Duration string
	Until    string
	Event    string
	Line     int
}

type Ref struct {
	Target string
	Line   int
}

type Property struct {
	Name  string
	Value string
}

type Resource struct {
	ID         string
	Type       string
	Name       string
	Provider   string
	URI        string
	Properties []Property
	// CredentialRef is a REFERENCE to a secret, never the secret itself.
	CredentialRef   string
	CredentialStore string
	Line            int
}

type Artifact struct {
	ID             string
	Type           string
	Name           string
	URI            string
	MediaType      string
	Digest         string
	Classification string
	Properties     []Property
	Line           int
}

type Node struct {
	ID   string
	Type string
	Name string
	Impl string
	// Idempotent is the AUTHOR's statement about whether this node may run
	// more than once with the same net effect. A runtime cannot deduce it.
	Idempotent   bool
	JoinPolicy   string
	Quorum       *int
	Compensates  string
	Description  string
	Inputs       []Port
	Outputs      []Port
	Config       []Property
	ResourceRefs []Ref
	ArtifactRefs []Ref
	Guard        *string
	Retry        *Retry
	Timeout      *Timeout
	Cases        *Cases
	Loop         *Loop
	Subworkflow  string
	Wait         *Wait
	Extensions   []Extension
	Line         int
}

type Extension struct {
	Namespace string
	Required  bool
}

func (n *Node) Input(name string) *Port {
	for i := range n.Inputs {
		if n.Inputs[i].Name == name {
			return &n.Inputs[i]
		}
	}
	return nil
}

func (n *Node) Output(name string) *Port {
	for i := range n.Outputs {
		if n.Outputs[i].Name == name {
			return &n.Outputs[i]
		}
	}
	return nil
}

type Edge struct {
	ID        string
	From      string
	To        string
	Type      string
	FromPort  string
	ToPort    string
	Condition string
	Line      int
}

func (e Edge) IsForward() bool { return IsForwardEdge(e.Type) }

type Metadata struct {
	Title           string
	Description     string
	Author          string
	Organization    string
	Created         string
	Modified        string
	License         string
	DocumentVersion string
	Tags            []string
}

// Harness is the whole workflow: one document, one unit of versioning,
// validation and execution.
type Harness struct {
	ID          string
	SpecVersion *string
	Name        string
	Entry       string
	Metadata    Metadata
	Resources   []Resource
	Artifacts   []Artifact
	Nodes       []Node
	Edges       []Edge
}

func (h *Harness) Node(id string) *Node {
	for i := range h.Nodes {
		if h.Nodes[i].ID == id {
			return &h.Nodes[i]
		}
	}
	return nil
}

func (h *Harness) Resource(id string) *Resource {
	for i := range h.Resources {
		if h.Resources[i].ID == id {
			return &h.Resources[i]
		}
	}
	return nil
}

func (h *Harness) Artifact(id string) *Artifact {
	for i := range h.Artifacts {
		if h.Artifacts[i].ID == id {
			return &h.Artifacts[i]
		}
	}
	return nil
}

// Incoming returns edges arriving at id. With forwardOnly, only control, data
// and dependency edges are considered.
func (h *Harness) Incoming(id string, forwardOnly bool) []Edge {
	var out []Edge
	for _, e := range h.Edges {
		if e.To == id && (!forwardOnly || e.IsForward()) {
			out = append(out, e)
		}
	}
	return out
}

// EntrySet reports where execution begins (§2.5).
//
// A node reachable only by an error or compensation edge is a HANDLER, not a
// start, so it is excluded even though it has no incoming forward edge.
func (h *Harness) EntrySet() []string {
	if h.Entry != "" {
		return []string{h.Entry}
	}
	var out []string
	for _, n := range h.Nodes {
		if len(h.Incoming(n.ID, false)) == 0 {
			out = append(out, n.ID)
		}
	}
	return out
}
