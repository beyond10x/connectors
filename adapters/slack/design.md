# Adapter design: Slack

The provider records are typed in the [adapter-owned ESS model](spec/ess/domains/records.yaml):
Conversation, Message, User, File and Emoji. No relation is declared: every edge is an
`UNMAPPED:` marker there, with a census at the foot of the file.

- **Status:** partly implemented through the catalog provider
  ([docs/catalog-slack.md](../../docs/catalog-slack.md)): `conversations.list`,
  `conversations.history`, `conversations.replies` and `users.list` are selected from the pinned
  Swagger 2.0 document in `upstream/`. `auth.test`, `team.info`, `emoji.list` and
  `search.messages` declare a required `token` parameter the catalog engine cannot withhold;
  the guide's *Methods that cannot be selected yet* names the engine change. Writes and files
  are not started.
- **Serves:** parity units U02, U08 and U13 of
  [the parity page](../../docs/fluxplane-plugin-parity.md#gap-units) (`:777`, `:783`, `:788`);
  stories `story:catalog-slack-reads`, `story:parity-slack-discovery-reads`,
  `story:parity-slack-message-writes` and `story:parity-slack-files`.
- **Sources:** the parity page § slack (`:199-239`), the planning stories above, and the
  fluxplane-plugin skill reference for Slack (plugin 0.23.0, read 2026-10-07, not in this
  repository; it states operation inputs, never outputs). No Slack API document was read.

## 1. Route

**Recommended: the catalog provider `slack` over a pinned upstream document**, the route
`story:catalog-slack-reads` already plans (`:32`): `slackapi/slack-api-specs` at
`bc08db49625630e3585bf2f1322128ea04f2a7f3`, Swagger 2.0, projected exactly to OpenAPI 3
(`story:catalog-swagger2-projection:32`). A native adapter is the fallback the parity page
names (`:777`).

| Question | What is known | What it decides |
|---|---|---|
| Can the catalog ingest Swagger 2.0? | No. Wave 20261006d was refused with `source declares no openapi version` (`story:catalog-swagger2-projection:23`). | That story lands before any unit here. |
| Does the pinned document carry every unit method? | Required only for `conversations.list`, `.history` and `.replies` (`story:catalog-swagger2-projection:32`). Not checked for the other 14 methods in § 2: the document is not in this tree. | A missing method is a `decision-blocker` (`story:catalog-slack-reads:32`); enough of them select the native route. |
| Is an archived document acceptable? | The parity page asks for "a current Slack Web API document" (`:777`); the planned pin comes from an archived repository (`story:catalog-slack-reads:32`). Not checked. | Owner decision. |
| Binary download and multi-step upload (U13) | A catalog engine change (`:788`). | Whether U13 stays on the catalog route. |
| Bot and user tokens on one provider | A catalog connection holds one `AuthConfig` with one `profile` (`adapters/catalog/src/local.rs`, `struct AuthConfig`), and nothing in the local configuration ties it to the bundle's recorded `auth_profile` (checked 2026-10-09). | Search is a second connection under `slack.user` with its own operations file; no host change. Blocked first by the required `token` (the guide's *Methods that cannot be selected yet*). |

## 2. Operations

Calls are since 2026-09-09 (parity page `:210-225`). The methods are the parity page's.

| Unit | fluxplane operation | Calls | Slack Web API method | Effect | Record |
|---|---|---:|---|---|---|
| U02 | `slack.thread` | 225 | `conversations.replies` | read | Message |
| U02 | `slack.message.list` | 92 | `conversations.history` | read | Message |
| U02 | `slack.search` | 73 | `search.messages` (user token) | read | Message |
| U02 | `slack.user.list` | 41 | `users.list` | read | User |
| U02 | `slack.channel.list` | 40 | `conversations.list` | read | Conversation |
| U02 | `slack.info` | 15 | `auth.test`, `team.info` | read | Workspace (not declared: no identity in reach) |
| U02 | `slack.test` | 3 | `auth.test` | read | none: `connections revalidate` (`:81-83`) |
| U02 | `slack.emoji.list` | 1 | `emoji.list` | read | Emoji |
| U08 | `slack.message.send` | 132 | `chat.postMessage`, plus `conversations.open` for a direct message | write | Message |
| U08 | `slack.message.edit` | 20 | `chat.update` | write | Message |
| U08 | `slack.message.delete` | 10 | `chat.delete` | write | Message |
| U13 | `slack.file.upload` | 24 | `files.getUploadURLExternal`, `files.completeUploadExternal` | write | File |
| U13 | `slack.file.download` | 11 | fetch of `url_private` | read, binary | File |
| U13 | `slack.file.delete` | 7 | `files.delete` | write | File |
| U13 | `slack.file.info` | 7 | `files.info` | read | File |
| U13 | `slack.file.list` | 3 | `files.list` | read | File |

`story:catalog-slack-reads` selects `conversations.replies`, `.history` and `.list`, so it
delivers the first two rows and the method behind `slack.channel.list`, which
`story:parity-slack-discovery-reads` also lists as its own. The model
declares nouns only; the writes' approval, idempotency and guards are decided with the U08
and U13 selections, not here.

## 3. Authentication

| Proposed profile | Scheme | Used by | Source |
|---|---|---|---|
| `slack.bot_token` | `http_bearer`, static token | every operation except search | `story:catalog-slack-reads:32`; `epic:catalog-knowledge-sources:34-35` (`bearer: true` exists) |
| `slack.user_token` | `http_bearer`, static token | `search.messages` | parity page `:213`, `:777` |

The profile names are proposals. fluxplane keeps one token set: `bot_token` and `user_token`
required, `app_token` optional. Bot, user and app-level tokens are distinct purposes, and an
app-level token is never a per-tenant installation token
(`contracts/auth/profile/v1alpha1/semantics.md:25`); no unit operation needs one, so no app
profile is proposed. Only static tokens: rotating Slack refresh tokens do not fit static entry
(`.engineering/planning/architecture-decision-record/oauth-material-as-static-entry.md:61-62`).
The identity probe is `auth.test` (parity page `:218`, `:224`).

`UNMAPPED:` the scopes each operation needs, and whether any operation other than search needs
the user token.

## 4. Out of scope

- The 14 operations with 0 calls (parity page `:837`): bookmarks, presence, reactions, unreads,
  mentions, channel join and mark-read, `slack.download` and the fluxplane-local index.
- Markdown. fluxplane renders native mrkdwn to Markdown and accepts Markdown on send. The
  catalog returns the provider's body byte-identical (`story:catalog-slack-reads:54`); a
  conversion is presentation, as Atlassian Document Format is for Jira (parity page `:75`).
- The feed binding (`story:slack-feed-binding`), OAuth acquisition, rotating refresh, events,
  and request signing (`http_signing` is reserved and refused,
  `contracts/auth/profile/v1alpha1/semantics.md:74`).

**Open for the owner:** direct-message reach. `slack.message.send` to a user opens a direct
message (parity page `:211`), while the predecessor dropped `im:*` and `mpim:*` scopes and
withheld the parameter that reaches a direct message (`CHANGELOG.md:1606-1607`).
