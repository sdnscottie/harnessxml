# HarnessXML — Python SDK

Python SDK for the [HarnessXML open specification](https://harnessxml.com/).

**Standard library only.** A specification that promises its released versions
stay reachable forever should not need a dependency tree to read them.

**Conformance: Core** — parse and validate. This SDK does not execute
workflows; the reference executor does.

## Install

```bash
pip install harnessxml          # once published
# or, from a checkout:
PYTHONPATH=sdk/python python3 -m harnessxml validate workflow.hxml
```

## Read a document

```python
import harnessxml

h = harnessxml.load("workflow.hxml")        # parses AND validates, or raises
print(h.id, len(h.nodes), "nodes")

for n in h.nodes:
    if not n.idempotent:
        print(n.id, "must never be auto-retried")
```

Inspect findings without raising:

```python
diags = harnessxml.check("workflow.hxml")
for d in diags.sorted():
    print(d.code, d.line, d.message)       # HX-3301 42 node 'pay' is declared…
```

## Build a document

The builder refuses constructs the specification forbids, so the mistake
surfaces where it is made rather than in someone else's build:

```python
from harnessxml import Builder

b = Builder("triage", entry="receive")
b.resource("classifier", "model", provider="anthropic",
           properties={"model": "claude-opus-5"},
           credential="ANTHROPIC_API_KEY", credential_store="vault")

b.node("receive", "source").output("document", "binary")
b.node("classify", "inference") \
    .resource_ref("classifier", role="model") \
    .input("document", "binary") \
    .output("confidence", "number") \
    .retry(4, retry_on=["rate_limit", "transient"]) \
    .timeout("PT3M", "retry")
b.decision("route").case("${classify.confidence >= 0.9}", "auto").otherwise("review")
b.node("auto", "task", impl="file.auto")
b.node("review", "human", impl="review.request", idempotent=False)

b.data("receive", "document", "classify", "document")
b.control("classify", "route")

print(b.to_xml())     # validates first; raises HarnessXMLError if invalid
```

Things it refuses outright:

```python
b.node("pay", "task", idempotent=False).retry(3)   # ValueError: HX-3301
b.node("l", "loop").loop("forEach", "body")        # TypeError: maxIterations required
b.edge("a", "b", "data")                           # ValueError: needs both ports
```

## CLI

```bash
python -m harnessxml validate workflow.hxml
python -m harnessxml graph    workflow.hxml
```

Exit codes follow specification §14.7 — `0` valid, `1` invalid, `2` the tool
itself failed. Separating `1` from `2` matters in CI: "this workflow is wrong"
and "the validator is broken" need different responses.

## Tests

```bash
python3 -m unittest discover -s sdk/python/tests
python3 conformance/validate.py --cmd "sdk/python/harnessxml-validate"
```

Licensed Apache-2.0. Specification text CC BY 4.0.
