---
format: aep.planning-md/3
id: story:slack-method-token-scope-table
kind: story
status: draft
title: A committed table gives each Slack method its token types, scopes and effect
relations:
- decomposes: epic:slack-complete-web-api
- depends_on: story:slack-sdk-upstream-import
scope:
- confidence: inferred
  path: .github/workflows/slow.yml
- confidence: inferred
  path: adapters/slack/upstream
- confidence: inferred
  path: crates/connectors-build
revision: 3
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
- Ten named rows hold these values, compared by a test against the committed table. The values
  are expected from Slack's method pages as known on 2026-11-23 and are confirmed against the
  pages when the table is first generated; a page that disagrees wins, and the row and this list
  change together with the page cited:

  | method | token types | scopes | effect |
  |---|---|---|---|
  | `conversations.replies` | bot, user | `channels:history`, `groups:history`, `im:history`, `mpim:history` (by conversation type) | read |
  | `chat.postMessage` | bot, user | `chat:write` | write |
  | `search.messages` | user | `search:read` | read |
  | `apps.connections.open` | app-level | `connections:write` | write |
  | `apps.manifest.create` | configuration | none | write |
  | `files.getUploadURLExternal` | bot, user | `files:write` | write |
  | `users.info` | bot, user | `users:read` | read |
  | `reactions.add` | bot, user | `reactions:write` | write |
  | `admin.users.list` | user | `admin.users:read` | read |
  | `tooling.tokens.rotate` | none (a refresh token in the body) | none | write; unverified |

## Scope

connectors: a generator in `crates/connectors-build`, the table under `adapters/slack/upstream/`;
the `slow` workflow. Inferred.
