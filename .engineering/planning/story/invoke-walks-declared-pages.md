---
format: aep.planning-md/3
id: story:invoke-walks-declared-pages
kind: story
status: active
title: invoke --walk follows a declared walk and writes or streams NDJSON with a summary
tags:
- d240
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:invoke-shapes-its-result
- depends_on: story:catalog-operation-declares-its-walk
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T11:46:45Z", actor: "human:timo", revision: 2, executor: "agent:conn-d240", correlation: "wave-d240-output-shaping"}
- {from: "proposed", to: "active", at: "2026-10-30T11:46:45Z", actor: "human:timo", revision: 3, executor: "agent:conn-d240", correlation: "wave-d240-output-shaping"}
---
## Problem

With a walk declared (`story:catalog-operation-declares-its-walk`), an agent still has to run the
loop itself, and a walk of 40 audits is 38,522 B in its context per page.

## Outcome

`connectors invoke` takes `--walk`, `--max-pages <n>` (default 20, at most 10,000) and
`--out <file|->` (decision D240 b = C).

- `connectors invoke zendesk/support ticket.audits --walk --out audits.ndjson --fields
  author_id,created_at,events[].field_name <<<'{"ticket_id":101,"page[size]":100}'` writes one
  record per line and prints
  `{"ok":true,"result":{"pages":1,"records":40,"end":"has_more_false","complete":true,"out":"audits.ndjson"}}`.
- `--out -` streams the NDJSON on stdout, the summary envelope last.
- `--walk` without `--out` is refused as `invalid_input` naming `out`.
- The file is created new and owner-only; an existing file is refused before any call.
- Each page is one admitted read: one audit record and one response bound per page, as today.
- The summary's `end` names the declared condition that ended the walk.
- The walk stops at `--max-pages`, on `rate_limited` or on any refusal: records already written
  stay, `complete` is false, `resume` holds the input for the next call (the cursor), and the
  refusal is in the summary.
- `--fields` shapes each record (`story:invoke-shapes-its-result`).
- `--walk` is refused on a write, and on an operation that declares no walk as `unsupported`
  naming `walk`, both before any call.

## Contract

- `ess/domains/cli.yaml` `InvokeInput` gains `walk`, `max_pages`, `out`; a walk summary result;
  `contracts/cli/v1alpha1/semantics.md` gains "Walking pages". Generated CLI contract and
  reference regenerated.

## Acceptance

Named tests in `apps/connectors/tests/invoke_walks.rs` against the Zendesk loopback fixture (it
already serves two-page walks for comments and audits): two-page walk to a file with
`end`/`complete`, `--out -`, `--max-pages 1` gives `complete:false` and `resume`, an existing
`--out` file refused, `--walk` without `--out` refused, undeclared operation refused as
`unsupported`, write refused, a mid-walk refusal keeps written records.
