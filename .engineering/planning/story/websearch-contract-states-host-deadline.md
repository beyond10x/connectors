---
format: aep.planning-md/3
id: story:websearch-contract-states-host-deadline
kind: story
status: implemented
title: The websearch contract states the deadline a call actually has
summary: contract promises 160 s / 150 s; the CLI cuts every call at 30 s
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: cited
  path: adapters/tavily/contracts/websearch/v1alpha1/semantics.md
- confidence: cited
  path: adapters/tavily/generated/tool-contract.yaml
- confidence: cited
  path: adapters/tavily/spec/ess/domains/requests.yaml
- confidence: cited
  path: adapters/tavily/spec/tool-profiles.json
- confidence: cited
  path: adapters/tavily/src/lib.rs
- confidence: cited
  path: adapters/tavily/src/local.rs
- confidence: cited
  path: adapters/tavily/tests/adversary_deadline.rs
- confidence: cited
  path: adapters/tavily/tests/protocol.rs
- confidence: cited
  path: website/docs/connectors/tavily.md
revision: 15
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:58:46Z", actor: "human:operator", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:58:46Z", actor: "human:operator", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "active", to: "implemented", at: "2026-11-10T06:20:12Z", actor: "human:operator", revision: 15, decided_on: {"recorded":{"test_result":1,"review_outcome":4,"verification":1}}, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Outcome

The websearch contract and the Tavily pages state the deadline a call actually has, and the Tavily
crawl budget sent upstream fits inside it.

## Why now

Post-0.54.0 review run 3 (lane C5 F1, verified 2026-10-19): the contract says "The transport waits at
most 160 s; Tavily bounds a crawl at 150 s"
(`adapters/tavily/contracts/websearch/v1alpha1/semantics.md:36`, `:66-67`, rendered at
`website/docs/reference/adapters/tavily/contracts/websearch.md:51`, `:81-82`). The shipped CLI goes
through the local owner and passes a deadline of now + 30 000 ms
(`apps/connectors/src/local/session.rs:399`); the service path cuts at 20 s
(`contracts/service/v1alpha1/semantics.md:38-39`). A crawl Tavily would finish in 31-150 s is answered
`timeout`, and the crawl may keep running upstream (inferred, not observed).
`website/docs/connectors/tavily.md` states no cap.

## Acceptance

- `semantics.md:36` and `:66-67` state the effective bound per path (CLI 30 s, service 20 s); the
  rendered page is regenerated.
- `adapters/tavily/src/lib.rs:25` (`CRAWL_TIMEOUT_S = 150`) sends a crawl `timeout` below the caller's
  remaining deadline, with a test that pins the value sent for a 30 s deadline.
- `website/docs/connectors/tavily.md` Limits names the bound.

## Scope

Landed on, from `git diff a83a03b85...impl/websearch-contract-states-host-deadline` (merged into wave/zero-issues at b0c9e8c0d); the earlier scoper lines marked inferred are replaced by this list. Generated reference pages are left out.

- `adapters/tavily/contracts/websearch/v1alpha1/semantics.md`
- `adapters/tavily/generated/tool-contract.yaml`
- `adapters/tavily/spec/ess/domains/requests.yaml`
- `adapters/tavily/spec/tool-profiles.json`
- `adapters/tavily/src/lib.rs`
- `adapters/tavily/src/local.rs`
- `adapters/tavily/tests/adversary_deadline.rs`
- `adapters/tavily/tests/protocol.rs`
- `website/docs/connectors/tavily.md`
