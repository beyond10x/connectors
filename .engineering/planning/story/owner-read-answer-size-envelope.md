---
format: aep.planning-md/3
id: story:owner-read-answer-size-envelope
kind: story
status: implemented
title: A read result the child admits fits the owner's answer
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:owner-read-answers-json-value
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: cited
  path: crates/connectors-host/src/local/owner/supervisor.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/supervisor/adversary_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
- confidence: cited
  path: crates/connectors-host/src/local/runtime.rs
- confidence: cited
  path: crates/connectors-host/src/local/runtime/channel.rs
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:58:15Z", actor: "human:timo", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:58:16Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "active", to: "implemented", at: "2026-11-10T06:20:12Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Defect

Found 2026-10-07 in wave 20261007a (handed back by the owner-read unit, pre-existing). The
adapter child admits a read result up to `RESULT_LIMIT` (8 MiB). The owner wraps it in its
answer with about 150 bytes of envelope (adapter, operation, revision), and its channel write
refuses an answer over the same limit (`crates/connectors-host/src/local/runtime/channel.rs`,
`write` returns Capacity). A result within about 150 bytes of the limit is admitted by the
child and refused at the owner. Before the JSON-value answer the text form inflated the result
further through escaping, so the gap is older than that change.

## Acceptance

- A read result the child admits reaches the CLI: the owner's read answer is bounded at
  `RESULT_LIMIT` plus its envelope, and no other answer's bound moves.
- A test at the exact limit, and one byte over it, pins both sides.

## Scope

Landed on, from `git diff 39078c638...impl/owner-read-answer-size-envelope` (merged into wave/zero-issues at bdbff2a9f); the earlier scoper lines marked inferred are replaced by this list. Generated reference pages are left out.

- `crates/connectors-host/src/local/owner/supervisor/adversary_tests.rs`
- `crates/connectors-host/src/local/owner/supervisor.rs`
- `crates/connectors-host/src/local/owner/transport.rs`
- `crates/connectors-host/src/local/runtime/channel.rs`
- `crates/connectors-host/src/local/runtime.rs`
