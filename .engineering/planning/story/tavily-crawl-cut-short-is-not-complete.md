---
format: aep.planning-md/3
id: story:tavily-crawl-cut-short-is-not-complete
kind: story
status: active
title: A Tavily crawl cut short by its timeout is not reported complete
tags:
- from-finding
relations:
- informed_by: review-result:wave-a-websearch-deadline-adversary-1
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T08:52:34Z", actor: "human:timo", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-10T08:52:35Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Defect

Found by the adversary of `story:websearch-contract-states-host-deadline`
(`review-result:wave-a-websearch-deadline-adversary-1`), pre-existing: `adapters/tavily/src/lib.rs:331-336`
marks a crawl `complete: true` whenever it returned fewer pages than `limit`. A crawl Tavily ends at
its own `timeout` with fewer pages and HTTP 200 is then reported complete, against the websearch
family rule (`contracts/datasources/websearch/v1alpha1/semantics.md:68`). What Tavily answers when it
cuts a crawl short has not been observed.

## Acceptance

- Tavily's answer to a crawl cut short by its `timeout` is observed (recorded fixture or documented
  upstream behaviour), and a crawl ended that way is not reported `complete: true`; a protocol test
  pins it.

## Wave status

Status on 2026-11-10: partly delivered in the single defect wave (merge 5d76bbad9, commit
6dc27cb7e).

- **Delivered.** A crawl answer whose `response_time` is at or past the `timeout` the adapter sent
  is now `complete: false`, with `provider_partial`. The test `a_crawl_that_ran_to_its_timeout_is_partial_and_incomplete`
  was red before the change; `adapters/tavily/tests/protocol.rs` now passes 12. The adapter's
  `semantics.md` states the rule.
- **Still open: acceptance line 1, "observed".** Two live crawls, each with `timeout` 10 and
  `limit` 500, ended before their timeout:
  - `en.wikipedia.org`: 0 pages, `response_time` 8.0;
  - `docs.python.org/3/`: 500 pages, `response_time` 6.25.

  Tavily's API reference does not say what a crawl that reaches its timeout returns. Until a
  cut-short answer is observed, the rule rests only on the documented meaning of `timeout`. A
  crawl that Tavily ends just before the timeout would not be caught.
