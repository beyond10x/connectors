---
format: aep.planning-md/3
id: story:tavily-crawl-cut-short-is-not-complete
kind: story
status: draft
title: A Tavily crawl cut short by its timeout is not reported complete
tags:
- from-finding
relations:
- informed_by: review-result:wave-a-websearch-deadline-adversary-1
revision: 1
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
