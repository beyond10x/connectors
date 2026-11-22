---
format: aep.planning-md/3
id: story:slack-four-profile-definition-generated
kind: story
status: draft
title: One generated Slack definition offers every method under user, bot, app-level and configuration profiles
relations:
- decomposes: epic:slack-complete-web-api
- depends_on: story:slack-sdk-upstream-import
- depends_on: story:slack-method-token-scope-table
- depends_on: story:definitions-default-write-guard
revision: 1
---
## Outcome

One generated Slack definition offers every Web API method under each profile whose token type
may call it: `slack.user` (xoxp), `slack.bot` (xoxb), `slack.app` (xapp, HTTP methods only) and
`slack.config` (xoxe). A method callable by two token types appears under both. No profile is
narrowed by a business rule: "no writes on a user token" is not in the definition.

## Change

- A generator joins `story:slack-sdk-upstream-import`'s projection with
  `story:slack-method-token-scope-table`'s table and writes the definition(s) in
  `connectors-definitions`; writes take `story:definitions-default-write-guard`'s default.
- The hand-written operations stay as amendments over the generated base and win over it:
  `file.download` (host and path limits), `file.upload` (three requests), the per-operation
  refusals and bounds of today's 18 bot and 5 user operations.
- Each operation's description names the scopes it needs per token type, from the table.

## Acceptance

- Every method of the table appears under each profile its row allows, and under no other; a
  test compares the counts per profile with the table.
- Today's 23 operations keep their ids and their behaviour (their existing tests pass unchanged).
- The bundle and docs generation stays inside the test budget (120 s step, 60 s per test):
  generation runs once, its output is committed with a digest.

## Scope

connectors-definitions: `connectors/slack*.yaml`. connectors: the generator in
`crates/connectors-build`, `adapters/catalog/providers/slack/`, generated docs. Inferred.
