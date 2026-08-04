"""Building and serialising HarnessXML documents.

Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0

A validator tells you a document is wrong. An SDK's real job is making the right
document easy to produce — so the builder refuses to emit constructs the
specification forbids, rather than letting you write them and finding out later.

    from harnessxml import Builder

    b = Builder("document_triage", entry="receive")
    b.resource("classifier", "model", provider="anthropic",
               properties={"model": "claude-opus-5"},
               credential="ANTHROPIC_API_KEY", credential_store="vault")
    b.node("receive", "source").output("document", "binary")
    b.node("classify", "inference").resource_ref("classifier", role="model") \\
        .input("document", "binary").output("confidence", "number") \\
        .retry(max_attempts=4, retry_on=["rate_limit", "transient"])
    b.decision("route").case("${classify.confidence >= 0.9}", "auto").otherwise("review")
    b.node("auto", "task"); b.node("review", "human", idempotent=False)
    b.data("receive", "document", "classify", "document")
    b.control("classify", "route")

    print(b.to_xml())      # raises if the document would be invalid
"""

from __future__ import annotations

import xml.etree.ElementTree as ET
from xml.dom import minidom

from .diag import Diagnostics, HarnessXMLError
from .model import NS, Harness
from .validate import validate

SCHEMA_LOCATION = "https://harnessxml.com/schema/v1.0/harnessxml-1.0.xsd"


class NodeBuilder:
    """Chainable node configuration. Returned by :meth:`Builder.node`."""

    def __init__(self, el: ET.Element, parent: "Builder") -> None:
        self._el = el
        self._parent = parent
        self._sections: dict[str, ET.Element] = {}

    # -- ordering ------------------------------------------------------
    # Child element order is schema-enforced, so the builder maintains it for
    # you rather than letting you discover it from a validation error.
    _ORDER = [
        "description", "inputs", "outputs", "config", "resourceRef",
        "artifactRef", "guard", "retry", "timeout", "cases", "loop",
        "subworkflow", "wait", "security", "extension",
    ]

    def _place(self, el: ET.Element) -> None:
        tag = el.tag.split("}")[-1]
        rank = self._ORDER.index(tag)
        for i, existing in enumerate(list(self._el)):
            etag = existing.tag.split("}")[-1]
            if self._ORDER.index(etag) > rank:
                self._el.insert(i, el)
                return
        self._el.append(el)

    def _section(self, tag: str) -> ET.Element:
        if tag not in self._sections:
            el = ET.Element(f"{{{NS}}}{tag}")
            self._sections[tag] = el
            self._place(el)
        return self._sections[tag]

    # -- API -----------------------------------------------------------
    def description(self, text: str) -> "NodeBuilder":
        el = ET.Element(f"{{{NS}}}description")
        el.text = text
        self._place(el)
        return self

    def input(self, name: str, type: str | None = None, *, required: bool = True,
              value: str | None = None, default: str | None = None) -> "NodeBuilder":
        el = ET.SubElement(self._section("inputs"), f"{{{NS}}}input", {"name": name})
        if type:
            el.set("type", type)
        if not required:
            el.set("required", "false")
        if value is not None:
            el.set("value", value)
        if default is not None:
            el.set("default", default)
        return self

    def output(self, name: str, type: str | None = None) -> "NodeBuilder":
        el = ET.SubElement(self._section("outputs"), f"{{{NS}}}output", {"name": name})
        if type:
            el.set("type", type)
        return self

    def config(self, **properties: object) -> "NodeBuilder":
        section = self._section("config")
        for k, v in properties.items():
            ET.SubElement(section, f"{{{NS}}}property", {"name": k, "value": str(v)})
        return self

    def resource_ref(self, ref: str, role: str | None = None) -> "NodeBuilder":
        el = ET.Element(f"{{{NS}}}resourceRef", {"ref": ref})
        if role:
            el.set("role", role)
        self._place(el)
        return self

    def artifact_ref(self, ref: str, direction: str = "in") -> "NodeBuilder":
        self._place(ET.Element(f"{{{NS}}}artifactRef", {"ref": ref, "direction": direction}))
        return self

    def guard(self, when: str) -> "NodeBuilder":
        self._place(ET.Element(f"{{{NS}}}guard", {"when": when}))
        return self

    def retry(self, max_attempts: int, *, backoff: str = "exponential",
              initial_delay: str = "PT1S", max_delay: str | None = None,
              multiplier: float | None = None, jitter: bool | None = None,
              retry_on: list[str] | None = None) -> "NodeBuilder":
        if self._el.get("idempotent") == "false":
            # §8.3 / HX-3301 — the combination is a contradiction. Refusing it
            # here beats emitting a document a validator will reject.
            raise ValueError(
                f"node '{self._el.get('id')}': a retry policy on a node declared "
                f"idempotent=\"false\" is invalid (HX-3301) — retrying it duplicates "
                f"its effect. Put the retry on the idempotent part instead."
            )
        attrs = {"maxAttempts": str(max_attempts), "backoff": backoff,
                 "initialDelay": initial_delay}
        if max_delay:
            attrs["maxDelay"] = max_delay
        if multiplier is not None:
            attrs["multiplier"] = str(multiplier)
        if jitter is not None:
            attrs["jitter"] = "true" if jitter else "false"
        if retry_on:
            attrs["retryOn"] = " ".join(retry_on)
        self._place(ET.Element(f"{{{NS}}}retry", attrs))
        return self

    def timeout(self, duration: str, on_timeout: str = "fail") -> "NodeBuilder":
        self._place(ET.Element(f"{{{NS}}}timeout",
                               {"duration": duration, "onTimeout": on_timeout}))
        return self

    def case(self, when: str, to: str) -> "NodeBuilder":
        ET.SubElement(self._section("cases"), f"{{{NS}}}case", {"when": when, "to": to})
        return self

    def otherwise(self, to: str) -> "NodeBuilder":
        ET.SubElement(self._section("cases"), f"{{{NS}}}otherwise", {"to": to})
        return self

    def loop(self, kind: str, body: str, max_iterations: int, *, over: str | None = None,
             while_expr: str | None = None, count: int | None = None,
             var: str | None = None, index_var: str | None = None,
             max_concurrency: int | None = None,
             on_item_failure: str | None = None) -> "NodeBuilder":
        if max_iterations is None:
            raise ValueError("maxIterations is REQUIRED — there is no unbounded loop")
        attrs = {"kind": kind, "maxIterations": str(max_iterations)}
        if over:
            attrs["over"] = over
        if while_expr:
            attrs["while"] = while_expr
        if count is not None:
            attrs["count"] = str(count)
        if var:
            attrs["var"] = var
        if index_var:
            attrs["indexVar"] = index_var
        if max_concurrency is not None:
            attrs["maxConcurrency"] = str(max_concurrency)
        if on_item_failure:
            attrs["onItemFailure"] = on_item_failure
        el = ET.Element(f"{{{NS}}}loop", attrs)
        ET.SubElement(el, f"{{{NS}}}body", {"ref": body})
        self._place(el)
        return self

    def wait(self, *, duration: str | None = None, until: str | None = None,
             event: str | None = None) -> "NodeBuilder":
        given = [x for x in (duration, until, event) if x is not None]
        if len(given) != 1:
            raise ValueError("<wait> must declare exactly one of duration, until or event")
        attrs = {}
        if duration:
            attrs["duration"] = duration
        if until:
            attrs["until"] = until
        if event:
            attrs["event"] = event
        self._place(ET.Element(f"{{{NS}}}wait", attrs))
        return self

    def subworkflow(self, href: str, digest: str | None = None) -> "NodeBuilder":
        attrs = {"href": href}
        if digest:
            attrs["digest"] = digest
        self._place(ET.Element(f"{{{NS}}}subworkflow", attrs))
        return self


class Builder:
    """Constructs a HarnessXML document."""

    def __init__(self, harness_id: str, *, spec_version: str = "1.0",
                 name: str | None = None, entry: str | None = None) -> None:
        self.root = ET.Element(f"{{{NS}}}harness", {
            "id": harness_id, "specVersion": spec_version,
        })
        if name:
            self.root.set("name", name)
        if entry:
            self.root.set("entry", entry)
        self._sections: dict[str, ET.Element] = {}
        self._order = ["metadata", "security", "resources", "artifacts", "nodes", "edges"]

    def _section(self, tag: str) -> ET.Element:
        if tag not in self._sections:
            el = ET.Element(f"{{{NS}}}{tag}")
            rank = self._order.index(tag)
            placed = False
            for i, existing in enumerate(list(self.root)):
                etag = existing.tag.split("}")[-1]
                if etag in self._order and self._order.index(etag) > rank:
                    self.root.insert(i, el)
                    placed = True
                    break
            if not placed:
                self.root.append(el)
            self._sections[tag] = el
        return self._sections[tag]

    def metadata(self, **fields: str) -> "Builder":
        section = self._section("metadata")
        for k, v in fields.items():
            tag = {"document_version": "documentVersion"}.get(k, k)
            el = ET.SubElement(section, f"{{{NS}}}{tag}")
            el.text = v
        return self

    def resource(self, rid: str, type: str, *, name: str | None = None,
                 provider: str | None = None, uri: str | None = None,
                 properties: dict[str, object] | None = None,
                 credential: str | None = None,
                 credential_store: str | None = None) -> "Builder":
        attrs = {"id": rid, "type": type}
        for k, v in (("name", name), ("provider", provider), ("uri", uri)):
            if v:
                attrs[k] = v
        el = ET.SubElement(self._section("resources"), f"{{{NS}}}resource", attrs)
        for k, v in (properties or {}).items():
            ET.SubElement(el, f"{{{NS}}}property", {"name": k, "value": str(v)})
        if credential:
            c = {"ref": credential}
            if credential_store:
                c["store"] = credential_store
            ET.SubElement(el, f"{{{NS}}}credential", c)
        return self

    def artifact(self, aid: str, type: str, *, name: str | None = None,
                 uri: str | None = None, media_type: str | None = None,
                 digest: str | None = None,
                 classification: str | None = None) -> "Builder":
        attrs = {"id": aid, "type": type}
        for k, v in (("name", name), ("uri", uri), ("mediaType", media_type),
                     ("digest", digest), ("classification", classification)):
            if v:
                attrs[k] = v
        ET.SubElement(self._section("artifacts"), f"{{{NS}}}artifact", attrs)
        return self

    def node(self, node_id: str, type: str, *, name: str | None = None,
             impl: str | None = None, idempotent: bool | None = None,
             join_policy: str | None = None, quorum: int | None = None,
             compensates: str | None = None) -> NodeBuilder:
        attrs = {"id": node_id, "type": type}
        if name:
            attrs["name"] = name
        if impl:
            attrs["impl"] = impl
        if idempotent is not None:
            attrs["idempotent"] = "true" if idempotent else "false"
        if join_policy:
            attrs["joinPolicy"] = join_policy
        if quorum is not None:
            attrs["quorum"] = str(quorum)
        if compensates:
            attrs["compensates"] = compensates
        el = ET.SubElement(self._section("nodes"), f"{{{NS}}}node", attrs)
        return NodeBuilder(el, self)

    def decision(self, node_id: str, *, name: str | None = None) -> NodeBuilder:
        return self.node(node_id, "decision", name=name)

    # -- edges ---------------------------------------------------------
    def edge(self, from_: str, to: str, type: str = "control", *,
             from_port: str | None = None, to_port: str | None = None,
             condition: str | None = None, edge_id: str | None = None) -> "Builder":
        if type == "data" and (from_port is None or to_port is None):
            raise ValueError(
                "a data edge must declare both fromPort and toPort (HX-2301)"
            )
        attrs = {"from": from_, "to": to, "type": type}
        if edge_id:
            attrs["id"] = edge_id
        if from_port:
            attrs["fromPort"] = from_port
        if to_port:
            attrs["toPort"] = to_port
        if condition:
            attrs["condition"] = condition
        ET.SubElement(self._section("edges"), f"{{{NS}}}edge", attrs)
        return self

    def control(self, from_: str, to: str, **kw) -> "Builder":
        return self.edge(from_, to, "control", **kw)

    def data(self, from_: str, from_port: str, to: str, to_port: str, **kw) -> "Builder":
        return self.edge(from_, to, "data", from_port=from_port, to_port=to_port, **kw)

    def dependency(self, from_: str, to: str, **kw) -> "Builder":
        return self.edge(from_, to, "dependency", **kw)

    def error(self, from_: str, to: str, **kw) -> "Builder":
        return self.edge(from_, to, "error", **kw)

    def compensation(self, from_: str, to: str, **kw) -> "Builder":
        return self.edge(from_, to, "compensation", **kw)

    # -- output --------------------------------------------------------
    def to_xml(self, *, validate_first: bool = True, schema_location: bool = True,
               indent: bool = True) -> str:
        """Serialise. Validates first by default — an SDK that happily emits an
        invalid document has moved the error to someone else's build."""
        ET.register_namespace("", NS)
        if schema_location:
            self.root.set(
                "{http://www.w3.org/2001/XMLSchema-instance}schemaLocation",
                f"{NS} {SCHEMA_LOCATION}",
            )
            ET.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance")
        raw = ET.tostring(self.root, encoding="unicode")
        text = '<?xml version="1.0" encoding="UTF-8"?>\n' + raw

        if validate_first:
            from .parse import parse
            d = Diagnostics()
            h = parse(text, d)
            if h is not None:
                validate(h, d)
            if d.has_errors():
                raise HarnessXMLError(d, "<builder>")

        if indent:
            pretty = minidom.parseString(text).toprettyxml(indent="  ")
            # minidom leaves blank lines where whitespace text nodes were.
            return "\n".join(l for l in pretty.split("\n") if l.strip()) + "\n"
        return text

    def build(self) -> Harness:
        """Return the parsed, validated object model."""
        from .parse import parse
        d = Diagnostics()
        h = parse(self.to_xml(validate_first=False, indent=False), d)
        if h is not None:
            validate(h, d)
        if d.has_errors():
            raise HarnessXMLError(d, "<builder>")
        assert h is not None
        return h
