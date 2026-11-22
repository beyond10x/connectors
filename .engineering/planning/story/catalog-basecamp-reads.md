---
format: aep.planning-md/3
id: story:catalog-basecamp-reads
kind: story
status: draft
title: A Basecamp connector reads projects, messages, comments, to-dos and people
summary: Takes over catalog-basecamp and catalog-link-header-next-page; WIP on wip/basecamp-reads and wip/basecamp-defs
relations:
- serves: vision:independent-contract-adapters
- supersedes: story:catalog-basecamp
- supersedes: story:catalog-link-header-next-page
revision: 1
---
# A Basecamp connector reads projects, messages, comments, to-dos and people

## Need

A Basecamp account can be read through Connectors: projects, message boards, messages, comments,
to-dos and people, through the CLI and the MCP server the `connectors` binary serves. Writes (a
to-do, a message, a comment) are a later story, built only after the reads are released.

This story takes over `story:catalog-basecamp` and `story:catalog-link-header-next-page`, both
archived when the Basecamp work stopped on 2026-11-14.

## Where the work stands

- Merged and released in 0.73.0: the User-Agent naming the project (`story:user-agent-names-the-project`,
  PR 262) and the base path filled from a config variable (`story:catalog-path-prefix-from-config`,
  PR 263).
- Not merged: `link_next` paging from the `Link` header's `rel="next"`, PR 264, branch
  `eng/catalog-link-header-next-page` (closed unmerged; reopen or rebase it).
- Work in progress, gate red: branch `wip/basecamp-reads` in this repository (stacked on PR 264) and
  branch `wip/basecamp-defs` in connectors-definitions (format fields, vendored OpenAPI,
  `connectors/basecamp.yaml`; `task check` passes there).
- The vendored source is the MIT-licensed Basecamp SDK's OpenAPI 3.1 document (300 operations, no
  `servers` or `securitySchemes`, paths under `/{accountId}/`).

## What is left

1. `story:definition-import-accepts-endpoint-path` (also `link_next`, once PR 264 merges): the
   importer refuses both fields, so on the WIP branch the catalog (49 tests), provider (1) and
   connectors-build (1) suites fail.
2. Map the Basecamp declaration in
   `apps/connectors/tests/setup_migrate.rs` `every_shipped_declaration_maps_its_native_provider_and_no_other_does`.
3. Merge PR 264, then the reads; release Connectors, then the connectors-definitions minor that
   carries the format fields.

## Acceptance

- Reads tested against a fake server only; the gate is green.
- The first version authenticates with a Basecamp personal access token through the existing
  bearer profile. The docs' first lines say the token is read-write at the vendor although the
  connector offers only reads, and where it is created and revoked
  (`https://app.basecamp.com/my/access_tokens`).
- The token enters only through the engine's secret input, never in a definition, an argument or
  an environment example.
- Paging follows `rel="next"` only on the instance's own scheme, host and port; another host is
  refused. A 429 is answered with its `Retry-After`.
- The docs show the MCP command and the tool list it prints.

## Out of scope

- Launchpad OAuth: `story:launchpad-token-request-parameters` and
  `story:oauth-fixed-loopback-redirect` (drafts).
- No account, OAuth application, client id or token is created by an agent; a live check is the
  operator's, with their own token.
