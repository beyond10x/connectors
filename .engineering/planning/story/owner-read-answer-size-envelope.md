---
format: aep.planning-md/3
id: story:owner-read-answer-size-envelope
kind: story
status: draft
title: A read result the child admits fits the owner's answer
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:owner-read-answers-json-value
revision: 1
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
