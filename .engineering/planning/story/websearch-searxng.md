---
format: aep.planning-md/3
id: story:websearch-searxng
kind: story
status: draft
title: Keyless web search through a self-hosted SearXNG adapter behind websearch.search
relations:
- depends_on: story:catalog-github-anonymous-reads
- depends_on: story:catalog-zendesk-ticket-writes
- depends_on: story:websearch-duckduckgo-abstracts
revision: 3
---
## Outcome

A consumer runs keyless web search through the Connectors CLI: `websearch.search`
(contracts/datasources/websearch/v1alpha1) is served by a SearXNG adapter against a configured,
self-hosted SearXNG base URL through its JSON API (`/search?format=json`), with no API key. It
follows story:websearch-duckduckgo-abstracts, which serves abstracts behind the same interface.

## Spec first

The SearXNG provider and its configuration (base URL, bounds) are declared in this repository's
ESS specification and the adapter specification kind before any implementation, in the shape the
Tavily adapter already has. A keyless provider runs through a connection like every other; if the
host's credential rule refuses a profile with no credential, this story depends on
`architecture-decision-record:credential-less-public-reads` and its implementation, and says so
rather than inventing a second exception.

## Acceptance

- `websearch.search` on a SearXNG connection sends one GET to `<base>/search` with `q`,
  `format=json` and the requested page, and maps each result to the interface's result shape
  (url, title, snippet, engine), byte-identical against a recorded fixture.
- Bounds, each refused or truncated by name and tested: result count (at most the interface's
  maximum, default 10), request timeout, and response bytes (a body over the bound is refused,
  never parsed partially).
- No query text in logs, audit records, errors or metrics: a test searches a sentinel string and
  finds it in no log line, record or error the run writes.
- A SearXNG instance with JSON output disabled (403) is a refusal naming that setting.
- docs: a guide with the config block, the commands for a local install and the limits.
