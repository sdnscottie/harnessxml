//! `.hxml` → [`Harness`].
//!
//! Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0
//!
//! Namespace-aware, per specification §2.6: elements are matched on namespace
//! URI plus local name, never on prefix.
//!
//! Per §2.8 an unrecognised element in the HarnessXML namespace is an ERROR
//! (HX-1003), not something to skip. That rule is enforced here, in the parser,
//! rather than left to the validator — a construct the parser silently dropped
//! is one the validator can never see.

use crate::diag::{Diagnostic, Diagnostics};
use crate::model::*;
use quick_xml::NsReader;
use quick_xml::XmlVersion;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::ResolveResult;

/// Byte offset → 1-based line number.
struct Lines(Vec<usize>);

impl Lines {
    fn new(src: &str) -> Self {
        let mut v = vec![0];
        for (i, b) in src.bytes().enumerate() {
            if b == b'\n' {
                v.push(i + 1);
            }
        }
        Self(v)
    }
    fn at(&self, offset: usize) -> usize {
        match self.0.binary_search(&offset) {
            Ok(i) => i + 1,
            Err(i) => i,
        }
    }
}

fn attr(e: &BytesStart, key: &str) -> Option<String> {
    for a in e.attributes().flatten() {
        if a.key.local_name().into_inner() == key.as_bytes() {
            // normalized_value() replaced the deprecated unescape_value() in
            // quick-xml 0.41. It additionally applies XML attribute-value
            // normalisation (tab/newline/CR collapse to spaces), which is what
            // the XML specification requires and what an XSD validator will
            // already have assumed — so the two validation layers now agree.
            //
            // XML 1.0 is asserted: HarnessXML documents are XML 1.0, and 1.1
            // differs here (it normalises NEL and LINE SEPARATOR too). Passing
            // a fixed version rather than sniffing the declaration keeps
            // normalisation identical whether or not a document carries one.
            return Some(
                a.normalized_value(XmlVersion::Explicit1_0)
                    .ok()?
                    .into_owned(),
            );
        }
    }
    None
}

fn bool_attr(e: &BytesStart, key: &str, default: bool) -> bool {
    match attr(e, key).as_deref() {
        Some("true") => true,
        Some("false") => false,
        _ => default,
    }
}

pub fn parse(src: &str, diags: &mut Diagnostics) -> Option<Harness> {
    let lines = Lines::new(src);
    let mut reader = NsReader::from_str(src);
    reader.config_mut().trim_text(true);

    let mut h = Harness::default();
    let mut path: Vec<String> = Vec::new();
    let mut extension_depth: usize = 0;
    let mut seen_root = false;

    loop {
        let position = reader.buffer_position() as usize;
        let line = lines.at(position);

        let (ns, event) = match reader.read_resolved_event() {
            Ok(pair) => pair,
            Err(e) => {
                diags.push(Diagnostic::error(
                    "HX-1001",
                    line,
                    format!("not well-formed: {e}"),
                ));
                return None;
            }
        };

        let in_hx_ns = matches!(ns, ResolveResult::Bound(n) if n.as_ref() == NS.as_bytes());

        match event {
            Event::Eof => break,

            Event::Start(ref e) | Event::Empty(ref e) => {
                let empty = matches!(event, Event::Empty(_));
                let name = String::from_utf8_lossy(e.local_name().into_inner()).into_owned();

                // Inside a vendor extension: anything goes, and none of it is
                // ours to interpret (§2.8).
                if extension_depth > 0 {
                    if !empty {
                        extension_depth += 1;
                    }
                    continue;
                }

                if name == "extension" && in_hx_ns {
                    if !empty {
                        extension_depth = 1;
                    }
                    if !empty {
                        path.push(name);
                    }
                    continue;
                }

                if !in_hx_ns {
                    diags.push(Diagnostic::error(
                        "HX-1006",
                        line,
                        format!("element <{name}> is from a foreign namespace and is only permitted inside <extension>"),
                    ));
                    if !empty {
                        path.push(name);
                    }
                    continue;
                }

                let parent = path.last().map(String::as_str).unwrap_or("");
                handle_start(&mut h, &mut seen_root, parent, &name, e, line, diags);

                if !empty {
                    path.push(name);
                }
            }

            Event::End(_) => {
                if extension_depth > 0 {
                    extension_depth -= 1;
                    if extension_depth == 0 {
                        path.pop();
                    }
                    continue;
                }
                path.pop();
            }

            _ => {}
        }
    }

    if !seen_root {
        diags.push(Diagnostic::error(
            "HX-1001",
            1,
            "no <harness> root element in the HarnessXML namespace".to_string(),
        ));
        return None;
    }
    Some(h)
}

#[allow(clippy::too_many_arguments)]
fn handle_start(
    h: &mut Harness,
    seen_root: &mut bool,
    parent: &str,
    name: &str,
    e: &BytesStart,
    line: usize,
    diags: &mut Diagnostics,
) {
    match (parent, name) {
        ("", "harness") => {
            *seen_root = true;
            h.id = attr(e, "id").unwrap_or_default();
            h.spec_version = attr(e, "specVersion");
            h.name = attr(e, "name");
            h.entry = attr(e, "entry");
        }

        // Containers carry no data of their own.
        ("harness", "metadata")
        | ("harness", "security")
        | ("harness", "resources")
        | ("harness", "artifacts")
        | ("harness", "nodes")
        | ("harness", "edges")
        | ("node", "inputs")
        | ("node", "outputs")
        | ("node", "config")
        | ("node", "security")
        | ("metadata", _)
        | ("provenance", _)
        | ("tags", "tag")
        | ("security", "permission")
        | ("resource", "description")
        | ("resource", "credential")
        | ("artifact", "description")
        | ("node", "description")
        | ("edge", "description")
        | ("input", "description")
        | ("output", "description") => {}

        ("resources", "resource") => h.resources.push(Resource {
            id: attr(e, "id").unwrap_or_default(),
            kind: attr(e, "type").unwrap_or_default(),
            line,
        }),

        ("artifacts", "artifact") => h.artifacts.push(Artifact {
            id: attr(e, "id").unwrap_or_default(),
            kind: attr(e, "type").unwrap_or_default(),
            line,
        }),

        ("nodes", "node") => h.nodes.push(Node {
            id: attr(e, "id").unwrap_or_default(),
            kind: attr(e, "type").unwrap_or_default(),
            idempotent: bool_attr(e, "idempotent", true),
            join_policy: attr(e, "joinPolicy").unwrap_or_else(|| "all".into()),
            quorum: attr(e, "quorum").and_then(|q| q.parse().ok()),
            compensates: attr(e, "compensates"),
            line,
            ..Default::default()
        }),

        ("edges", "edge") => {
            let raw = attr(e, "type").unwrap_or_else(|| "control".into());
            match EdgeType::parse(&raw) {
                Some(ty) => h.edges.push(Edge {
                    id: attr(e, "id"),
                    from: attr(e, "from").unwrap_or_default(),
                    to: attr(e, "to").unwrap_or_default(),
                    ty,
                    from_port: attr(e, "fromPort"),
                    to_port: attr(e, "toPort"),
                    line,
                }),
                None => diags.push(Diagnostic::error(
                    "HX-1003",
                    line,
                    format!("unrecognised edge type '{raw}'"),
                )),
            }
        }

        ("inputs", "input") | ("outputs", "output") => {
            let port = Port {
                name: attr(e, "name").unwrap_or_default(),
                ty: attr(e, "type"),
                required: bool_attr(e, "required", true),
                has_default: attr(e, "default").is_some(),
                has_value: attr(e, "value").is_some(),
                line,
            };
            if let Some(v) = attr(e, "value") {
                h.scalar_values.push(Scalar {
                    context: format!("port '{}'", port.name),
                    name: port.name.clone(),
                    value: v,
                    line,
                });
            }
            if let Some(n) = h.nodes.last_mut() {
                if name == "input" {
                    n.inputs.push(port);
                } else {
                    n.outputs.push(port);
                }
            }
        }

        ("config", "property") | ("resource", "property") | ("artifact", "property") => {
            let pname = attr(e, "name").unwrap_or_default();
            h.scalar_values.push(Scalar {
                context: format!("property '{pname}'"),
                name: pname,
                value: attr(e, "value").unwrap_or_default(),
                line,
            });
        }

        ("node", "resourceRef") => {
            if let Some(n) = h.nodes.last_mut() {
                n.resource_refs.push(Ref {
                    target: attr(e, "ref").unwrap_or_default(),
                    line,
                });
            }
        }

        ("node", "artifactRef") => {
            if let Some(n) = h.nodes.last_mut() {
                n.artifact_refs.push(Ref {
                    target: attr(e, "ref").unwrap_or_default(),
                    line,
                });
            }
        }

        ("node", "guard") => {
            if let Some(n) = h.nodes.last_mut() {
                n.has_guard = true;
            }
        }

        ("node", "retry") => {
            if let Some(n) = h.nodes.last_mut() {
                n.has_retry = true;
            }
        }

        ("node", "timeout") => {
            if let Some(d) = attr(e, "duration")
                && let Some(n) = h.nodes.last_mut()
            {
                n.durations.push(("timeout".into(), d));
            }
        }

        ("node", "cases") => {
            if let Some(n) = h.nodes.last_mut() {
                n.cases = Some(Cases {
                    line,
                    ..Default::default()
                });
            }
        }

        ("cases", "case") => {
            if let Some(c) = h.nodes.last_mut().and_then(|n| n.cases.as_mut()) {
                c.cases.push((
                    attr(e, "when").unwrap_or_default(),
                    attr(e, "to").unwrap_or_default(),
                ));
            }
        }

        ("cases", "otherwise") => {
            if let Some(c) = h.nodes.last_mut().and_then(|n| n.cases.as_mut()) {
                c.otherwise = attr(e, "to");
            }
        }

        ("node", "loop") => {
            if let Some(n) = h.nodes.last_mut() {
                n.loop_spec = Some(LoopSpec {
                    kind: attr(e, "kind").unwrap_or_default(),
                    over: attr(e, "over"),
                    while_expr: attr(e, "while"),
                    count: attr(e, "count").and_then(|c| c.parse().ok()),
                    max_iterations: attr(e, "maxIterations").and_then(|m| m.parse().ok()),
                    body: None,
                    line,
                });
            }
        }

        ("loop", "body") => {
            if let Some(l) = h.nodes.last_mut().and_then(|n| n.loop_spec.as_mut()) {
                l.body = attr(e, "ref");
            }
        }

        ("node", "subworkflow") => {
            if let Some(n) = h.nodes.last_mut() {
                n.subworkflow = attr(e, "href");
            }
        }

        ("node", "wait") => {
            if let Some(d) = attr(e, "duration")
                && let Some(n) = h.nodes.last_mut()
            {
                n.durations.push(("wait".into(), d));
            }
            if let Some(n) = h.nodes.last_mut() {
                n.wait = Some(Wait {
                    duration: attr(e, "duration"),
                    until: attr(e, "until"),
                    event: attr(e, "event"),
                    line,
                });
            }
        }

        // §2.8 — an unrecognised element in the HarnessXML namespace is an
        // error. Skipping it is how a run reports success having done less
        // than it was told.
        _ => diags.push(Diagnostic::error(
            "HX-1003",
            line,
            format!("unrecognised element <{name}> inside <{parent}>"),
        )),
    }
}
