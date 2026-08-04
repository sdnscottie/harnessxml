# HarnessXML — Go SDK

Go SDK for the [HarnessXML open specification](https://harnessxml.com/).

**Standard library only** (`encoding/xml`) — `go.mod` has no requires.

**Conformance: Core** — parse and validate. This SDK does not execute
workflows; the reference executor does.

## Install

```bash
go get gitlab.com/visml/harnessxml/sdk/go
go install gitlab.com/visml/harnessxml/sdk/go/cmd/harnessxml@latest
```

## Read a document

```go
package main

import (
    "fmt"
    "log"

    harnessxml "gitlab.com/visml/harnessxml/sdk/go"
)

func main() {
    h, err := harnessxml.Load("workflow.hxml")   // parses AND validates
    if err != nil {
        log.Fatal(err)                            // *ValidationError, with codes
    }
    fmt.Println(h.ID, len(h.Nodes), "nodes")

    for _, n := range h.Nodes {
        if !n.Idempotent {
            fmt.Println(n.ID, "must never be auto-retried")
        }
    }
}
```

Inspect findings without treating invalidity as an error:

```go
h, diags := harnessxml.Check(src)
for _, d := range diags.Sorted() {
    fmt.Println(d.Code, d.Line, d.Message)   // HX-3301 42 node 'pay' is declared…
}
```

## CLI

```bash
harnessxml validate workflow.hxml
harnessxml graph    workflow.hxml
```

Exit codes follow specification §14.7 — `0` valid, `1` invalid, `2` the tool
itself failed. Separating `1` from `2` matters in CI: "this workflow is wrong"
and "the validator is broken" need different responses.

## Tests

```bash
cd sdk/go && go test ./...
go build -o /tmp/hx ./cmd/harnessxml
python3 conformance/validate.py --cmd "/tmp/hx validate"
```

Licensed Apache-2.0. Specification text CC BY 4.0.
