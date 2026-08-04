// Command harnessxml validates HarnessXML documents.
//
// Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0
//
// Exit codes follow specification §14.7, so this is usable in CI:
//
//	0  valid; warnings may have been reported
//	1  invalid; at least one error
//	2  the tool itself failed
package main

import (
	"fmt"
	"os"

	harnessxml "gitlab.com/visml/harnessxml/sdk/go"
)

const usage = `harnessxml — Go SDK for the HarnessXML open specification
https://harnessxml.com/

USAGE:
    harnessxml validate FILE...
    harnessxml graph    FILE
    harnessxml --version

EXIT CODES (specification §14.7):
    0    valid; warnings may have been reported
    1    invalid; at least one error
    2    the tool itself failed
`

func main() { os.Exit(run(os.Args[1:])) }

func run(args []string) int {
	if len(args) == 0 {
		fmt.Fprint(os.Stderr, usage)
		return 2
	}
	if args[0] == "-h" || args[0] == "--help" {
		fmt.Print(usage)
		return 0
	}
	if args[0] == "--version" {
		fmt.Printf("harnessxml %s (specification %s)\n", harnessxml.Version, harnessxml.SpecVersion)
		return 0
	}

	command, files := args[0], args[1:]
	if len(files) == 0 {
		fmt.Fprintf(os.Stderr, "harnessxml: %s needs a file\n", command)
		return 2
	}

	switch command {
	case "graph":
		return graph(files[0])
	case "validate":
		worst := 0
		for _, f := range files {
			_, d, err := harnessxml.CheckFile(f)
			if err != nil {
				// Exit 2, not 1: "the validator is broken" and "the workflow
				// is wrong" need different responses in CI.
				fmt.Fprintf(os.Stderr, "harnessxml: cannot read %s: %v\n", f, err)
				return 2
			}
			if len(d.Items) > 0 {
				fmt.Print(d.Report(f))
			}
			switch {
			case d.HasErrors():
				fmt.Printf("%s: %d error(s)\n", f, len(d.Errors()))
				worst = 1
			case len(d.Warnings()) > 0:
				fmt.Printf("%s: valid (%d warning(s))\n", f, len(d.Warnings()))
			default:
				fmt.Printf("%s: valid\n", f)
			}
		}
		// A build that fails on advisory findings trains people to suppress
		// warnings, which loses the errors too (§14.7).
		return worst
	default:
		fmt.Fprintf(os.Stderr, "harnessxml: unknown command %q\n\n%s", command, usage)
		return 2
	}
}

func graph(path string) int {
	h, err := harnessxml.Load(path)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		return 1
	}
	sv := "MISSING"
	if h.SpecVersion != nil {
		sv = *h.SpecVersion
	}
	fmt.Printf("harness %s (specVersion %s)\n", h.ID, sv)
	fmt.Printf("  %d node(s), %d edge(s)\n", len(h.Nodes), len(h.Edges))
	if len(h.Resources) > 0 {
		fmt.Println("\nresources")
		for _, r := range h.Resources {
			fmt.Printf("  %-20s %s\n", r.ID, r.Type)
		}
	}
	fmt.Println("\nnodes")
	for _, n := range h.Nodes {
		flags := ""
		if !n.Idempotent {
			flags += " NOT-IDEMPOTENT"
		}
		if n.Retry != nil {
			flags += " retry"
		}
		if n.Guard != nil {
			flags += " guard"
		}
		if flags != "" {
			flags = "   [" + flags[1:] + "]"
		}
		fmt.Printf("  %-22s %-12s%s\n", n.ID, n.Type, flags)
	}
	fmt.Println("\nedges")
	for _, e := range h.Edges {
		ports := ""
		if e.FromPort != "" && e.ToPort != "" {
			ports = fmt.Sprintf("  (%s -> %s)", e.FromPort, e.ToPort)
		}
		fmt.Printf("  %-22s --%-13s--> %s%s\n", e.From, e.Type, e.To, ports)
	}
	return 0
}
