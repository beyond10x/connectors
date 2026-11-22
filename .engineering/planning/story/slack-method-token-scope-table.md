---
format: aep.planning-md/3
id: story:slack-method-token-scope-table
kind: story
status: draft
title: A committed table gives each Slack method its token types, scopes and effect
relations:
- decomposes: epic:slack-complete-web-api
revision: 1
---
## Outcome

A committed table states, for every Slack Web API method, the token types that may call it
(user, bot, app-level, configuration), the scopes each token type needs, and whether the method
reads or writes. Its source is Slack's method reference pages.

## Change

- A generator reads the method pages once, in the `slow` lane only, and writes the table with
  the digest of what it read and the retrieval date. Tests read the committed table; the test
  step makes no network request.
- A method whose page states no token type or no scope is listed with `unknown`, never guessed.
- Read or write: from the page where it says so; otherwise from the method name's verb with the
  rule named in the file, each such row marked `inferred`.
- The configuration token's rules (`xoxe`, `tooling.tokens.rotate`, how long a token lives, how
  a refresh token is used) are **unverified** on 2026-11-23: the story records what the pages
  say and marks each rule as read from a page or unverified.

## Acceptance

- Every method of `story:slack-sdk-upstream-import`'s projection has a row; a test fails on a
  method without one.
- Ten named rows match their pages: `conversations.replies`, `chat.postMessage`,
  `search.messages` (user only), `apps.connections.open` (app-level only), `apps.manifest.create`
  (configuration only), `files.getUploadURLExternal`, `users.info`, `reactions.add`,
  `admin.users.list`, `tooling.tokens.rotate`.

## Scope

connectors: a generator in `crates/connectors-build`, the table under `adapters/slack/upstream/`;
the `slow` workflow. Inferred.
