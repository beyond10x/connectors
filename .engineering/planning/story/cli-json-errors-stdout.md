---
format: aep.planning-md/3
id: story:cli-json-errors-stdout
kind: story
status: archived
title: Decide where --output json errors are written
revision: 3
transitions:
- {from: "draft", to: "archived", at: "2026-09-29T08:40:28Z", actor: "human:timo", revision: 3}
---
## Problem
With `--output json`, errors go to stderr and stdout is empty; a consumer reading stdout sees nothing. Check the CLI contract (contracts/cli) and either document stderr or write the JSON error to stdout.

## Acceptance
- The contract names the stream, and a CLI test holds it.

## Finding

Already satisfied, found by `story-scoper` on 2026-09-29 and confirmed by reading the cited lines:
the CLI contract names the stream — `contracts/cli/v1alpha1/semantics.md:156-158`, "Failure leaves
stdout empty and emits one `{"ok":false,...}` envelope plus newline to stderr" — and
`crates/connectors-conformance/tests/cli_surface.rs:80-85` (`refusal()`, 11 call sites) asserts
stdout empty and the envelope on stderr. Nothing was built. A consumer reads JSON errors from
stderr; the knowledge-ingest consumer has been told so.
