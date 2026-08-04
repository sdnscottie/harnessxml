"""The HarnessXML object model — specification chapter 1.

Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0

Deliberately close to the document: one dataclass per element, attributes as
fields, nothing normalised at parse time. Validation is a separate pass so a
document failing one rule can still be reported on for the others.
"""

from __future__ import annotations

from dataclasses import dataclass, field

NS = "https://harnessxml.com/spec/1.0"

#: Closed enumeration. A runtime meeting an unrecognised type MUST reject the
#: document (HX-1003) — never skip the node.
NODE_TYPES = (
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
)

#: The type determines what the scheduler does. These are semantics, not
#: diagram styling.
EDGE_TYPES = ("control", "data", "dependency", "error", "compensation")

#: Edges that participate in acyclicity (HX-3003) and forward reachability.
#: `error` and `compensation` do not: a handler may legitimately point
#: backwards, and compensation points backwards by definition.
FORWARD_EDGE_TYPES = ("control", "data", "dependency")


@dataclass
class Port:
    name: str = ""
    type: str | None = None
    required: bool = True
    default: str | None = None
    value: str | None = None
    line: int = 0

    @property
    def has_value(self) -> bool:
        return self.value is not None

    @property
    def has_default(self) -> bool:
        return self.default is not None


@dataclass
class Retry:
    """Retry policy — §8.1. Absent means ONE attempt."""

    max_attempts: int = 1
    backoff: str = "exponential"
    initial_delay: str = "PT1S"
    max_delay: str | None = None
    multiplier: float = 2.0
    jitter: bool = True
    #: Error classes to retry on. Empty means "retry any failure", which is
    #: convenient and usually wrong (§8.1.2).
    retry_on: list[str] = field(default_factory=list)


@dataclass
class Timeout:
    """§8.2 — bounds a SINGLE ATTEMPT, not the node's total lifetime."""

    duration: str = ""
    on_timeout: str = "fail"


@dataclass
class Cases:
    #: (when, to) in DOCUMENT ORDER. The order is normative: the first true
    #: case wins, so a decision is deterministic.
    cases: list[tuple[str, str]] = field(default_factory=list)
    otherwise: str | None = None
    line: int = 0


@dataclass
class Loop:
    kind: str = ""
    over: str | None = None
    while_expr: str | None = None
    count: int | None = None
    #: REQUIRED. There is no unbounded form — an unbounded loop in a workflow
    #: that runs unattended is a defect, not a feature.
    max_iterations: int | None = None
    body: str | None = None
    var: str = "item"
    index_var: str = "index"
    max_concurrency: int = 1
    on_item_failure: str = "fail"
    line: int = 0


@dataclass
class Wait:
    duration: str | None = None
    until: str | None = None
    event: str | None = None
    line: int = 0


@dataclass
class Ref:
    target: str = ""
    line: int = 0


@dataclass
class Resource:
    id: str = ""
    type: str = ""
    name: str | None = None
    provider: str | None = None
    uri: str | None = None
    properties: list[tuple[str, str]] = field(default_factory=list)
    #: A REFERENCE to a secret, never the secret itself.
    credential_ref: str | None = None
    credential_store: str | None = None
    line: int = 0


@dataclass
class Artifact:
    id: str = ""
    type: str = ""
    name: str | None = None
    uri: str | None = None
    media_type: str | None = None
    digest: str | None = None
    classification: str | None = None
    properties: list[tuple[str, str]] = field(default_factory=list)
    line: int = 0


@dataclass
class Node:
    id: str = ""
    type: str = ""
    name: str | None = None
    impl: str | None = None
    #: The author's statement about whether this node may run more than once
    #: with the same net effect. A runtime cannot deduce it.
    idempotent: bool = True
    join_policy: str = "all"
    quorum: int | None = None
    compensates: str | None = None
    description: str | None = None
    inputs: list[Port] = field(default_factory=list)
    outputs: list[Port] = field(default_factory=list)
    config: list[tuple[str, str]] = field(default_factory=list)
    resource_refs: list[Ref] = field(default_factory=list)
    artifact_refs: list[Ref] = field(default_factory=list)
    guard: str | None = None
    retry: Retry | None = None
    timeout: Timeout | None = None
    cases: Cases | None = None
    loop: Loop | None = None
    subworkflow_href: str | None = None
    wait: Wait | None = None
    extensions: list[tuple[str, bool]] = field(default_factory=list)
    line: int = 0

    def input(self, name: str) -> Port | None:
        return next((p for p in self.inputs if p.name == name), None)

    def output(self, name: str) -> Port | None:
        return next((p for p in self.outputs if p.name == name), None)


@dataclass
class Edge:
    from_: str = ""
    to: str = ""
    type: str = "control"
    id: str | None = None
    from_port: str | None = None
    to_port: str | None = None
    condition: str | None = None
    line: int = 0

    @property
    def is_forward(self) -> bool:
        return self.type in FORWARD_EDGE_TYPES


@dataclass
class Metadata:
    title: str | None = None
    description: str | None = None
    author: str | None = None
    organization: str | None = None
    created: str | None = None
    modified: str | None = None
    license: str | None = None
    document_version: str | None = None
    tags: list[str] = field(default_factory=list)


@dataclass
class Harness:
    id: str = ""
    spec_version: str | None = None
    name: str | None = None
    entry: str | None = None
    metadata: Metadata = field(default_factory=Metadata)
    resources: list[Resource] = field(default_factory=list)
    artifacts: list[Artifact] = field(default_factory=list)
    nodes: list[Node] = field(default_factory=list)
    edges: list[Edge] = field(default_factory=list)

    def node(self, node_id: str) -> Node | None:
        return next((n for n in self.nodes if n.id == node_id), None)

    def resource(self, rid: str) -> Resource | None:
        return next((r for r in self.resources if r.id == rid), None)

    def artifact(self, aid: str) -> Artifact | None:
        return next((a for a in self.artifacts if a.id == aid), None)

    def incoming(self, node_id: str, forward_only: bool = True) -> list[Edge]:
        return [
            e
            for e in self.edges
            if e.to == node_id and (e.is_forward or not forward_only)
        ]

    def outgoing(self, node_id: str, edge_type: str | None = None) -> list[Edge]:
        return [
            e
            for e in self.edges
            if e.from_ == node_id and (edge_type is None or e.type == edge_type)
        ]

    def entry_set(self) -> list[str]:
        """§2.5 — where execution begins.

        A node reachable only by an `error` or `compensation` edge is a
        HANDLER, not a start, so it is excluded even though it has no incoming
        forward edge.
        """
        if self.entry:
            return [self.entry]
        return [
            n.id
            for n in self.nodes
            if not self.incoming(n.id, forward_only=False)
        ]
