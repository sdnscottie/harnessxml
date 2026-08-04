"""`.hxml` -> Harness.

Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0

Namespace-aware per §2.6: elements are matched on namespace URI plus local name,
never on prefix.

Per §2.8 an unrecognised element in the HarnessXML namespace is an ERROR
(HX-1003), not something to skip. That rule is enforced here, in the parser,
rather than left to the validator — a construct the parser silently dropped is
one the validator can never see.

Standard library only: xml.etree.ElementTree, with a line-number-tracking
parser so diagnostics can point at the offending element.
"""

from __future__ import annotations

import xml.etree.ElementTree as ET

from .diag import Diagnostics
from .model import (
    NS,
    Artifact,
    Cases,
    Edge,
    Harness,
    Loop,
    Metadata,
    Node,
    Port,
    Ref,
    Resource,
    Retry,
    Timeout,
    Wait,
)


def _annotate_lines(root: ET.Element, text: str) -> None:
    """Attach source line numbers to every element.

    ElementTree drops them, and subclassing XMLParser._start does not work
    because the C accelerator bypasses it — which is why an earlier version of
    this module reported every finding at line 0. A diagnostic without a
    location is a rule number, not a diagnostic (§13.5).

    So: run expat over the source to collect start-element lines in document
    order, then zip them against a depth-first preorder walk of the tree, which
    is the same order.
    """
    import xml.parsers.expat

    lines: list[int] = []
    parser = xml.parsers.expat.ParserCreate(namespace_separator="}")

    def start(_name, _attrs):
        lines.append(parser.CurrentLineNumber)

    parser.StartElementHandler = start
    try:
        parser.Parse(text.encode("utf-8"), True)
    except xml.parsers.expat.ExpatError:
        return  # already reported by ElementTree; leave lines at 0

    for el, line in zip(root.iter(), lines):
        el.set("__line__", str(line))


def _line(el: ET.Element) -> int:
    try:
        return int(el.get("__line__", "0"))
    except ValueError:
        return 0


def _local(tag: str) -> tuple[str | None, str]:
    """Split `{ns}local` into (ns, local)."""
    if tag.startswith("{"):
        ns, _, local = tag[1:].partition("}")
        return ns, local
    return None, tag


def _bool(el: ET.Element, key: str, default: bool) -> bool:
    v = el.get(key)
    if v == "true":
        return True
    if v == "false":
        return False
    return default


def _int(el: ET.Element, key: str) -> int | None:
    v = el.get(key)
    if v is None:
        return None
    try:
        return int(v)
    except ValueError:
        return None


def _float(el: ET.Element, key: str, default: float) -> float:
    v = el.get(key)
    if v is None:
        return default
    try:
        return float(v)
    except ValueError:
        return default


#: Elements whose only job is to contain others.
_CONTAINERS = {
    "metadata", "provenance", "tags", "security", "permission", "generator",
    "source", "signature", "resources", "artifacts", "nodes", "edges",
    "inputs", "outputs", "config", "description", "credential", "tag",
    "title", "author", "organization", "created", "modified", "license",
    "documentVersion",
}


def parse(text: str, diags: Diagnostics) -> Harness | None:
    """Parse a HarnessXML document. Returns None if it is not well-formed or
    has no `<harness>` root."""
    try:
        root = ET.fromstring(text)
    except ET.ParseError as e:
        line = e.position[0] if e.position else 1
        diags.error("HX-1001", line, f"not well-formed: {e}")
        return None
    _annotate_lines(root, text)

    ns, local = _local(root.tag)
    if local != "harness" or ns != NS:
        diags.error(
            "HX-1001",
            _line(root),
            f"root element is <{local}> in namespace {ns!r}; "
            f"expected <harness> in {NS!r}",
        )
        return None

    h = Harness(
        id=root.get("id", ""),
        spec_version=root.get("specVersion"),
        name=root.get("name"),
        entry=root.get("entry"),
    )

    for child in root:
        cns, cl = _local(child.tag)
        if cns != NS:
            diags.error(
                "HX-1006",
                _line(child),
                f"element <{cl}> is from a foreign namespace and is only "
                f"permitted inside <extension>",
            )
            continue
        if cl == "metadata":
            _metadata(child, h)
        elif cl == "security":
            pass
        elif cl == "resources":
            _resources(child, h, diags)
        elif cl == "artifacts":
            _artifacts(child, h, diags)
        elif cl == "nodes":
            _nodes(child, h, diags)
        elif cl == "edges":
            _edges(child, h, diags)
        elif cl == "extension":
            pass  # vendor content is not ours to interpret
        else:
            diags.error(
                "HX-1003",
                _line(child),
                f"unrecognised element <{cl}> inside <harness>; "
                f"an unknown construct must be rejected, never skipped",
            )
    return h


def _metadata(el: ET.Element, h: Harness) -> None:
    m = Metadata()
    for c in el:
        _, cl = _local(c.tag)
        text = (c.text or "").strip()
        if cl == "title":
            m.title = text
        elif cl == "description":
            m.description = text
        elif cl == "author":
            m.author = text
        elif cl == "organization":
            m.organization = text
        elif cl == "created":
            m.created = text
        elif cl == "modified":
            m.modified = text
        elif cl == "license":
            m.license = text
        elif cl == "documentVersion":
            m.document_version = text
        elif cl == "tags":
            m.tags = [(t.text or "").strip() for t in c]
    h.metadata = m


def _resources(el: ET.Element, h: Harness, diags: Diagnostics) -> None:
    for c in el:
        _, cl = _local(c.tag)
        if cl != "resource":
            diags.error("HX-1003", _line(c), f"unrecognised element <{cl}> inside <resources>")
            continue
        r = Resource(
            id=c.get("id", ""),
            type=c.get("type", ""),
            name=c.get("name"),
            provider=c.get("provider"),
            uri=c.get("uri"),
            line=_line(c),
        )
        for g in c:
            _, gl = _local(g.tag)
            if gl == "property":
                r.properties.append((g.get("name", ""), g.get("value", "")))
            elif gl == "credential":
                r.credential_ref = g.get("ref")
                r.credential_store = g.get("store")
        h.resources.append(r)


def _artifacts(el: ET.Element, h: Harness, diags: Diagnostics) -> None:
    for c in el:
        _, cl = _local(c.tag)
        if cl != "artifact":
            diags.error("HX-1003", _line(c), f"unrecognised element <{cl}> inside <artifacts>")
            continue
        a = Artifact(
            id=c.get("id", ""),
            type=c.get("type", ""),
            name=c.get("name"),
            uri=c.get("uri"),
            media_type=c.get("mediaType"),
            digest=c.get("digest"),
            classification=c.get("classification"),
            line=_line(c),
        )
        for g in c:
            _, gl = _local(g.tag)
            if gl == "property":
                a.properties.append((g.get("name", ""), g.get("value", "")))
        h.artifacts.append(a)


def _ports(el: ET.Element, want: str) -> list[Port]:
    out = []
    for c in el:
        _, cl = _local(c.tag)
        if cl != want:
            continue
        out.append(
            Port(
                name=c.get("name", ""),
                type=c.get("type"),
                required=_bool(c, "required", True),
                default=c.get("default"),
                value=c.get("value"),
                line=_line(c),
            )
        )
    return out


def _nodes(el: ET.Element, h: Harness, diags: Diagnostics) -> None:
    for c in el:
        _, cl = _local(c.tag)
        if cl != "node":
            diags.error("HX-1003", _line(c), f"unrecognised element <{cl}> inside <nodes>")
            continue
        n = Node(
            id=c.get("id", ""),
            type=c.get("type", ""),
            name=c.get("name"),
            impl=c.get("impl"),
            idempotent=_bool(c, "idempotent", True),
            join_policy=c.get("joinPolicy", "all"),
            quorum=_int(c, "quorum"),
            compensates=c.get("compensates"),
            line=_line(c),
        )
        for g in c:
            gns, gl = _local(g.tag)
            if gns != NS:
                diags.error("HX-1006", _line(g), f"foreign element <{gl}> outside <extension>")
                continue
            if gl == "description":
                n.description = (g.text or "").strip()
            elif gl == "inputs":
                n.inputs = _ports(g, "input")
            elif gl == "outputs":
                n.outputs = _ports(g, "output")
            elif gl == "config":
                n.config = [
                    (p.get("name", ""), p.get("value", ""))
                    for p in g
                    if _local(p.tag)[1] == "property"
                ]
            elif gl == "resourceRef":
                n.resource_refs.append(Ref(g.get("ref", ""), _line(g)))
            elif gl == "artifactRef":
                n.artifact_refs.append(Ref(g.get("ref", ""), _line(g)))
            elif gl == "guard":
                n.guard = g.get("when", "")
            elif gl == "retry":
                n.retry = Retry(
                    max_attempts=_int(g, "maxAttempts") or 1,
                    backoff=g.get("backoff", "exponential"),
                    initial_delay=g.get("initialDelay", "PT1S"),
                    max_delay=g.get("maxDelay"),
                    multiplier=_float(g, "multiplier", 2.0),
                    jitter=_bool(g, "jitter", True),
                    retry_on=(g.get("retryOn") or "").split(),
                )
            elif gl == "timeout":
                n.timeout = Timeout(
                    duration=g.get("duration", ""),
                    on_timeout=g.get("onTimeout", "fail"),
                )
            elif gl == "cases":
                cs = Cases(line=_line(g))
                for k in g:
                    _, kl = _local(k.tag)
                    if kl == "case":
                        cs.cases.append((k.get("when", ""), k.get("to", "")))
                    elif kl == "otherwise":
                        cs.otherwise = k.get("to")
                n.cases = cs
            elif gl == "loop":
                lp = Loop(
                    kind=g.get("kind", ""),
                    over=g.get("over"),
                    while_expr=g.get("while"),
                    count=_int(g, "count"),
                    max_iterations=_int(g, "maxIterations"),
                    var=g.get("var", "item"),
                    index_var=g.get("indexVar", "index"),
                    max_concurrency=_int(g, "maxConcurrency") or 1,
                    on_item_failure=g.get("onItemFailure", "fail"),
                    line=_line(g),
                )
                for k in g:
                    if _local(k.tag)[1] == "body":
                        lp.body = k.get("ref")
                n.loop = lp
            elif gl == "subworkflow":
                n.subworkflow_href = g.get("href")
            elif gl == "wait":
                n.wait = Wait(
                    duration=g.get("duration"),
                    until=g.get("until"),
                    event=g.get("event"),
                    line=_line(g),
                )
            elif gl == "security":
                pass
            elif gl == "extension":
                n.extensions.append(
                    (g.get("namespace", ""), g.get("required") == "true")
                )
            else:
                diags.error(
                    "HX-1003",
                    _line(g),
                    f"unrecognised element <{gl}> inside <node id='{n.id}'>",
                )
        h.nodes.append(n)


def _edges(el: ET.Element, h: Harness, diags: Diagnostics) -> None:
    for c in el:
        _, cl = _local(c.tag)
        if cl != "edge":
            diags.error("HX-1003", _line(c), f"unrecognised element <{cl}> inside <edges>")
            continue
        ty = c.get("type", "control")
        from .model import EDGE_TYPES

        if ty not in EDGE_TYPES:
            diags.error("HX-1003", _line(c), f"unrecognised edge type '{ty}'")
            continue
        h.edges.append(
            Edge(
                id=c.get("id"),
                from_=c.get("from", ""),
                to=c.get("to", ""),
                type=ty,
                from_port=c.get("fromPort"),
                to_port=c.get("toPort"),
                condition=c.get("condition"),
                line=_line(c),
            )
        )
