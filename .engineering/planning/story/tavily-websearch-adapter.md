---
format: aep.planning-md/3
id: story:tavily-websearch-adapter
kind: story
status: active
title: Tavily answers the websearch family
relations:
- decomposes: epic:generic-websearch
- depends_on: story:websearch-contract
- depends_on: story:read-post-capability
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T02:15:49Z", actor: "agent:claude", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-05T02:15:49Z", actor: "agent:claude", revision: 3}
---
## Outcome

A native `tavily` adapter (`adapters/tavily/`, a workspace member following `sql` and `kubernetes`)
binds `datasource.websearch/v1alpha1` under the native profile `tavily/2026-10`:

- `upstream/` pins Tavily's OpenAPI document with source URL, retrieval date and SHA-256.
- `spec/adapter.json` and `generated/descriptor.json` declare `websearch.search`, `websearch.fetch`
  and `websearch.crawl`, each a read sent as a POST. `map` is not exposed.
- `contracts/websearch/v1alpha1/semantics.md` maps each family field to a Tavily parameter or result
  field and lists what the profile refuses.
- Profile `tavily.api-key` takes the protected entry `{"api_key": "<key>"}`; the connection is
  checked by one search with `max_results: 1`.

## Acceptance

- Protocol tests against a local fake server: each operation's request (path, method, body) and its
  mapping of the answer into the family's result shape.
- An input the profile does not honour is refused by name with no request sent.
- `content` is bounded; truncation is reported.
- The API key appears in no error, log or debug rendering.
- `connectors connections connect --adapter tavily --profile tavily.api-key --credential-stdin`
  stores a key, and `connections list --adapter tavily` shows the connection.
