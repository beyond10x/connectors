---
format: aep.planning-md/3
id: approval-record:wave-d240-output-shaping
kind: approval-record
status: approved
title: 'Wave D240: output shaping (fields, each, walk) in invoke and MCP, release 0.63.0'
summary: Four units, S1 and S3 in parallel, then S4, then S2; one PR, release 0.63.0
tags:
- d240
relations:
- decides: story:invoke-shapes-its-result
- decides: story:catalog-operation-declares-its-walk
- decides: story:invoke-walks-declared-pages
- decides: story:mcp-tool-call-shapes-its-result
- decides: story:definition-lowerer-carries-walk
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-10-30T11:46:44Z", actor: "human:timo", revision: 2, executor: "agent:conn-d240", correlation: "wave-d240-output-shaping"}
---
## Wave D240: output shaping

Four units on the integration branch `wave/d240-output-shaping` (tree `conn-d240`), started from
`origin/wave/d265-c4` at `a7f2dc5f2` (0.62.0 in its pull request). When `v0.62.0` is on `main`,
`main` is merged into this branch. One pull request, release 0.63.0 (0.61.0 and 0.62.0 are taken).

| unit | story | depends on |
|---|---|---|
| S1 | `story:invoke-shapes-its-result` | none |
| S3 | `story:catalog-operation-declares-its-walk` | none |
| S4 | `story:invoke-walks-declared-pages` | S1, S3 |
| S2 | `story:mcp-tool-call-shapes-its-result` | S1, S4 |

S1 and S3 run in parallel (CLI shaping in `apps/connectors` and `ess/domains/cli.yaml`; walk
declaration in `adapters/catalog`). S4 and S1 both edit `InvokeInput` and `invoke.rs`, so S4
waits for S1. S2 takes `walk` over MCP, so it waits for S4 as well as S1 (the design ordered S2
after S1 only; the walk argument makes S4 a prerequisite too).

## Decisions (D240, closed by the orchestrator, 2026-10-30)

Quoted from the orchestrator's item: "The decisions are taken (D240, closed by the orchestrator):
a = A (own path list), b = C (`--out <file>` or `--out -`), c = A (with `fields`,
`outputSchema.result` is any JSON and the operation schema moves to `_meta`)."

- a = A: an own path list: dotted paths, `[]`, `alias:`, RFC 6901 escapes; no filters, no new
  dependency.
- b = C: a walk's records go to `--out <file>` or, with `--out -`, to stdout, the summary last.
- c = A: every MCP tool's `outputSchema.result` is any JSON; the operation schema moves to `_meta`.

## Approval

The orchestrator session, 2026-10-30, item D240, under the grant of the workspace instructions
(class C: a wave, its commits, pull request, merge and release). The operator's words relayed
with it, 2026-10-30: "implement with /implementing and use /aep:wave" and "Please do not wait on
me. Move forward, make decisions for me."

## Decided in the wave (coordinator)

- `--walk` without `--out` is refused naming `out`, so a large walk never lands on stdout by
  default (b = C keeps `-` as an explicit choice).
- Carrying `walk` through the definition lowerer needs a field in `connectors-definition/1`, whose
  schema the definitions repository owns: deferred to `story:definition-lowerer-carries-walk`
  (draft). Lowered operations refuse `--walk` as `unsupported` until then.
- Tests are targeted per unit; no local full gate (disk budget). CI's full gate on the pull
  request decides.

## Commits this approval authorises

One or more commits per unit on its `impl/*` branch, the merges into the integration branch, the
merge of `main` after `v0.62.0`, the opening and closing store commits, the 0.63.0 release
commit, one pull request merged through the bot route, the `v0.63.0` tag and its release page.
Nothing else.
