---
format: aep.planning-md/3
id: story:parity-confluence-cql-search-v1-held
kind: story
status: draft
title: Confluence CQL search from the REST v1 source, held on vendoring
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- supersedes: story:parity-confluence-cql-search
revision: 1
---
## Outcome

Confluence CQL search (`confluence.page.search`, 8 calls) through a second catalog provider,
`confluence-v1`, built from Atlassian's Confluence Cloud REST v1 OpenAPI document. Held until it is
decided whether that document may be vendored into this public repository.

## What exists

- Local branch `held/confluence-v1-cql-search`, commit `ebbd3596e`, archived with `worktree
  archive` as `conn-confluence-v1`; never pushed.
- Source: https://developer.atlassian.com/cloud/confluence/swagger.v3.json, upstream sha256
  `6c66a606fa7535268512f07f599fe1e3f9de2ba0b1da7eb6ead405509875577e`; no `license`,
  `termsOfService` https://atlassian.com/terms/.
- The document is pinned redacted under a new rule `upstream-redaction/3`
  (`crates/connectors-build/src/upstream_redaction.rs` on that branch): the password of a curl
  user argument becomes `$EXAMPLE_REDACTED_TOKEN`, because the vendor text holds a `curl -u`
  example credential that the secret scanner refuses.
- Selection `search.content` to `searchByCQL`, `cql` required, `limit` 1..=100 (the document
  gives no maximum), on the shared `atlassian.basic` profile; `expand` is sent as one value.
- Tests: `adapters/catalog/tests/confluence_cql_search.rs` (11 cases), redaction rule cases;
  green on the package gates of connectors-build and connectors-catalog-provider.

## Open

- Whether the v1 document may be vendored. The cleared
  `decision-blocker:confluence-openapi-redistribution` answer ("2 documents: jira + confluence -
  but they can share the auth profile.") does not say whether it covers this second Confluence
  document.

## Acceptance

- `confluence.page.search` answers through `connectors operations invoke` on a saved
  `confluence-v1` connection against a recorded fixture, and the parity row moves to covered.
