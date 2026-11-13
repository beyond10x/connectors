---
format: aep.planning-md/3
id: story:catalog-basecamp
kind: story
status: active
title: Basecamp through the catalog provider
summary: Read projects, message boards, messages, comments and to-dos; writes as a second batch
relations:
- decomposes: epic:business-teams-google-and-hubspot
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-13T10:44:52Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-13T10:44:53Z", actor: "human:operator", revision: 3}
---
# Basecamp through the catalog provider

## Need

A user asked for a Basecamp connector, served to an AI client through the `connectors` MCP server
like the other catalog connectors. Basecamp publishes its API as an OpenAPI 3.1 document,
`openapi.json` in github.com/basecamp/basecamp-sdk (MIT, 300 operations, version 2026-11-05),
with no `servers` and no `securitySchemes`; the base URL is
`https://3.basecampapi.com/{account_id}/` (github.com/basecamp/bc3-api README). Requests need
`Authorization: Bearer`, a `User-Agent` naming the application and a contact, and answer 429
with `Retry-After`; pages follow the `Link` header's `rel="next"`.

Sign-in: the first version takes a Basecamp personal access token
(https://app.basecamp.com/my/access_tokens) through the host's existing bearer profile. OAuth
through Launchpad needs story:launchpad-token-request-parameters and
story:oauth-fixed-loopback-redirect and is not part of this version.

## Acceptance

Batch 1, reads (no write operation is offered):
- A definition `connectors/basecamp.yaml` in connectors-definitions, from the vendored
  `openapi.json` with its SHA-256, exposes: projects list and get, project people, message board
  get, messages list and get, comments list and get, to-do set get, to-do lists list and get,
  to-dos list and get.
- Its bearer profile takes the token only through the host's secret input; no definition,
  argument or example carries a token-shaped value.
- The connector's documentation says in its first lines that a personal access token is
  read-write at Basecamp although this version offers only reads, and where the user creates
  and revokes it.
- The catalog provider lowers it and each read is tested against a fake Basecamp server: the
  account id in the path, the bearer token, the `User-Agent`, `Link` paging to the last page,
  429 with `Retry-After` answered as rate limited, 404 as not found.
- `connectors server --transport stdio` with a Basecamp instance lists these operations as MCP
  tools.

Batch 2, writes, built only after batch 1 is released: create a to-do, a message and a comment,
each behind the catalog's approval for writes, tested against the same fake.

Depends on story:user-agent-names-the-project, story:catalog-path-prefix-from-config and
story:catalog-link-header-next-page.

No live run by an agent: the token of a real account is the operator's to create and enter.
