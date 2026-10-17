---
format: aep.planning-md/3
id: story:websearch-duckduckgo-abstracts
kind: story
status: draft
title: Keyless abstracts from the DuckDuckGo Instant Answer API behind websearch.search
relations:
- depends_on: story:catalog-github-anonymous-reads
- depends_on: story:catalog-zendesk-ticket-writes
revision: 1
---
## Outcome

A consumer gets keyless abstracts through the Connectors CLI: `websearch.search`
(contracts/datasources/websearch/v1alpha1) is served by a DuckDuckGo adapter that calls the
Instant Answer API only (`https://api.duckduckgo.com/?q=…&format=json`). The descriptor declares it
in abstracts mode: it answers with abstracts, labelled as such, never with web search results.
There is no results mode.

## Spec first

The adapter, its abstracts mode and its bounds are declared in this repository's ESS specification
and the adapter specification kind before any implementation, beside the Tavily adapter. A keyless
provider runs through a connection like every other; it rides on
`architecture-decision-record:credential-less-public-reads` and its implementation rather than a
second exception to the host's credential rule.

## Acceptance

- One GET to `api.duckduckgo.com/` with `q`, `format=json` (and `no_html=1`, `skip_disambig=1`)
  per call; the answer's `Abstract`, `AbstractText`, `AbstractURL`, `AbstractSource`, `Heading`
  and `RelatedTopics` map to entries marked `kind: abstract`, byte-identical against a recorded
  fixture. No result field is filled.
- The adapter fetches no path DuckDuckGo's robots.txt disallows: a test holds the request target
  to the Instant Answer endpoint and refuses `/html` and `/lite` by construction.
- A rate-limit answer (`202` with a `Ratelimit` body, as reported in
  github.com/deedy5/duckduckgo_search/issues/290) is a typed `rate_limited` failure, sent once and
  never retried in a loop.
- An empty abstract is an empty answer, not a failure.
- Bounds, each refused or truncated by name and tested: entry count, request timeout and response
  bytes (a body over the bound is refused, never parsed partially).
- No query text in logs, audit records, errors or metrics: a test searches a sentinel string and
  finds it in no line, record or error the run writes.
- docs: a guide with the config block, the commands for a local install, abstracts mode and the
  limits.
