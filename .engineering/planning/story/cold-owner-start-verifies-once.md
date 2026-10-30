---
format: aep.planning-md/3
id: story:cold-owner-start-verifies-once
kind: story
status: active
title: A cold owner start verifies the store once and answers describe within the caller's deadline
relations:
- informed_by: story:owner-start-outlasts-a-fixed-bound
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T05:28:52Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-30T05:28:52Z", actor: "human:timo", revision: 4}
---
## Problem

0.59.3 lets a read wait for a slow owner start within the read's own deadline
(story:owner-start-outlasts-a-fixed-bound). The cold start is still slow enough to exceed a
consumer's own limit.

On 2026-10-30, a local consumer ran v0.59.3 against a 217 MB metadata store:
- A cold `operations describe` took 95.7 s and exited 0.
- The next read took 26.3 s, and the one after it 1.2 s.
- The consumer limits each call to 60 s.

The 0.59.3 change's author observed that every cold start checks the whole store twice: once
when the CLI admits the call itself, and again when the owner it starts verifies the store before
it answers its greeting. Only that observation backs this; no profile confirms it.

## Outcome

A cold owner start verifies the store once. `operations describe`, and every read that starts an
owner, answers within the caller's deadline on a store of that size.

## Acceptance

- One cold start, measured on a store of about 200 MB, verifies the store exactly once. A test
  counts the verification passes.
- On that store, a cold `operations describe` and a cold read each answer within 60 s on the CI
  runner class, and the next read takes no longer than a warm one.
- A store that fails verification still refuses before any provider call.
