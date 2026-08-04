"""The validation rules of specification chapter 13.

Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0

Every rule here carries the code the specification assigns it, and every code has
a conformance fixture that must be rejected with exactly that code. This module
is a deliberate re-implementation of the same rules as the Rust reference
validator — if the two disagree, the conformance suite says which is wrong.
"""

from __future__ import annotations

import re

from .diag import Diagnostics
from .model import EDGE_TYPES, NODE_TYPES, Harness

#: Known key prefixes. Detection is necessarily heuristic (§13.3), so a
#: confident hit is an error and a suspicious one is a warning.
_CREDENTIAL_PREFIXES = (
    "sk-ant-", "sk-", "AKIA", "ghp_", "github_pat_", "xoxb-", "AIza", "-----BEGIN",
)
_SECRET_NAMES = ("key", "secret", "token", "password", "passwd", "credential")

_ID_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_.\-]*$")


def validate(h: Harness, d: Diagnostics) -> None:
    _document(h, d)
    _identifiers(h, d)
    _references(h, d)
    _node_shape(h, d)
    _ports_and_edges(h, d)
    _graph(h, d)
    _policy(h, d)
    _credentials(h, d)


# ------------------------------------------------------------------ HX-1xxx

def _document(h: Harness, d: Diagnostics) -> None:
    if h.spec_version is None:
        d.error(
            "HX-1002", 1,
            "<harness> has no specVersion; a runtime cannot safely guess which "
            "semantics apply, and guessing wrong is worse than refusing",
        )
    if not h.nodes:
        d.error("HX-1102", 1, "<nodes> must contain at least one <node>")
    for n in h.nodes:
        if n.type not in NODE_TYPES:
            d.error(
                "HX-1003", n.line,
                f"node '{n.id}': unrecognised type '{n.type}'. A runtime must "
                f"refuse rather than skip it",
            )
        if n.id and not _ID_RE.match(n.id):
            d.error("HX-1001", n.line, f"node id '{n.id}' is not a valid identifier")


def _identifiers(h: Harness, d: Diagnostics) -> None:
    def dup(kind: str, items, get_id, get_line) -> None:
        seen: set[str] = set()
        for it in items:
            i = get_id(it)
            if not i:
                continue
            if i in seen:
                d.error(
                    "HX-1101", get_line(it),
                    f"duplicate {kind} id '{i}'; every reference to it would be ambiguous",
                )
            seen.add(i)

    dup("node", h.nodes, lambda x: x.id, lambda x: x.line)
    dup("resource", h.resources, lambda x: x.id, lambda x: x.line)
    dup("artifact", h.artifacts, lambda x: x.id, lambda x: x.line)
    dup("edge", [e for e in h.edges if e.id], lambda x: x.id, lambda x: x.line)

    for n in h.nodes:
        for direction, ports in (("input", n.inputs), ("output", n.outputs)):
            seen = set()
            for p in ports:
                if p.name in seen:
                    d.error(
                        "HX-1101", p.line,
                        f"node '{n.id}': duplicate {direction} port '{p.name}'",
                    )
                seen.add(p.name)


# ------------------------------------------------------------------ HX-2xxx

def _references(h: Harness, d: Diagnostics) -> None:
    nodes = {n.id for n in h.nodes}
    resources = {r.id for r in h.resources}
    artifacts = {a.id for a in h.artifacts}

    for e in h.edges:
        label = f" '{e.id}'" if e.id else ""
        for which, target in (("from", e.from_), ("to", e.to)):
            if target not in nodes:
                d.error(
                    "HX-2001", e.line,
                    f"edge{label}: {which} names '{target}', which is not a declared node",
                )

    for n in h.nodes:
        if n.cases:
            for _, to in n.cases.cases:
                if to not in nodes:
                    d.error("HX-2001", n.cases.line,
                            f"node '{n.id}': case targets '{to}', which is not a declared node")
            if n.cases.otherwise and n.cases.otherwise not in nodes:
                d.error("HX-2001", n.cases.line,
                        f"node '{n.id}': otherwise targets '{n.cases.otherwise}', "
                        f"which is not a declared node")
        if n.loop and n.loop.body and n.loop.body not in nodes:
            d.error("HX-2001", n.loop.line,
                    f"node '{n.id}': loop body references '{n.loop.body}', "
                    f"which is not a declared node")
        for r in n.resource_refs:
            if r.target not in resources:
                d.error("HX-2002", r.line,
                        f"node '{n.id}': resourceRef '{r.target}' is not a declared resource")
        for a in n.artifact_refs:
            if a.target not in artifacts:
                d.error("HX-2003", a.line,
                        f"node '{n.id}': artifactRef '{a.target}' is not a declared artifact")
        if n.compensates and n.compensates not in nodes:
            d.error("HX-2004", n.line,
                    f"node '{n.id}': compensates '{n.compensates}', which is not a declared node")


def _node_shape(h: Harness, d: Diagnostics) -> None:
    for n in h.nodes:
        for elem, attr, code in (
            ("cases", "decision", "HX-2201"),
            ("loop", "loop", "HX-2202"),
            ("subworkflow_href", "subworkflow", "HX-2203"),
            ("wait", "wait", "HX-2204"),
        ):
            present = getattr(n, elem) is not None
            expected = n.type == attr
            if present != expected:
                name = {"subworkflow_href": "subworkflow"}.get(elem, elem)
                if expected:
                    d.error(code, n.line, f"node '{n.id}': type=\"{attr}\" requires <{name}>")
                else:
                    d.error(code, n.line,
                            f"node '{n.id}': <{name}> belongs on type=\"{attr}\" and nowhere "
                            f"else (this node is \"{n.type}\")")

        if n.wait:
            count = sum(x is not None for x in (n.wait.duration, n.wait.until, n.wait.event))
            if count != 1:
                d.error("HX-2205", n.wait.line,
                        f"node '{n.id}': <wait> must declare exactly one of duration, "
                        f"until or event (found {count})")

        if n.cases:
            if not n.cases.cases:
                d.error("HX-2206", n.cases.line,
                        f"node '{n.id}': <cases> must contain at least one <case>")
            if n.cases.otherwise is None:
                d.warning("HX-4103", n.cases.line,
                          f"node '{n.id}': no <otherwise>; if no case matches at runtime "
                          f"this fails with HX-4103")

        if n.loop:
            lp = n.loop
            if lp.max_iterations is None:
                d.error("HX-1001", lp.line,
                        f"node '{n.id}': loop has no maxIterations. There is no unbounded "
                        f"form — an unbounded loop in an unattended workflow is a defect")
            if lp.body is None:
                d.error("HX-1001", lp.line, f"node '{n.id}': <loop> has no <body>")
            missing = {
                "forEach": ("over", lp.over),
                "while": ("while", lp.while_expr),
                "until": ("while", lp.while_expr),
                "times": ("count", lp.count),
            }.get(lp.kind)
            if missing is None:
                d.error("HX-1003", lp.line, f"node '{n.id}': unrecognised loop kind '{lp.kind}'")
            elif missing[1] is None:
                d.error("HX-2207", lp.line,
                        f"node '{n.id}': loop kind=\"{lp.kind}\" requires the "
                        f"'{missing[0]}' attribute")
            if lp.count is not None and lp.max_iterations is not None and lp.count > lp.max_iterations:
                d.error("HX-2208", lp.line,
                        f"node '{n.id}': count {lp.count} exceeds maxIterations "
                        f"{lp.max_iterations}; the document states two different bounds")

        incoming = len(h.incoming(n.id))
        if n.join_policy == "quorum":
            if n.quorum is None:
                d.error("HX-2401", n.line,
                        f"node '{n.id}': joinPolicy=\"quorum\" requires @quorum")
            elif n.quorum > incoming:
                d.error("HX-2402", n.line,
                        f"node '{n.id}': quorum {n.quorum} exceeds its {incoming} incoming "
                        f"edge(s); it can never be satisfied")

        if n.type == "inference":
            has_model = any(
                (r := h.resource(ref.target)) is not None and r.type == "model"
                for ref in n.resource_refs
            )
            if not has_model:
                d.error("HX-2501", n.line,
                        f"node '{n.id}': type=\"inference\" must reference a resource "
                        f"of type=\"model\"")


def _ports_and_edges(h: Harness, d: Diagnostics) -> None:
    fed: dict[tuple[str, str], int] = {}

    for e in h.edges:
        if e.type != "data":
            continue
        if e.from_port is None or e.to_port is None:
            d.error("HX-2301", e.line,
                    f"edge {e.from_} -> {e.to}: a data edge must declare both "
                    f"fromPort and toPort")
            continue
        src = h.node(e.from_)
        dst = h.node(e.to)
        if src and src.output(e.from_port) is None:
            d.error("HX-2302", e.line,
                    f"edge {e.from_} -> {e.to}: fromPort '{e.from_port}' is not an "
                    f"output on '{e.from_}'")
        if dst and dst.input(e.to_port) is None:
            d.error("HX-2303", e.line,
                    f"edge {e.from_} -> {e.to}: toPort '{e.to_port}' is not an "
                    f"input on '{e.to}'")
        fed[(e.to, e.to_port)] = fed.get((e.to, e.to_port), 0) + 1

        # HX-3201 — checked only when BOTH ports declare a type. Untyped means
        # unchecked, not "any".
        if src and dst:
            sp, dp = src.output(e.from_port), dst.input(e.to_port)
            if sp and dp and sp.type and dp.type and sp.type != dp.type and dp.type != "json":
                d.error("HX-3201", e.line,
                        f"edge {e.from_} -> {e.to}: type '{sp.type}' is not compatible "
                        f"with '{dp.type}'")

    for (node_id, port), count in fed.items():
        if count > 1:
            n = h.node(node_id)
            d.error("HX-2304", n.line if n else 1,
                    f"node '{node_id}': input '{port}' is fed by {count} data edges; "
                    f"there is no defined winner")

    for n in h.nodes:
        for p in n.inputs:
            if not p.required:
                continue
            by_edge = (n.id, p.name) in fed
            if not by_edge and not p.has_value and not p.has_default:
                d.error("HX-2101", p.line,
                        f"node '{n.id}': required input '{p.name}' is satisfied by "
                        f"neither a data edge nor a value")
            if by_edge and p.has_value:
                d.error("HX-2102", p.line,
                        f"node '{n.id}': input '{p.name}' has both a data edge and a "
                        f"value; a reader cannot tell which wins")


# ------------------------------------------------------------------ HX-3xxx

def _graph(h: Harness, d: Diagnostics) -> None:
    entry = h.entry_set()
    if not entry and h.nodes:
        d.error("HX-3001", 1,
                "the entry set is empty: every node waits for another, so nothing can begin")

    # HX-3003 — acyclicity over forward edges only.
    adj: dict[str, list[str]] = {}
    for e in h.edges:
        if e.is_forward:
            adj.setdefault(e.from_, []).append(e.to)

    WHITE, GREY, BLACK = 0, 1, 2
    mark = {n.id: WHITE for n in h.nodes}
    cycle: list[str] | None = None

    def dfs(at: str, stack: list[str]) -> None:
        nonlocal cycle
        if cycle:
            return
        mark[at] = GREY
        stack.append(at)
        for nxt in adj.get(at, []):
            state = mark.get(nxt, WHITE)
            if state == GREY:
                start = stack.index(nxt) if nxt in stack else 0
                cycle = stack[start:] + [nxt]
                return
            if state == WHITE:
                dfs(nxt, stack)
                if cycle:
                    return
        stack.pop()
        mark[at] = BLACK

    for n in h.nodes:
        if mark.get(n.id, WHITE) == WHITE:
            dfs(n.id, [])

    if cycle:
        d.error("HX-3003", 1,
                f"control flow contains a cycle: {' -> '.join(cycle)}. "
                f"Use a loop node, which carries a required bound")

    # HX-3004 — a loop body must not be sequenced from outside the loop.
    # A DATA edge into a body is explicitly permitted: that is how a
    # loop-invariant input is bound, and forbidding it would make loops
    # nearly unusable.
    for n in h.nodes:
        if n.loop and n.loop.body:
            if any(
                e.to == n.loop.body and e.from_ != n.id and e.type in ("control", "dependency")
                for e in h.edges
            ):
                d.error("HX-3004", n.loop.line,
                        f"node '{n.id}': loop body '{n.loop.body}' is also sequenced by a "
                        f"control or dependency edge from outside the loop; it would run "
                        f"at the wrong time")

    # HX-2005 / HX-2004 — compensation targets.
    for e in h.edges:
        if e.type != "compensation":
            continue
        if any(f.to == e.to and f.type in ("control", "data") for f in h.edges):
            d.error("HX-2005", e.line,
                    f"node '{e.to}' is a compensation target but is also reachable by a "
                    f"forward edge; it will eventually run at the wrong time")
        target = h.node(e.to)
        if target and target.compensates and target.compensates != e.from_:
            d.error("HX-2004", e.line,
                    f"node '{e.to}' declares compensates=\"{target.compensates}\" but a "
                    f"compensation edge arrives from '{e.from_}'")

    # HX-3005 — reachability. A WARNING: legitimate mid-authoring.
    seen: set[str] = set()
    queue = list(entry)
    while queue:
        at = queue.pop()
        if at in seen:
            continue
        seen.add(at)
        queue.extend(e.to for e in h.edges if e.from_ == at)
        n = h.node(at)
        if n:
            if n.cases:
                queue.extend(to for _, to in n.cases.cases)
                if n.cases.otherwise:
                    queue.append(n.cases.otherwise)
            if n.loop and n.loop.body:
                queue.append(n.loop.body)
    for n in h.nodes:
        if n.id not in seen:
            d.warning("HX-3005", n.line, f"node '{n.id}' is not reachable from the entry set")


def _policy(h: Harness, d: Diagnostics) -> None:
    for n in h.nodes:
        if not n.idempotent and n.retry is not None:
            d.error("HX-3301", n.line,
                    f"node '{n.id}' is declared idempotent=\"false\" but carries a retry "
                    f"policy; retrying it duplicates its effect")
        durations = []
        if n.timeout:
            durations.append(("timeout", n.timeout.duration))
        if n.wait and n.wait.duration:
            durations.append(("wait", n.wait.duration))
        if n.retry:
            durations.append(("retry initialDelay", n.retry.initial_delay))
            if n.retry.max_delay:
                durations.append(("retry maxDelay", n.retry.max_delay))
        for where, dur in durations:
            date_part = dur.split("T")[0]
            if "Y" in date_part or "M" in date_part:
                d.error("HX-3401", n.line,
                        f"node '{n.id}': {where} duration '{dur}' uses months or years, "
                        f"whose length is not fixed")


def _credentials(h: Harness, d: Diagnostics) -> None:
    """HX-3501 — a literal credential in a document designed to be committed to
    git, diffed in pull requests and archived for audit."""
    scalars: list[tuple[str, str, str, int]] = []
    for r in h.resources:
        for k, v in r.properties:
            scalars.append((f"resource '{r.id}' property '{k}'", k, v, r.line))
    for a in h.artifacts:
        for k, v in a.properties:
            scalars.append((f"artifact '{a.id}' property '{k}'", k, v, a.line))
    for n in h.nodes:
        for k, v in n.config:
            scalars.append((f"node '{n.id}' config '{k}'", k, v, n.line))
        for p in n.inputs:
            if p.value:
                scalars.append((f"node '{n.id}' input '{p.name}'", p.name, p.value, p.line))

    for context, name, value, line in scalars:
        v = (value or "").strip()
        # An expression or a reference is exactly what the format wants.
        if not v or v.startswith("${"):
            continue
        if any(v.startswith(p) for p in _CREDENTIAL_PREFIXES):
            d.error("HX-3501", line,
                    f"{context} appears to contain a literal credential. Use "
                    f"<credential ref=\"…\" store=\"…\"/> instead")
            continue
        if any(s in name.lower() for s in _SECRET_NAMES) and len(v) >= 20:
            d.warning("HX-3501", line,
                      f"{context} is named like a secret and holds a long literal; "
                      f"if it is a credential, reference it instead")
