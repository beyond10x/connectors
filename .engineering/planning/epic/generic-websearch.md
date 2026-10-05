---
format: aep.planning-md/3
id: epic:generic-websearch
kind: epic
status: active
title: Web search results, pages and crawls in one shape, whatever provider answers
summary: A shared datasource.websearch family with Tavily as its first binding.
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T02:15:43Z", actor: "agent:claude", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-05T02:15:43Z", actor: "agent:claude", revision: 3}
---
## Outcome

A provider-independent websearch family: a caller asks for web search results, a set of pages, or a
bounded crawl from one URL, and gets websites with a title, a description and their content in one
shape whatever provider answers. Tavily is the first binding. Requested by the operator on
2026-10-05 for a consumer that feeds web content into knowledge stores on a schedule.

## Findings (2026-10-05)

| Fact | Source |
|---|---|
| Shared families live under `contracts/`; adapters bind them under `adapters/<id>/contracts/<family>/`, as Loki, Kubernetes and Docker bind `datasource.logs/v1alpha1` | `contracts/README.md`, `contracts/datasources/logs/v1alpha1/semantics.md` |
| An HTTP adapter reaches its provider through the host's `ScopedHttp`, which implements `AuthenticatedHttp` with `get` and `get_prefix` only | `crates/connectors-host/src/http.rs:51,511`, `crates/connectors-sdk/src/lib.rs:71-87` |
| A write goes through `AuthenticatedWrite` and needs an approval | `crates/connectors-sdk/src/lib.rs:89-118` |
| The catalog engine admits `effect: read` only for GET | `adapters/catalog/src/lib.rs:313-319` |
| Tavily serves `POST /search`, `/extract`, `/crawl`, `/map` with `Authorization: Bearer`, and publishes OpenAPI at `https://docs.tavily.com/documentation/api-reference/openapi.json` (HTTP 200, 156,727 bytes on 2026-10-05) | Tavily documentation |

Every Tavily operation reads and changes nothing at the provider, yet each is a POST. So the family
needs a read that is sent as a POST, without the approval a write needs.

## Decomposition

1. `story:websearch-contract`: the shared `datasource.websearch/v1alpha1` semantics and model.
2. `story:read-post-capability`: `AuthenticatedHttp::post_json`, reachable only by an operation its
   adapter declares a read sent as a POST.
3. `story:tavily-websearch-adapter`: the native `tavily` adapter binding the family.

Out of scope: admitting POST reads in catalog selections (HubSpot search), `map`, Tavily `research`,
and any provider beyond Tavily.
