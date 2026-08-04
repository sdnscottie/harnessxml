//! The HarnessXML object model, as defined in specification chapter 1.
//!
//! Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0
//!
//! Deliberately close to the document: one struct per element, attributes as
//! fields, nothing normalised at parse time. Validation is a separate pass so
//! that a document failing one rule can still be reported on for the others.

pub const NS: &str = "https://harnessxml.com/spec/1.0";

#[derive(Debug, Default)]
pub struct Harness {
    pub id: String,
    pub spec_version: Option<String>,
    pub name: Option<String>,
    pub entry: Option<String>,
    pub resources: Vec<Resource>,
    pub artifacts: Vec<Artifact>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    /// Every `property`/`value` in the document, kept flat for the
    /// literal-credential scan (HX-3501).
    pub scalar_values: Vec<Scalar>,
}

#[derive(Debug)]
pub struct Scalar {
    pub context: String,
    pub name: String,
    pub value: String,
    pub line: usize,
}

#[derive(Debug, Default)]
pub struct Resource {
    pub id: String,
    pub kind: String,
    pub line: usize,
}

#[derive(Debug, Default)]
pub struct Artifact {
    pub id: String,
    pub kind: String,
    pub line: usize,
}

#[derive(Debug, Default)]
pub struct Node {
    pub id: String,
    pub kind: String,
    pub idempotent: bool,
    pub join_policy: String,
    pub quorum: Option<u32>,
    pub compensates: Option<String>,
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,
    pub resource_refs: Vec<Ref>,
    pub artifact_refs: Vec<Ref>,
    pub has_retry: bool,
    pub has_guard: bool,
    pub cases: Option<Cases>,
    pub loop_spec: Option<LoopSpec>,
    pub subworkflow: Option<String>,
    pub wait: Option<Wait>,
    pub durations: Vec<(String, String)>,
    pub line: usize,
}

#[derive(Debug, Default)]
pub struct Port {
    pub name: String,
    pub ty: Option<String>,
    pub required: bool,
    pub has_default: bool,
    pub has_value: bool,
    pub line: usize,
}

#[derive(Debug)]
pub struct Ref {
    pub target: String,
    pub line: usize,
}

#[derive(Debug, Default)]
pub struct Cases {
    pub cases: Vec<(String, String)>,
    pub otherwise: Option<String>,
    pub line: usize,
}

#[derive(Debug, Default)]
pub struct LoopSpec {
    pub kind: String,
    pub over: Option<String>,
    pub while_expr: Option<String>,
    pub count: Option<u64>,
    pub max_iterations: Option<u64>,
    pub body: Option<String>,
    pub line: usize,
}

#[derive(Debug, Default)]
pub struct Wait {
    pub duration: Option<String>,
    pub until: Option<String>,
    pub event: Option<String>,
    pub line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    Control,
    Data,
    Dependency,
    Error,
    Compensation,
}

impl EdgeType {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "control" => Self::Control,
            "data" => Self::Data,
            "dependency" => Self::Dependency,
            "error" => Self::Error,
            "compensation" => Self::Compensation,
            _ => return None,
        })
    }

    /// Whether this edge participates in the acyclicity check (HX-3003) and in
    /// forward reachability. `error` and `compensation` do not: a handler may
    /// legitimately point backwards, and compensation points backwards by
    /// definition.
    pub fn is_forward(self) -> bool {
        matches!(self, Self::Control | Self::Data | Self::Dependency)
    }
}

#[derive(Debug)]
pub struct Edge {
    pub id: Option<String>,
    pub from: String,
    pub to: String,
    pub ty: EdgeType,
    pub from_port: Option<String>,
    pub to_port: Option<String>,
    pub line: usize,
}

pub const NODE_TYPES: &[&str] = &[
    "task",
    "inference",
    "transform",
    "decision",
    "loop",
    "parallel",
    "barrier",
    "subworkflow",
    "source",
    "sink",
    "wait",
    "human",
];

impl Harness {
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }
}

impl Node {
    pub fn input(&self, name: &str) -> Option<&Port> {
        self.inputs.iter().find(|p| p.name == name)
    }
    pub fn output(&self, name: &str) -> Option<&Port> {
        self.outputs.iter().find(|p| p.name == name)
    }
}
