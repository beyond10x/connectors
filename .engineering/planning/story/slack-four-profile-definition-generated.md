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
scope:
- confidence: inferred
  path: adapters/catalog/providers/slack
- confidence: inferred
  path: adapters/catalog/tests/adversary_credential_pass1.rs
- confidence: inferred
  path: crates/connectors-build
- confidence: inferred
  path: crates/connectors-catalog/src/definition.rs
- confidence: inferred
  path: website/docs/reference
revision: 4
---
## Outcome

One generated Slack definition offers every Web API method under each profile whose token type
may call it: `slack.user` (xoxp), `slack.bot` (xoxb), `slack.app` (xapp, HTTP methods only) and
`slack.config` (xoxe). A method callable by two token types appears under both. Each operation
carries its required scopes per profile as data. No profile is narrowed by a business rule: "no
writes on a user token" is not in the definition.

## Change

- A generator joins `story:slack-sdk-upstream-import`'s projection with
  `story:slack-method-token-scope-table`'s table and writes the definition(s) in
  `connectors-definitions`; writes take `story:definitions-default-write-guard`'s default.
- Required scopes per profile become a field of each operation in the definition format and in
  the lowered operation records (spec first), so admission reads them as data
  (`story:admission-checks-token-scopes`). Descriptions name them too.
- A method whose row has `unknown` token types appears under no profile; the generator writes it
  to a committed list of unplaced methods with the reason.
- The hand-written operations stay as amendments over the generated base and win over it:
  `file.download` (host and path limits), `file.upload` (three requests), the per-operation
  refusals and bounds of today's 18 bot and 5 user operations.
- This story replaces the assertion "no write included" in
  `adapters/catalog/tests/adversary_credential_pass1.rs:290-296`, which fails once `slack.user`
  carries writes: the test then asserts that the user selection equals the table's user rows.

## Acceptance

- Every method of the table with known token types appears under each profile its row allows,
  and under no other; a test compares the counts per profile with the table.
- Required scopes are data: the lowered `conversations.replies` under `slack.user` carries no
  scope field before this story, and after it carries `channels:history`, `groups:history`,
  `im:history` and `mpim:history`, equal to the table's row; a test compares every lowered
  operation's scopes with its table row.
- A table with one `unknown` row (a test fixture) lowers to a definition without that method
  under any profile and with it in the unplaced list.
- Today's 23 operations keep their ids and their behaviour; their existing tests pass unchanged,
  except the assertion at `adversary_credential_pass1.rs:290-296`, replaced as stated above.
- Generation runs once and its output is committed with a digest; the test step stays inside its
  budget (120 s, 60 s per test).

## Scope

connectors-definitions: `connectors/slack*.yaml`. connectors: the generator in
`crates/connectors-build`, the format in `crates/connectors-catalog/src/definition.rs`,
`adapters/catalog/providers/slack/`, `adapters/catalog/tests/adversary_credential_pass1.rs`,
generated docs. Inferred.
