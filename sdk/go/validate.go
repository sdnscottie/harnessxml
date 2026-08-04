package harnessxml

// The validation rules of specification chapter 13.
//
// Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0
//
// Every rule here carries the code the specification assigns it, and every code
// has a conformance fixture that must be rejected with exactly that code. This
// is a deliberate re-implementation of the same rules as the Rust reference
// validator and the Python SDK — if any two disagree, the conformance suite
// says which is wrong.

import (
	"regexp"
	"strings"
)

var identRe = regexp.MustCompile(`^[A-Za-z_][A-Za-z0-9_.\-]*$`)

// Detection is necessarily heuristic (§13.3), so a confident hit is an error
// and a suspicious one is a warning.
var credentialPrefixes = []string{
	"sk-ant-", "sk-", "AKIA", "ghp_", "github_pat_", "xoxb-", "AIza", "-----BEGIN",
}

var secretNames = []string{"key", "secret", "token", "password", "passwd", "credential"}

// Validate applies every rule of chapter 13 to h.
func Validate(h *Harness, d *Diagnostics) {
	validateDocument(h, d)
	validateIdentifiers(h, d)
	validateReferences(h, d)
	validateNodeShape(h, d)
	validatePortsAndEdges(h, d)
	validateGraph(h, d)
	validatePolicy(h, d)
	validateCredentials(h, d)
}

// ------------------------------------------------------------------ HX-1xxx

func validateDocument(h *Harness, d *Diagnostics) {
	if h.SpecVersion == nil {
		d.Errorf("HX-1002", 1,
			"<harness> has no specVersion; a runtime cannot safely guess which semantics apply, and guessing wrong is worse than refusing")
	}
	if len(h.Nodes) == 0 {
		d.Errorf("HX-1102", 1, "<nodes> must contain at least one <node>")
	}
	for _, n := range h.Nodes {
		known := false
		for _, t := range NodeTypes {
			if t == n.Type {
				known = true
			}
		}
		if !known {
			d.Errorf("HX-1003", n.Line,
				"node '%s': unrecognised type '%s'. A runtime must refuse rather than skip it", n.ID, n.Type)
		}
		if n.ID != "" && !identRe.MatchString(n.ID) {
			d.Errorf("HX-1001", n.Line, "node id '%s' is not a valid identifier", n.ID)
		}
	}
}

func validateIdentifiers(h *Harness, d *Diagnostics) {
	dup := func(kind string, ids []string, lines []int) {
		seen := map[string]bool{}
		for i, id := range ids {
			if id == "" {
				continue
			}
			if seen[id] {
				d.Errorf("HX-1101", lines[i],
					"duplicate %s id '%s'; every reference to it would be ambiguous", kind, id)
			}
			seen[id] = true
		}
	}

	var ids []string
	var lines []int
	for _, n := range h.Nodes {
		ids = append(ids, n.ID)
		lines = append(lines, n.Line)
	}
	dup("node", ids, lines)

	ids, lines = nil, nil
	for _, r := range h.Resources {
		ids = append(ids, r.ID)
		lines = append(lines, r.Line)
	}
	dup("resource", ids, lines)

	ids, lines = nil, nil
	for _, a := range h.Artifacts {
		ids = append(ids, a.ID)
		lines = append(lines, a.Line)
	}
	dup("artifact", ids, lines)

	ids, lines = nil, nil
	for _, e := range h.Edges {
		if e.ID != "" {
			ids = append(ids, e.ID)
			lines = append(lines, e.Line)
		}
	}
	dup("edge", ids, lines)

	for _, n := range h.Nodes {
		for _, pair := range []struct {
			dir   string
			ports []Port
		}{{"input", n.Inputs}, {"output", n.Outputs}} {
			seen := map[string]bool{}
			for _, p := range pair.ports {
				if seen[p.Name] {
					d.Errorf("HX-1101", p.Line, "node '%s': duplicate %s port '%s'", n.ID, pair.dir, p.Name)
				}
				seen[p.Name] = true
			}
		}
	}
}

// ------------------------------------------------------------------ HX-2xxx

func validateReferences(h *Harness, d *Diagnostics) {
	nodes := map[string]bool{}
	for _, n := range h.Nodes {
		nodes[n.ID] = true
	}
	resources := map[string]bool{}
	for _, r := range h.Resources {
		resources[r.ID] = true
	}
	artifacts := map[string]bool{}
	for _, a := range h.Artifacts {
		artifacts[a.ID] = true
	}

	for _, e := range h.Edges {
		label := ""
		if e.ID != "" {
			label = " '" + e.ID + "'"
		}
		if !nodes[e.From] {
			d.Errorf("HX-2001", e.Line, "edge%s: from names '%s', which is not a declared node", label, e.From)
		}
		if !nodes[e.To] {
			d.Errorf("HX-2001", e.Line, "edge%s: to names '%s', which is not a declared node", label, e.To)
		}
	}

	for _, n := range h.Nodes {
		if n.Cases != nil {
			for _, c := range n.Cases.Cases {
				if !nodes[c.To] {
					d.Errorf("HX-2001", n.Cases.Line,
						"node '%s': case targets '%s', which is not a declared node", n.ID, c.To)
				}
			}
			if n.Cases.Otherwise != nil && !nodes[*n.Cases.Otherwise] {
				d.Errorf("HX-2001", n.Cases.Line,
					"node '%s': otherwise targets '%s', which is not a declared node", n.ID, *n.Cases.Otherwise)
			}
		}
		if n.Loop != nil && n.Loop.Body != "" && !nodes[n.Loop.Body] {
			d.Errorf("HX-2001", n.Loop.Line,
				"node '%s': loop body references '%s', which is not a declared node", n.ID, n.Loop.Body)
		}
		for _, r := range n.ResourceRefs {
			if !resources[r.Target] {
				d.Errorf("HX-2002", r.Line,
					"node '%s': resourceRef '%s' is not a declared resource", n.ID, r.Target)
			}
		}
		for _, a := range n.ArtifactRefs {
			if !artifacts[a.Target] {
				d.Errorf("HX-2003", a.Line,
					"node '%s': artifactRef '%s' is not a declared artifact", n.ID, a.Target)
			}
		}
		if n.Compensates != "" && !nodes[n.Compensates] {
			d.Errorf("HX-2004", n.Line,
				"node '%s': compensates '%s', which is not a declared node", n.ID, n.Compensates)
		}
	}
}

func validateNodeShape(h *Harness, d *Diagnostics) {
	for i := range h.Nodes {
		n := &h.Nodes[i]

		check := func(present bool, wantType, elem, code string) {
			expected := n.Type == wantType
			if present == expected {
				return
			}
			if expected {
				d.Errorf(code, n.Line, "node '%s': type=\"%s\" requires <%s>", n.ID, wantType, elem)
			} else {
				d.Errorf(code, n.Line,
					"node '%s': <%s> belongs on type=\"%s\" and nowhere else (this node is \"%s\")",
					n.ID, elem, wantType, n.Type)
			}
		}
		check(n.Cases != nil, "decision", "cases", "HX-2201")
		check(n.Loop != nil, "loop", "loop", "HX-2202")
		check(n.Subworkflow != "", "subworkflow", "subworkflow", "HX-2203")
		check(n.Wait != nil, "wait", "wait", "HX-2204")

		if n.Wait != nil {
			count := 0
			for _, v := range []string{n.Wait.Duration, n.Wait.Until, n.Wait.Event} {
				if v != "" {
					count++
				}
			}
			if count != 1 {
				d.Errorf("HX-2205", n.Wait.Line,
					"node '%s': <wait> must declare exactly one of duration, until or event (found %d)", n.ID, count)
			}
		}

		if n.Cases != nil {
			if len(n.Cases.Cases) == 0 {
				d.Errorf("HX-2206", n.Cases.Line, "node '%s': <cases> must contain at least one <case>", n.ID)
			}
			if n.Cases.Otherwise == nil {
				d.Warnf("HX-4103", n.Cases.Line,
					"node '%s': no <otherwise>; if no case matches at runtime this fails with HX-4103", n.ID)
			}
		}

		if n.Loop != nil {
			l := n.Loop
			if l.MaxIterations == nil {
				d.Errorf("HX-1001", l.Line,
					"node '%s': loop has no maxIterations. There is no unbounded form — an unbounded loop in an unattended workflow is a defect", n.ID)
			}
			if l.Body == "" {
				d.Errorf("HX-1001", l.Line, "node '%s': <loop> has no <body>", n.ID)
			}
			switch l.Kind {
			case "forEach":
				if l.Over == "" {
					d.Errorf("HX-2207", l.Line, "node '%s': loop kind=\"forEach\" requires the 'over' attribute", n.ID)
				}
			case "while", "until":
				if l.While == "" {
					d.Errorf("HX-2207", l.Line, "node '%s': loop kind=\"%s\" requires the 'while' attribute", n.ID, l.Kind)
				}
			case "times":
				if l.Count == nil {
					d.Errorf("HX-2207", l.Line, "node '%s': loop kind=\"times\" requires the 'count' attribute", n.ID)
				}
			default:
				d.Errorf("HX-1003", l.Line, "node '%s': unrecognised loop kind '%s'", n.ID, l.Kind)
			}
			if l.Count != nil && l.MaxIterations != nil && *l.Count > *l.MaxIterations {
				d.Errorf("HX-2208", l.Line,
					"node '%s': count %d exceeds maxIterations %d; the document states two different bounds",
					n.ID, *l.Count, *l.MaxIterations)
			}
		}

		incoming := len(h.Incoming(n.ID, true))
		if n.JoinPolicy == "quorum" {
			if n.Quorum == nil {
				d.Errorf("HX-2401", n.Line, "node '%s': joinPolicy=\"quorum\" requires @quorum", n.ID)
			} else if *n.Quorum > incoming {
				d.Errorf("HX-2402", n.Line,
					"node '%s': quorum %d exceeds its %d incoming edge(s); it can never be satisfied",
					n.ID, *n.Quorum, incoming)
			}
		}

		if n.Type == "inference" {
			hasModel := false
			for _, ref := range n.ResourceRefs {
				if r := h.Resource(ref.Target); r != nil && r.Type == "model" {
					hasModel = true
				}
			}
			if !hasModel {
				d.Errorf("HX-2501", n.Line,
					"node '%s': type=\"inference\" must reference a resource of type=\"model\"", n.ID)
			}
		}
	}
}

func validatePortsAndEdges(h *Harness, d *Diagnostics) {
	type key struct{ node, port string }
	fed := map[key]int{}

	for _, e := range h.Edges {
		if e.Type != "data" {
			continue
		}
		if e.FromPort == "" || e.ToPort == "" {
			d.Errorf("HX-2301", e.Line,
				"edge %s -> %s: a data edge must declare both fromPort and toPort", e.From, e.To)
			continue
		}
		src, dst := h.Node(e.From), h.Node(e.To)
		if src != nil && src.Output(e.FromPort) == nil {
			d.Errorf("HX-2302", e.Line,
				"edge %s -> %s: fromPort '%s' is not an output on '%s'", e.From, e.To, e.FromPort, e.From)
		}
		if dst != nil && dst.Input(e.ToPort) == nil {
			d.Errorf("HX-2303", e.Line,
				"edge %s -> %s: toPort '%s' is not an input on '%s'", e.From, e.To, e.ToPort, e.To)
		}
		fed[key{e.To, e.ToPort}]++

		// HX-3201 — checked only when BOTH ports declare a type. Untyped means
		// unchecked, not "any".
		if src != nil && dst != nil {
			sp, dp := src.Output(e.FromPort), dst.Input(e.ToPort)
			if sp != nil && dp != nil && sp.Type != "" && dp.Type != "" &&
				sp.Type != dp.Type && dp.Type != "json" {
				d.Errorf("HX-3201", e.Line,
					"edge %s -> %s: type '%s' is not compatible with '%s'", e.From, e.To, sp.Type, dp.Type)
			}
		}
	}

	for k, count := range fed {
		if count > 1 {
			line := 1
			if n := h.Node(k.node); n != nil {
				line = n.Line
			}
			d.Errorf("HX-2304", line,
				"node '%s': input '%s' is fed by %d data edges; there is no defined winner",
				k.node, k.port, count)
		}
	}

	for _, n := range h.Nodes {
		for _, p := range n.Inputs {
			if !p.Required {
				continue
			}
			byEdge := fed[key{n.ID, p.Name}] > 0
			if !byEdge && !p.HasValue() && !p.HasDefault() {
				d.Errorf("HX-2101", p.Line,
					"node '%s': required input '%s' is satisfied by neither a data edge nor a value", n.ID, p.Name)
			}
			if byEdge && p.HasValue() {
				d.Errorf("HX-2102", p.Line,
					"node '%s': input '%s' has both a data edge and a value; a reader cannot tell which wins",
					n.ID, p.Name)
			}
		}
	}
}

// ------------------------------------------------------------------ HX-3xxx

func validateGraph(h *Harness, d *Diagnostics) {
	entry := h.EntrySet()
	if len(entry) == 0 && len(h.Nodes) > 0 {
		d.Errorf("HX-3001", 1,
			"the entry set is empty: every node waits for another, so nothing can begin")
	}

	// HX-3003 — acyclicity over forward edges only.
	adj := map[string][]string{}
	for _, e := range h.Edges {
		if e.IsForward() {
			adj[e.From] = append(adj[e.From], e.To)
		}
	}
	const white, grey, black = 0, 1, 2
	mark := map[string]int{}
	for _, n := range h.Nodes {
		mark[n.ID] = white
	}
	var cycle []string
	var dfs func(string, []string)
	dfs = func(at string, stack []string) {
		if cycle != nil {
			return
		}
		mark[at] = grey
		stack = append(stack, at)
		for _, next := range adj[at] {
			switch mark[next] {
			case grey:
				start := 0
				for i, s := range stack {
					if s == next {
						start = i
						break
					}
				}
				cycle = append(append([]string{}, stack[start:]...), next)
				return
			case white:
				dfs(next, stack)
				if cycle != nil {
					return
				}
			}
		}
		mark[at] = black
	}
	for _, n := range h.Nodes {
		if mark[n.ID] == white {
			dfs(n.ID, nil)
		}
	}
	if cycle != nil {
		d.Errorf("HX-3003", 1,
			"control flow contains a cycle: %s. Use a loop node, which carries a required bound",
			strings.Join(cycle, " -> "))
	}

	// HX-3004 — a loop body must not be SEQUENCED from outside the loop. A
	// DATA edge into a body is explicitly permitted: that is how a
	// loop-invariant input is bound, and forbidding it would make loops
	// nearly unusable.
	for _, n := range h.Nodes {
		if n.Loop == nil || n.Loop.Body == "" {
			continue
		}
		for _, e := range h.Edges {
			if e.To == n.Loop.Body && e.From != n.ID &&
				(e.Type == "control" || e.Type == "dependency") {
				d.Errorf("HX-3004", n.Loop.Line,
					"node '%s': loop body '%s' is also sequenced by a control or dependency edge from outside the loop; it would run at the wrong time",
					n.ID, n.Loop.Body)
				break
			}
		}
	}

	// HX-2005 / HX-2004 — compensation targets.
	for _, e := range h.Edges {
		if e.Type != "compensation" {
			continue
		}
		for _, f := range h.Edges {
			if f.To == e.To && (f.Type == "control" || f.Type == "data") {
				d.Errorf("HX-2005", e.Line,
					"node '%s' is a compensation target but is also reachable by a forward edge; it will eventually run at the wrong time", e.To)
				break
			}
		}
		if t := h.Node(e.To); t != nil && t.Compensates != "" && t.Compensates != e.From {
			d.Errorf("HX-2004", e.Line,
				"node '%s' declares compensates=\"%s\" but a compensation edge arrives from '%s'",
				e.To, t.Compensates, e.From)
		}
	}

	// HX-3005 — reachability. A WARNING: legitimate mid-authoring.
	seen := map[string]bool{}
	queue := append([]string{}, entry...)
	for len(queue) > 0 {
		at := queue[len(queue)-1]
		queue = queue[:len(queue)-1]
		if seen[at] {
			continue
		}
		seen[at] = true
		for _, e := range h.Edges {
			if e.From == at {
				queue = append(queue, e.To)
			}
		}
		if n := h.Node(at); n != nil {
			if n.Cases != nil {
				for _, c := range n.Cases.Cases {
					queue = append(queue, c.To)
				}
				if n.Cases.Otherwise != nil {
					queue = append(queue, *n.Cases.Otherwise)
				}
			}
			if n.Loop != nil && n.Loop.Body != "" {
				queue = append(queue, n.Loop.Body)
			}
		}
	}
	for _, n := range h.Nodes {
		if !seen[n.ID] {
			d.Warnf("HX-3005", n.Line, "node '%s' is not reachable from the entry set", n.ID)
		}
	}
}

func validatePolicy(h *Harness, d *Diagnostics) {
	for _, n := range h.Nodes {
		if !n.Idempotent && n.Retry != nil {
			d.Errorf("HX-3301", n.Line,
				"node '%s' is declared idempotent=\"false\" but carries a retry policy; retrying it duplicates its effect", n.ID)
		}
		type dur struct{ where, value string }
		var durations []dur
		if n.Timeout != nil {
			durations = append(durations, dur{"timeout", n.Timeout.Duration})
		}
		if n.Wait != nil && n.Wait.Duration != "" {
			durations = append(durations, dur{"wait", n.Wait.Duration})
		}
		if n.Retry != nil {
			durations = append(durations, dur{"retry initialDelay", n.Retry.InitialDelay})
			if n.Retry.MaxDelay != "" {
				durations = append(durations, dur{"retry maxDelay", n.Retry.MaxDelay})
			}
		}
		for _, x := range durations {
			// In ISO 8601 the designators before 'T' are the date part: 'M'
			// there is months, 'M' after 'T' is minutes and is fine.
			datePart := strings.SplitN(x.value, "T", 2)[0]
			if strings.ContainsAny(datePart, "YM") {
				d.Errorf("HX-3401", n.Line,
					"node '%s': %s duration '%s' uses months or years, whose length is not fixed",
					n.ID, x.where, x.value)
			}
		}
	}
}

// validateCredentials implements HX-3501 — a literal credential in a document
// designed to be committed to git, diffed in pull requests and archived for
// audit. Those are three excellent ways to publish a key.
func validateCredentials(h *Harness, d *Diagnostics) {
	type scalar struct {
		context, name, value string
		line                 int
	}
	var scalars []scalar
	for _, r := range h.Resources {
		for _, p := range r.Properties {
			scalars = append(scalars, scalar{"resource '" + r.ID + "' property '" + p.Name + "'", p.Name, p.Value, r.Line})
		}
	}
	for _, a := range h.Artifacts {
		for _, p := range a.Properties {
			scalars = append(scalars, scalar{"artifact '" + a.ID + "' property '" + p.Name + "'", p.Name, p.Value, a.Line})
		}
	}
	for _, n := range h.Nodes {
		for _, p := range n.Config {
			scalars = append(scalars, scalar{"node '" + n.ID + "' config '" + p.Name + "'", p.Name, p.Value, n.Line})
		}
		for _, p := range n.Inputs {
			if p.Value != nil {
				scalars = append(scalars, scalar{"node '" + n.ID + "' input '" + p.Name + "'", p.Name, *p.Value, p.Line})
			}
		}
	}

	for _, s := range scalars {
		v := strings.TrimSpace(s.value)
		// An expression or a reference is exactly what the format wants.
		if v == "" || strings.HasPrefix(v, "${") {
			continue
		}
		confident := false
		for _, p := range credentialPrefixes {
			if strings.HasPrefix(v, p) {
				confident = true
				break
			}
		}
		if confident {
			d.Errorf("HX-3501", s.line,
				"%s appears to contain a literal credential. Use <credential ref=\"…\" store=\"…\"/> instead", s.context)
			continue
		}
		lower := strings.ToLower(s.name)
		for _, sn := range secretNames {
			if strings.Contains(lower, sn) && len(v) >= 20 {
				d.Warnf("HX-3501", s.line,
					"%s is named like a secret and holds a long literal; if it is a credential, reference it instead", s.context)
				break
			}
		}
	}
}
