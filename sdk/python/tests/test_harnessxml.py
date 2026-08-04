"""Python SDK tests.

Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0

Run:  python3 -m unittest discover -s sdk/python/tests
"""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import harnessxml
from harnessxml import Builder, HarnessXMLError, check_text

REPO = Path(__file__).resolve().parents[3]

MINIMAL = """<?xml version="1.0"?>
<harness xmlns="https://harnessxml.com/spec/1.0" id="m" specVersion="1.0">
  <nodes><node id="only" type="task" impl="noop"/></nodes>
</harness>"""


def codes(text):
    return [d.code for d in check_text(text).errors]


class TestParsing(unittest.TestCase):
    def test_minimal_is_valid(self):
        h = harnessxml.loads(MINIMAL)
        self.assertEqual(h.id, "m")
        self.assertEqual(len(h.nodes), 1)

    def test_not_well_formed_is_hx_1001(self):
        self.assertIn("HX-1001", codes("<harness"))

    def test_wrong_namespace_is_rejected(self):
        bad = MINIMAL.replace("https://harnessxml.com/spec/1.0", "https://example.com/other")
        self.assertIn("HX-1001", codes(bad))

    def test_prefixed_namespace_is_accepted(self):
        # §2.6 — match on namespace URI and local name, never on prefix.
        prefixed = """<?xml version="1.0"?>
<hx:harness xmlns:hx="https://harnessxml.com/spec/1.0" id="p" specVersion="1.0">
  <hx:nodes><hx:node id="a" type="task"/></hx:nodes>
</hx:harness>"""
        self.assertEqual(harnessxml.loads(prefixed).id, "p")

    def test_diagnostics_carry_a_line_number(self):
        d = check_text(MINIMAL.replace('type="task"', 'type="quantum"'))
        self.assertTrue(all(x.line > 0 for x in d.errors), "a finding with no location is a rule number")


class TestValidation(unittest.TestCase):
    def test_missing_spec_version_is_hx_1002(self):
        self.assertIn("HX-1002", codes(MINIMAL.replace(' specVersion="1.0"', "")))

    def test_unknown_node_type_is_hx_1003(self):
        self.assertIn("HX-1003", codes(MINIMAL.replace('type="task"', 'type="quantum"')))

    def test_unknown_element_is_hx_1003_not_ignored(self):
        self.assertIn("HX-1003", codes(MINIMAL.replace("<nodes>", "<nodes><wibble/>")))

    def test_duplicate_node_id_is_hx_1101(self):
        doc = MINIMAL.replace(
            '<node id="only" type="task" impl="noop"/>',
            '<node id="a" type="task"/><node id="a" type="task"/>')
        self.assertIn("HX-1101", codes(doc))

    def test_dangling_edge_is_hx_2001(self):
        doc = """<harness xmlns="https://harnessxml.com/spec/1.0" id="d" specVersion="1.0">
  <nodes><node id="a" type="task"/></nodes>
  <edges><edge from="a" to="ghost" type="control"/></edges>
</harness>"""
        self.assertIn("HX-2001", codes(doc))

    def test_retry_on_non_idempotent_is_hx_3301(self):
        doc = MINIMAL.replace(
            '<node id="only" type="task" impl="noop"/>',
            '<node id="p" type="task" idempotent="false"><retry maxAttempts="3"/></node>')
        self.assertIn("HX-3301", codes(doc))

    def test_cycle_is_hx_3003(self):
        doc = """<harness xmlns="https://harnessxml.com/spec/1.0" id="c" specVersion="1.0">
  <nodes><node id="a" type="task"/><node id="b" type="task"/></nodes>
  <edges>
    <edge from="a" to="b" type="control"/>
    <edge from="b" to="a" type="control"/>
  </edges>
</harness>"""
        self.assertIn("HX-3003", codes(doc))

    def test_error_edges_do_not_create_a_cycle(self):
        doc = """<harness xmlns="https://harnessxml.com/spec/1.0" id="e" specVersion="1.0">
  <nodes><node id="a" type="task"/><node id="b" type="task"/></nodes>
  <edges>
    <edge from="a" to="b" type="control"/>
    <edge from="b" to="a" type="error"/>
  </edges>
</harness>"""
        self.assertNotIn("HX-3003", codes(doc))

    def test_unbounded_loop_is_rejected(self):
        doc = """<harness xmlns="https://harnessxml.com/spec/1.0" id="u" specVersion="1.0">
  <nodes>
    <node id="l" type="loop"><loop kind="while" while="${true}"><body ref="w"/></loop></node>
    <node id="w" type="task"/>
  </nodes>
</harness>"""
        self.assertIn("HX-1001", codes(doc))

    def test_literal_credential_is_hx_3501(self):
        doc = """<harness xmlns="https://harnessxml.com/spec/1.0" id="l" specVersion="1.0">
  <resources>
    <resource id="m" type="model">
      <property name="apiKey" value="sk-ant-api03-AAAAAAAAAAAAAAAAAAAAAAAA"/>
    </resource>
  </resources>
  <nodes><node id="a" type="inference"><resourceRef ref="m"/></node></nodes>
</harness>"""
        self.assertIn("HX-3501", codes(doc))

    def test_credential_reference_is_fine(self):
        doc = """<harness xmlns="https://harnessxml.com/spec/1.0" id="ok" specVersion="1.0">
  <resources>
    <resource id="m" type="model"><credential ref="ANTHROPIC_API_KEY" store="vault"/></resource>
  </resources>
  <nodes><node id="a" type="inference"><resourceRef ref="m"/></node></nodes>
</harness>"""
        self.assertNotIn("HX-3501", codes(doc))

    def test_data_edge_needs_both_ports(self):
        doc = """<harness xmlns="https://harnessxml.com/spec/1.0" id="dp" specVersion="1.0">
  <nodes>
    <node id="a" type="source"><outputs><output name="x"/></outputs></node>
    <node id="b" type="task"><inputs><input name="y"/></inputs></node>
  </nodes>
  <edges><edge from="a" to="b" type="data"/></edges>
</harness>"""
        self.assertIn("HX-2301", codes(doc))


class TestRealExamples(unittest.TestCase):
    def test_every_shipped_example_validates(self):
        examples = sorted((REPO / "examples").rglob("*.hxml"))
        self.assertGreaterEqual(len(examples), 5, "expected the shipped examples")
        for f in examples:
            with self.subTest(example=f.name):
                harnessxml.load(f)  # raises if invalid

    def test_every_invalid_fixture_is_rejected_with_its_code(self):
        d = REPO / "conformance" / "invalid"
        for hx in sorted(d.glob("*.hxml")):
            expected = hx.with_suffix(".expected")
            if not expected.exists():
                continue
            want = expected.read_text().strip()
            with self.subTest(fixture=hx.name):
                got = codes(hx.read_text())
                self.assertTrue(got, f"{hx.name} should have been rejected")
                self.assertIn(want, got, f"{hx.name}: expected {want}, got {got}")


class TestBuilder(unittest.TestCase):
    def test_builds_a_valid_document(self):
        b = Builder("built", entry="start", name="Built by the SDK")
        b.metadata(title="Built by the SDK", author="VisML")
        b.resource("m", "model", provider="anthropic",
                   properties={"model": "claude-opus-5"},
                   credential="ANTHROPIC_API_KEY", credential_store="vault")
        b.node("start", "source").output("doc", "binary")
        b.node("classify", "inference").resource_ref("m", role="model") \
            .input("doc", "binary").output("confidence", "number") \
            .retry(4, retry_on=["rate_limit", "transient"]).timeout("PT3M", "retry")
        b.decision("route").case("${classify.confidence >= 0.9}", "auto").otherwise("review")
        b.node("auto", "task", impl="file.auto")
        b.node("review", "human", impl="review.request", idempotent=False)
        b.data("start", "doc", "classify", "doc")
        b.control("classify", "route")

        xml = b.to_xml()  # validates; raises if invalid
        self.assertIn("<harness", xml)
        h = harnessxml.loads(xml)
        self.assertEqual(len(h.nodes), 5)
        self.assertEqual(h.node("review").idempotent, False)
        self.assertEqual(h.node("classify").retry.max_attempts, 4)

    def test_builder_refuses_retry_on_non_idempotent(self):
        # Better to fail where the mistake is made than to emit a document a
        # validator will reject later.
        b = Builder("x")
        n = b.node("pay", "task", idempotent=False)
        with self.assertRaises(ValueError) as cm:
            n.retry(3)
        self.assertIn("HX-3301", str(cm.exception))

    def test_builder_refuses_an_unbounded_loop(self):
        b = Builder("x")
        n = b.node("l", "loop")
        with self.assertRaises(TypeError):
            n.loop("forEach", "body")  # max_iterations is a required argument

    def test_builder_refuses_a_data_edge_without_ports(self):
        b = Builder("x")
        with self.assertRaises(ValueError):
            b.edge("a", "b", "data")

    def test_to_xml_raises_on_an_invalid_document(self):
        b = Builder("bad")
        b.node("a", "task")
        b.control("a", "ghost")           # dangling
        with self.assertRaises(HarnessXMLError) as cm:
            b.to_xml()
        self.assertIn("HX-2001", str(cm.exception))

    def test_round_trip_through_the_parser(self):
        b = Builder("rt", entry="a")
        b.node("a", "task", impl="x").config(threshold="0.9")
        b.node("b", "task")
        b.control("a", "b")
        h = b.build()
        self.assertEqual(h.entry, "a")
        self.assertEqual(h.node("a").config, [("threshold", "0.9")])


if __name__ == "__main__":
    unittest.main()
