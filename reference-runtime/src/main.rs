//! `harnessxml` — the reference parser and validator for HarnessXML.
//!
//! Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0
//!
//! The goal of this implementation is to be UNAMBIGUOUS, not fast. It exists so
//! that every normative rule has running code and a test behind it, and so that
//! a disagreement about what the specification means can be settled by reading
//! an implementation instead of by arguing about prose.
//!
//! It is explicitly not RuMima. If this and RuMima disagree, the specification
//! decides, and at most one of them is right.

mod diag;
mod model;
mod parse;
mod validate;

use diag::Diagnostics;
use std::process::ExitCode;

const USAGE: &str = "\
harnessxml — reference implementation of the HarnessXML open specification
https://harnessxml.com/

USAGE:
    harnessxml <COMMAND> <FILE.hxml>

COMMANDS:
    validate    check a document against the specification's rules
    graph       print the resolved execution graph
    explain     per-node scheduling analysis — what each node waits for

EXIT CODES (specification §14.7):
    0    valid; warnings may have been reported
    1    invalid; at least one error
    2    the tool itself failed
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 || args[0] == "-h" || args[0] == "--help" {
        print!("{USAGE}");
        return if args.is_empty() {
            ExitCode::from(2)
        } else {
            ExitCode::SUCCESS
        };
    }

    let (command, path) = (args[0].as_str(), args[1].as_str());

    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("harnessxml: cannot read {path}: {e}");
            // Exit 2, not 1: "the validator is broken" and "the workflow is
            // wrong" need different responses, and collapsing them into one
            // non-zero exit makes a broken toolchain look like a broken document.
            return ExitCode::from(2);
        }
    };

    let mut diags = Diagnostics::default();
    let harness = parse::parse(&src, &mut diags);

    match command {
        "validate" => {
            if let Some(h) = &harness {
                validate::validate(h, &mut diags);
            }
            if !diags.is_empty() {
                print!("{}", diags.report(path));
            }
            if diags.has_errors() {
                let n = diags.errors().count();
                println!("{path}: {n} error(s)");
                ExitCode::from(1)
            } else {
                let warnings = diags.sorted().len();
                if warnings > 0 {
                    println!("{path}: valid ({warnings} warning(s))");
                } else {
                    println!("{path}: valid");
                }
                ExitCode::SUCCESS
            }
        }

        "graph" => match harness {
            Some(h) => {
                print_graph(&h);
                ExitCode::SUCCESS
            }
            None => {
                print!("{}", diags.report(path));
                ExitCode::from(1)
            }
        },

        "explain" => match harness {
            Some(h) => {
                explain(&h);
                ExitCode::SUCCESS
            }
            None => {
                print!("{}", diags.report(path));
                ExitCode::from(1)
            }
        },

        other => {
            eprintln!("harnessxml: unknown command '{other}'\n");
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn print_graph(h: &model::Harness) {
    println!(
        "harness {} (specVersion {})",
        h.id,
        h.spec_version.as_deref().unwrap_or("MISSING")
    );
    println!("  {} node(s), {} edge(s)", h.nodes.len(), h.edges.len());

    if !h.resources.is_empty() {
        println!("\nresources");
        for r in &h.resources {
            println!("  {:<20} {}", r.id, r.kind);
        }
    }

    if !h.artifacts.is_empty() {
        println!("\nartifacts");
        for a in &h.artifacts {
            println!("  {:<20} {}", a.id, a.kind);
        }
    }

    println!("\nnodes");
    for n in &h.nodes {
        let mut flags = Vec::new();
        if !n.idempotent {
            flags.push("NOT-IDEMPOTENT");
        }
        if n.has_retry {
            flags.push("retry");
        }
        if n.has_guard {
            flags.push("guard");
        }
        if n.join_policy != "all" {
            flags.push("join");
        }
        let suffix = if flags.is_empty() {
            String::new()
        } else {
            format!("   [{}]", flags.join(" "))
        };
        println!("  {:<22} {:<12}{}", n.id, n.kind, suffix);
    }

    println!("\nedges");
    for e in &h.edges {
        let ports = match (&e.from_port, &e.to_port) {
            (Some(f), Some(t)) => format!("  ({f} -> {t})"),
            _ => String::new(),
        };
        println!(
            "  {:<22} --{:^13}--> {}{}",
            e.from,
            edge_label(e.ty),
            e.to,
            ports
        );
    }
}

fn edge_label(t: model::EdgeType) -> &'static str {
    match t {
        model::EdgeType::Control => "control",
        model::EdgeType::Data => "data",
        model::EdgeType::Dependency => "dependency",
        model::EdgeType::Error => "error",
        model::EdgeType::Compensation => "compensation",
    }
}

/// Per-node scheduling analysis: what each node waits for, and what it releases.
/// This is the view that answers "why has this not started yet", which is the
/// question an operator actually has.
fn explain(h: &model::Harness) {
    let entry: Vec<&str> = match &h.entry {
        Some(e) => vec![e.as_str()],
        None => h
            .nodes
            .iter()
            .filter(|n| !h.edges.iter().any(|e| e.ty.is_forward() && e.to == n.id))
            .map(|n| n.id.as_str())
            .collect(),
    };

    println!(
        "entry set: {}",
        if entry.is_empty() {
            "EMPTY (HX-3001)".into()
        } else {
            entry.join(", ")
        }
    );

    for n in &h.nodes {
        println!("\n{} ({})", n.id, n.kind);

        let incoming: Vec<&model::Edge> = h.edges.iter().filter(|e| e.to == n.id).collect();
        if incoming.is_empty() {
            println!("  waits for: nothing — entry node");
        } else {
            let policy = if n.join_policy == "quorum" {
                format!("quorum {}", n.quorum.unwrap_or(0))
            } else {
                n.join_policy.clone()
            };
            println!("  join: {policy}");
            for e in &incoming {
                let condition = match e.ty {
                    model::EdgeType::Control => "source SUCCEEDED or SKIPPED",
                    model::EdgeType::Data => "source SUCCEEDED or SKIPPED, value available",
                    model::EdgeType::Dependency => "source reached ANY terminal state",
                    model::EdgeType::Error => "source FAILED after retries",
                    model::EdgeType::Compensation => "unwinding only",
                };
                println!(
                    "    <- {:<20} {:<13} {}",
                    e.from,
                    edge_label(e.ty),
                    condition
                );
            }
        }

        if n.has_guard {
            println!("  guard: may become SKIPPED without running (a SUCCESSFUL outcome)");
        }
        if !n.idempotent {
            println!(
                "  idempotent=false: MUST NOT be auto-retried, MUST NOT be cancelled while RUNNING"
            );
        }
        if let Some(l) = &n.loop_spec {
            println!(
                "  loop: kind={} maxIterations={} maxConcurrency implied",
                l.kind,
                l.max_iterations
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| "MISSING".into())
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str) -> Diagnostics {
        let mut d = Diagnostics::default();
        if let Some(h) = parse::parse(src, &mut d) {
            validate::validate(&h, &mut d);
        }
        d
    }

    fn codes(d: &Diagnostics) -> Vec<&str> {
        d.errors().map(|x| x.code).collect()
    }

    const MINIMAL: &str = r#"<?xml version="1.0"?>
<harness xmlns="https://harnessxml.com/spec/1.0" id="m" specVersion="1.0">
  <nodes><node id="only" type="task" impl="noop"/></nodes>
</harness>"#;

    #[test]
    fn minimal_document_is_valid() {
        let d = check(MINIMAL);
        assert!(!d.has_errors(), "{:?}", codes(&d));
    }

    #[test]
    fn missing_spec_version_is_hx_1002() {
        let src = MINIMAL.replace(" specVersion=\"1.0\"", "");
        assert!(codes(&check(&src)).contains(&"HX-1002"));
    }

    #[test]
    fn duplicate_node_id_is_hx_1101() {
        let src = MINIMAL.replace(
            "<node id=\"only\" type=\"task\" impl=\"noop\"/>",
            "<node id=\"a\" type=\"task\"/><node id=\"a\" type=\"task\"/>",
        );
        assert!(codes(&check(&src)).contains(&"HX-1101"));
    }

    #[test]
    fn dangling_edge_is_hx_2001() {
        let src = r#"<harness xmlns="https://harnessxml.com/spec/1.0" id="d" specVersion="1.0">
  <nodes><node id="a" type="task"/></nodes>
  <edges><edge from="a" to="ghost" type="control"/></edges>
</harness>"#;
        assert!(codes(&check(src)).contains(&"HX-2001"));
    }

    #[test]
    fn unknown_node_type_is_hx_1003() {
        let src = MINIMAL.replace("type=\"task\"", "type=\"quantum\"");
        assert!(codes(&check(&src)).contains(&"HX-1003"));
    }

    #[test]
    fn unknown_element_is_hx_1003_not_ignored() {
        let src = MINIMAL.replace("<nodes>", "<nodes><wibble/>");
        assert!(
            codes(&check(&src)).contains(&"HX-1003"),
            "an unrecognised element must be an error, never skipped"
        );
    }

    #[test]
    fn retry_on_non_idempotent_is_hx_3301() {
        let src = MINIMAL.replace(
            "<node id=\"only\" type=\"task\" impl=\"noop\"/>",
            "<node id=\"p\" type=\"task\" idempotent=\"false\"><retry maxAttempts=\"3\"/></node>",
        );
        assert!(codes(&check(&src)).contains(&"HX-3301"));
    }

    #[test]
    fn cycle_is_hx_3003() {
        let src = r#"<harness xmlns="https://harnessxml.com/spec/1.0" id="c" specVersion="1.0">
  <nodes><node id="a" type="task"/><node id="b" type="task"/></nodes>
  <edges>
    <edge from="a" to="b" type="control"/>
    <edge from="b" to="a" type="control"/>
  </edges>
</harness>"#;
        let d = check(src);
        assert!(codes(&d).contains(&"HX-3003"));
        assert!(
            codes(&d).contains(&"HX-3001"),
            "a pure cycle also has no entry node"
        );
    }

    #[test]
    fn error_edges_do_not_create_a_cycle() {
        let src = r#"<harness xmlns="https://harnessxml.com/spec/1.0" id="e" specVersion="1.0">
  <nodes><node id="a" type="task"/><node id="b" type="task"/></nodes>
  <edges>
    <edge from="a" to="b" type="control"/>
    <edge from="b" to="a" type="error"/>
  </edges>
</harness>"#;
        assert!(!codes(&check(src)).contains(&"HX-3003"));
    }

    #[test]
    fn literal_credential_is_hx_3501() {
        let src = r#"<harness xmlns="https://harnessxml.com/spec/1.0" id="l" specVersion="1.0">
  <resources>
    <resource id="m" type="model">
      <property name="apiKey" value="sk-ant-api03-AAAAAAAAAAAAAAAAAAAAAAAA"/>
    </resource>
  </resources>
  <nodes><node id="a" type="inference"><resourceRef ref="m"/></node></nodes>
</harness>"#;
        assert!(codes(&check(src)).contains(&"HX-3501"));
    }

    #[test]
    fn credential_reference_is_fine() {
        let src = r#"<harness xmlns="https://harnessxml.com/spec/1.0" id="ok" specVersion="1.0">
  <resources>
    <resource id="m" type="model">
      <credential ref="ANTHROPIC_API_KEY" store="vault"/>
    </resource>
  </resources>
  <nodes><node id="a" type="inference"><resourceRef ref="m"/></node></nodes>
</harness>"#;
        assert!(!codes(&check(src)).contains(&"HX-3501"));
    }

    #[test]
    fn unbounded_loop_is_rejected() {
        let src = r#"<harness xmlns="https://harnessxml.com/spec/1.0" id="u" specVersion="1.0">
  <nodes>
    <node id="l" type="loop"><loop kind="while" while="${true}"><body ref="w"/></loop></node>
    <node id="w" type="task"/>
  </nodes>
</harness>"#;
        assert!(codes(&check(src)).contains(&"HX-1001"));
    }

    #[test]
    fn months_in_a_duration_are_hx_3401_but_minutes_are_fine() {
        let bad = MINIMAL.replace(
            "<node id=\"only\" type=\"task\" impl=\"noop\"/>",
            "<node id=\"n\" type=\"task\"><timeout duration=\"P1M\"/></node>",
        );
        assert!(codes(&check(&bad)).contains(&"HX-3401"));

        let good = MINIMAL.replace(
            "<node id=\"only\" type=\"task\" impl=\"noop\"/>",
            "<node id=\"n\" type=\"task\"><timeout duration=\"PT5M\"/></node>",
        );
        assert!(
            !codes(&check(&good)).contains(&"HX-3401"),
            "PT5M is five minutes"
        );
    }

    #[test]
    fn vendor_extension_content_is_not_parsed_as_harnessxml() {
        let src = MINIMAL.replace(
            "<node id=\"only\" type=\"task\" impl=\"noop\"/>",
            r#"<node id="n" type="task">
                 <extension namespace="https://acme.example/h/1">
                   <acme:anything xmlns:acme="https://acme.example/h/1"><acme:nested/></acme:anything>
                 </extension>
               </node>"#,
        );
        let d = check(&src);
        assert!(!d.has_errors(), "{:?}", codes(&d));
    }
}
