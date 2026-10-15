---
format: aep.planning-md/3
id: story:confluence-v1-authored-contract
kind: story
status: draft
title: Confluence REST v1 reads from a contract this repository writes
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- supersedes: story:parity-confluence-cql-search-v1-held
revision: 1
---
## Outcome

Confluence Cloud REST v1 reads are served from a source document this repository writes
itself, from Atlassian's public REST v1 reference: spaces, content and CQL search first. A token
that carries classic scopes reads Confluence. Today it connects and then reads nothing: it
answers 200 on `/wiki/rest/api/{space,content,search}` and 401 "Unauthorized; scope does not
match" on `/wiki/api/v2/{spaces,pages}`.

## Rules

- No Atlassian v1 document is vendored, copied or fetched at build time. The v1 OpenAPI
  document declares no licence. Each operation in the authored document names the reference
  page it was written from, as a URL, and is written in this repository's own words and
  structure: paths, parameters and the response fields the shipped reads rely on.
- If an operation cannot be written without copying the document, that operation is left out
  and named in the story's evidence.
- The branch `held/confluence-v1-cql-search` vendors the document and stays held. Its tests and
  selection ids may guide this work; its source and its redaction rule are not reused.

## Scope

- An authored source document for the Confluence REST v1 reads (for example
  `adapters/atlassian/contracts/confluence-rest-v1/`), compiled by the existing catalog
  pipeline into a `confluence-v1` bundle, with the drift test and pinned operation-id checks
  every bundle has.
- A selection set of reads: spaces list and get; content list (by space, by type, updated
  since through CQL), get with the storage body, child comments; CQL search over content. Each
  is `effect: read`.
- Operation ids match the v2 set where the meaning matches (`pages.changed`, `space.pages`,
  `page.get`, `page.comments`), so a caller's id is the same whichever API serves it. v1-only
  reads get their own ids (for example `content.search`, `spaces.list`, `space.get`).
- The ESS model of the Atlassian documents contract and the catalog guide
  (`docs/catalog-confluence.md`) say which reads exist under v1.

## Acceptance

- Each v1 read answers through `connectors operations invoke` against a recorded fixture of
  the v1 response shape, with paging (`_links.next`, `start`/`limit`) followed to its end.
- `confluence.page.search` (CQL) answers through the same route, and the parity row moves to
  covered.
- The bundle drift test reproduces the committed `confluence-v1` bundle byte for byte from the
  authored document.
- No file in the change is a copy of an Atlassian document. Each operation in the authored
  document names its reference URL.
