---
format: aep.planning-md/3
id: story:websearch-contract-states-host-deadline
kind: story
status: draft
title: The websearch contract states the deadline a call actually has
summary: contract promises 160 s / 150 s; the CLI cuts every call at 30 s
relations:
- serves: vision:independent-contract-adapters
revision: 1
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
