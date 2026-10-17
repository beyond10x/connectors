# Slack through the catalog provider

The catalog provider reads Slack conversations — the channels a bot token can
see, a channel's messages and one thread — the workspace's users, the
workspace itself, its custom emoji, the token's identity and the files the token
can see, with their content, from the pinned Slack Web API document, posts
messages, a direct message included, under approval, and, on a
second instance with a user token, reads the conversations, history and
threads that user can read, the channels the bot is not in included, and
searches messages. Nothing here is
Slack-specific code: the Swagger 2.0 document is projected into OpenAPI 3.1.0, the projection is compiled into a
bundle, two reviewed selection sets expose ten reads and four writes for a bot
token and five reads for a user token, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them.
Configuration, connection, approval and invocation work as described there;
this page covers what differs for Slack. The terms connector, instance and connection are those
of the [vocabulary](../README.md#vocabulary).

Slack runs as two pinned instances of the connector `slack`, one per selection set: a bot
instance and a user instance ([Configure the two instances](#configure-the-two-instances)). The
catalog provider ships no connector declaration for Slack, so neither is derived.

## Source and bundle

The pinned source is
[`adapters/slack/upstream/slack_web_openapi_v2_without_examples.json`](../adapters/slack/upstream/README.md),
SHA-256 `8b92da26a3c5b11d20042a9f36d81f1fa6fc9382c5ddc471babb68b91936bc3a`:
`slackapi/slack-api-specs` at commit `bc08db49`, Swagger `2.0`, `info.version`
`1.7.0`, 174 operations. It is projected under the rule table of
`crates/connectors-catalog/src/swagger.rs` (`swagger2-openapi/2`), and the
bundle is compiled from the projection with the derivation recorded:

```sh
cargo run --locked -p connectors-build -- swagger \
  --source adapters/slack/upstream/slack_web_openapi_v2_without_examples.json \
  --out adapters/slack/generated/slack-web.openapi.json
cargo run --locked -p connectors-build -- catalog \
  --provider slack \
  --source adapters/slack/generated/slack-web.openapi.json \
  --derived-from adapters/slack/upstream/slack_web_openapi_v2_without_examples.json \
  --directory adapters/catalog/generated/bundles \
  --auth-profile slack.bot \
  --amendments adapters/slack/upstream/slack-web.amendments.json \
  --replace
```

`--replace` is needed because the committed index already carries `slack`;
without it the second command is refused with "provider is already indexed".
`--amendments` applies
[`slack-web.amendments.json`](../adapters/slack/upstream/slack-web.amendments.json),
bound to the projection's SHA-256: it records `chat_postMessage`,
`conversations_open`, `chat_update` and `chat_delete` with an `application/json`
request body, the one body the engine sends
([cited amendments](local-catalog-provider.md#build-the-bundle)).
The pinned document lists `application/json` in all four operations' `consumes`
beside the form type, but writes their parameters as `formData`, which the
projection carries only under the form media type.
The first command writes `slack-web.openapi.json` and its projection record
`slack-web.openapi.projection.json`: all 174 operations are projected. The
document's host `slack.com` and base path `/api` become the projection's one
server, `https://slack.com/api`, and the bundle records each path below it, so
Swagger's `/conversations.list` is recorded as `/api/conversations.list`. The
bundle carries all 174 operations and names none unsupported; its derivation
records format `swagger/2.0`. `adapters/catalog/tests/bundle_drift.rs`
regenerates the projection and its record from the pinned document and the
bundle from the projection, and refuses any of them, or the index, that a fresh
run would not reproduce byte for byte.

## The shipped selection set

[`adapters/catalog/providers/slack/operations.json`](../adapters/catalog/providers/slack/operations.json)
exposes ten reads for a bot token, each `effect: read`, and four writes,
`chat.postMessage`, `conversations.open`, `chat.update` and `chat.delete`, each
`effect: write`
([Messages](#messages)).
[`adapters/catalog/providers/slack/user-operations.json`](../adapters/catalog/providers/slack/user-operations.json)
exposes `search.messages`, the three conversation reads and `users.list`, and
no write, for a user token
([User token](#reads-and-search-on-a-user-token-connection)). A selection id is the Slack
method name, except `file.download`, which names no method
([Files](#files)). `adapters/catalog/tests/slack.rs` pins both exact id lists and
each id's `operationId` and path in the pinned document, so a renamed or
dropped id, or a source operation that moved, fails the gate. The bundle
refuses at load any `operation_id` the projection lacks.

Each of the four lists returns one page per call. The provider returns `status`, `body` and
`provenance`; `body` is Slack's answer, and the field that decides the end of a
walk is in it.

| id | pinned `operationId` | request | paging parameters | end condition | time filter or deltas |
|---|---|---|---|---|---|
| `conversations.list` | `conversations_list` | `GET /api/conversations.list` | `cursor`, `limit` (1–1000) | empty `response_metadata.next_cursor` | none; list the channels, then page each channel's history |
| `conversations.history` | `conversations_history` | `GET /api/conversations.history` | `cursor`, `limit` | empty `response_metadata.next_cursor` | `oldest`, `latest` (Slack `ts`); deltas with `oldest` set to the newest `ts` already read |
| `conversations.replies` | `conversations_replies` | `GET /api/conversations.replies` | `cursor`, `limit` | empty `response_metadata.next_cursor` | `oldest`, `latest` (Slack `ts`) within one thread |
| `users.list` | `users_list` | `GET /api/users.list` | `cursor`, `limit` | empty `response_metadata.next_cursor` | none; the whole list is read again |

- **Following a page.** Send the first page without `cursor`. For the next
  page, send the previous page's `response_metadata.next_cursor` as `cursor`,
  with the other parameters unchanged. A page whose `next_cursor` is the empty
  string, or that has no `response_metadata`, is the last. `has_more` on the
  message reads says the same thing; the walk reads `next_cursor`.
- **`conversations.list`.** `types` takes a comma-separated list of
  `public_channel`, `private_channel`, `mpim` and `im` (Slack's default is
  `public_channel`); `exclude_archived` set to `true` leaves archived channels
  out. The pinned `limit` description says it "must be an integer no larger
  than 1000", so the selection bounds `limit` to 1–1000 and 0, 1001 or a value
  that is not an integer is refused as `invalid_input` before any request. The
  list has no time filter: a consumer lists the channels, then pages each one's
  history.
- **`conversations.history`** requires `channel`, a conversation id from
  `conversations.list`; an input without it is refused before any request.
  Messages come newest first. `oldest` and `latest` bound the window by message
  `ts`; `inclusive` set to `true` includes messages whose `ts` equals either
  bound. To take deltas, keep the newest `ts` a walk returned and send it as
  `oldest` next time.
- **`conversations.replies`** requires `channel` and `ts`, the thread parent's
  `ts`; the first message of the first page is the parent itself. A message
  with no replies returns only itself. `oldest`, `latest` and `inclusive`
  window the replies as they do the history.
- **`users.list`** pages the workspace's users under `members`, each an
  `objs_user` with `id` (`U…` or `W…`) and `name`. `include_locale` set to
  `true` adds each user's locale. The pinned `limit` description states no
  maximum, only that "providing no `limit` value will result in Slack
  attempting to deliver you the entire result set", so the selection bounds
  nothing; send `limit` and walk the pages. There is no updated-since filter.
  Looking up one user by id or e-mail (`users.info`, `users.lookupByEmail`) is
  not selected: read the list.
- **Timestamps.** The pinned document types `ts`, `oldest` and `latest` as
  `number`, and the bundle records no input type for them, so they take any
  scalar and are sent as given. Send them as strings, such as
  `"1780000100.000100"`: a JSON number is sent as its shortest decimal text, so
  `1780000100.000100` would be sent as `1780000100.0001`.
- **`token`.** Every selected operation in the pinned document also declares
  the credential as a `token` parameter. Where it is optional, the selection
  withholds it; where the document marks it required, the selection names it
  as its `credential` ([below](#credential-parameters)). Either way the token
  travels only in the `Authorization` header, and an input carrying `token`
  (or `query:token`, `header:token`) is refused before any request.

Every other query parameter the projection declares for an operation is
accepted by name; one it does not declare is refused before any request.

The three reads that are not lists each answer in one call:

| id | pinned `operationId` | request | scope | answer |
|---|---|---|---|---|
| `team.info` | `team_info` | `GET /api/team.info` | `team:read` | `team`, an `objs_team` with `id`, `name`, `domain`, `email_domain` and `icon` |
| `emoji.list` | `emoji_list` | `GET /api/emoji.list` | `emoji:read` | the document states only `ok` |
| `auth.test` | `auth_test` | `GET /api/auth.test` | none | `url`, `team`, `user`, `team_id` and `user_id`, and `bot_id` for a bot token |

- **`team.info`** reads the token's own workspace. Its optional `team` names
  another workspace, which Slack answers only when the token sees it through
  an externally shared channel.
- **`emoji.list`** lists the workspace's custom emoji, not the standard set.
  The pinned answer is the document's default success template, so the
  document states no member beyond `ok`; Slack's answer with `"ok": true` is
  returned as it came.
- **`auth.test`** returns the answer of the request connecting already makes
  ([Authentication](#authentication)), as an operation a caller reads:
  together with `team.info` it gives the token's identity and workspace.
  Connecting and `connections revalidate` keep only its `user_id`.

## Files

Three reads list files, read one file's record and read its content:

| id | pinned `operationId` | request | scope | answer |
|---|---|---|---|---|
| `files.list` | `files_list` | `GET /api/files.list` | `files:read` | `files`, each with `id`, `name`, `filetype`, `mimetype`, `size` and `url_private`, and `paging` |
| `files.info` | `files_info` | `GET /api/files.info` | `files:read` | `file`, the same record for one file |
| `file.download` | none | `GET` of the file's `url_private` on `https://files.slack.com` | none stated: the pinned document has no download method | the content as a binary body |

- **`files.list`** narrows by `channel`, `user`, `types` and `ts_from` and
  `ts_to` (Slack `ts`). It pages by number, not by cursor: `page` counts from
  1 and `count` sets the page size; each answer carries `paging.page`,
  `paging.total` and, where Slack states it, `paging.pages`. The walk ends on
  the page numbered `paging.pages`, or on one that lists no file.
- **`files.info`** requires `file`, a file id from the list; an input without
  it is refused before any request.
- **`file.download`** takes one input, `url`: the file's `url_private` or
  `url_private_download` as `files.list` or `files.info` gives it. It must be an
  `https` URL on `files.slack.com` whose path starts with `/files-pri/`; any
  other URL is refused before any request. It is sent as one GET with the bot
  token in the `Authorization` header, which Slack's file host requires. The
  answer is `{media_type, length, sha256, content_base64}`, the bytes in
  base64; a file over 2 MiB is refused as `capacity`, never truncated
  ([binary reads](local-catalog-provider.md#configure-the-provider)).

The download reaches a host besides `api_base`, so the connection must admit it
with the credential. Add `hosts` to the bot configuration
([Authentication](#authentication)):

```json
"hosts": [{"origin": "https://files.slack.com", "credential": true}]
```

Without it the configuration still loads and every other read works, but
`file.download` is refused as `forbidden` with nothing sent. Admitted without
`credential`, the request carries no token and Slack's file host refuses it.
Adding `hosts` changes the configuration revision.

## Messages

Two writes post a message, one edits it and one deletes it. Each is a
mutation: the host requires an approval that binds the operation and the whole
input by digest, records the attempt, and sends the request once
([the catalog provider guide](local-catalog-provider.md)).

| id | pinned `operationId` | request | scope | body | answer |
|---|---|---|---|---|---|
| `chat.postMessage` | `chat_postMessage` | `POST /api/chat.postMessage`, JSON body | `chat:write` | `channel` and `text` required; `thread_ts`, `mrkdwn`, `unfurl_links` | `channel`, `ts` and `message` |
| `conversations.open` | `conversations_open` | `POST /api/conversations.open`, JSON body | `im:write`; `mpim:write` for a group | `users` required; `return_im` | `channel.id` |
| `chat.update` | `chat_update` | `POST /api/chat.update`, JSON body | `chat:write` | `channel`, `ts` and `text`, all required | `channel`, `ts`, `text` and `message` |
| `chat.delete` | `chat_delete` | `POST /api/chat.delete`, JSON body | `chat:write` | `channel` and `ts`, both required | `channel` and `ts` |

- **The body is closed.** Only the keys above are admitted: `channel`, `text`,
  `thread_ts`, `ts` and `users` as strings, the rest as JSON booleans. Any other
  key, `token` included, is refused before any request. `blocks`,
  `attachments`, `as_user`, `parse`, `link_names` and the other members the
  pinned document declares are not admitted. `channel`, `text`, `ts` and
  `users` must not be the empty string.
- **`chat.postMessage`** posts `text` to `channel`, a channel id or the direct
  message `conversations.open` answers; the bot must be in the channel.
  `thread_ts`, the parent message's `ts`, makes it a reply in that thread.
  `mrkdwn` set to `false` turns Slack markup off; `unfurl_links` set to `true`
  unfurls text links. The answer's `channel` and `ts` are the posted message's
  address, the pair `conversations.replies` and a later edit take. Nothing
  about a new message exists to compare before it is sent, so the same input
  approved and sent again posts a second message.
- **`conversations.open`** takes `users`: one user id (`U…`) for a direct
  message, or a comma-separated list of user ids for a multi-person one.
  `return_im` set to `true` asks for the whole conversation in the answer. Its
  `channel.id` (`D…` for a direct message) is the `channel` to post to; the
  pinned answer declares `already_open` and `no_op` for a conversation that
  was open before. A direct message
  is two invocations, open then post, each with its own approval and attempt.
- **`chat.update`** replaces the text of the message `channel` and `ts` name,
  in a channel or a direct message, with `text`, and answers the same
  `channel` and `ts` with the new `text`. The pair is the one
  `chat.postMessage` answered, or one `conversations.history` lists; `channel`
  is the conversation id (`C…`, `D…`), not a name. The message's `blocks` and
  `attachments` stay as they were, so a message that shows blocks may still
  show them. The pinned document says an omitted `parse` or `link_names` is
  reset to its default (`client`, `none`); neither is admitted, so every edit
  resets both. The same edit approved and sent again sets the same text.
- **`chat.delete`** deletes the message `channel` and `ts` name and answers
  that `channel` and `ts`. `ts` is the string Slack gives, such as
  `"1780000700.000700"`, although the pinned document types it as a number; a
  number is refused before any request. A message deleted once is not found
  the second time (`message_not_found`).
- **Only the bot's own messages.** A bot token edits and deletes only the
  messages it posted. Slack answers any other message `200` with
  `"ok": false` and `cant_update_message` or `cant_delete_message`.
- **The guard.** Before each write the provider reads `GET /api/auth.test`
  with the connection's token, which needs no scope, and refuses the write
  unless it answers `"ok": true`, the JSON boolean: the write is
  `not_attempted` and nothing is posted
  ([Invoke](local-catalog-provider.md#invoke)). The refusal is `forbidden`
  when `auth.test` answers `"ok"` with anything else (`false`, or a look-alike
  such as the string `"true"`) or answers `404`; `upstream_protocol` when its
  answer carries no `ok` or is not JSON; and otherwise as its status reads, as
  for any read: `unauthorized` for a `401`, `rate_limited` for a `429`,
  `unavailable` for a `5xx`. After the write Slack's answer must carry
  `"ok": true` too, as the boolean.
- **What the caller sees.** A `2xx` answer with `"ok": true` is applied, and
  the provider returns `status`, `body` (Slack's answer) and `provenance`.
  Slack refuses a write — `channel_not_found`, `not_in_channel`,
  `missing_scope`, `user_not_found` — as HTTP `200` with `"ok": false`. The
  engine classifies by status, so the guard's check of `ok` is what keeps that
  answer from being recorded as applied: it is `outcome_unknown`, and Slack's
  `error` string is not returned. Read the conversation to see whether the
  message is there. A dispatch without a definite answer, a lost connection or
  a `5xx`, is `outcome_unknown` as well. An unknown write is never sent again
  by the provider or the host.
- **Edit and delete refusals are named.** For `chat.update` and `chat.delete`
  the selection declares the `error` values that leave the message unchanged
  (`postflight.refusals`,
  [the catalog provider guide](local-catalog-provider.md#configure-the-provider)).
  Such an answer is classified `refused`, not `outcome_unknown`, with the
  code below as the provider's own answer and the `error` value as its
  reason, and it is never sent again:

  | `error` | method | refused with |
  |---|---|---|
  | `cant_update_message`, `edit_window_closed` | `chat.update` | `forbidden` |
  | `cant_delete_message`, `compliance_exports_prevent_deletion` | `chat.delete` | `forbidden` |
  | `message_not_found`, `channel_not_found` | both | `not_found` |
  | `msg_too_long`, `no_text` | `chat.update` | `invalid_input` |

  Each value is one the pinned document's error enum lists for that method.
  Any other `"ok": false` answer to an edit or a delete, such as
  `not_in_channel`, `is_inactive` or `invalid_auth`, is `outcome_unknown`,
  as for the other two writes.

## Errors are refusals

Slack reports most failures of a read — `channel_not_found`, `not_in_channel`,
`missing_scope`, `invalid_auth` — as HTTP `200` with `"ok": false` and an
`error` string. Every read that answers JSON, in both sets, declares Slack's
success envelope (`envelope`,
[the catalog provider guide](local-catalog-provider.md#configure-the-provider)),
so such an answer is refused, never returned as an empty read: Slack's `error`
is the provider's reason, and the refusal's code is the provider's own answer:

| `error` | refused with |
|---|---|
| `missing_scope`, `not_in_channel`, `no_permission`, `user_is_bot` | `forbidden` |
| `not_authed`, `invalid_auth`, `account_inactive`, `token_revoked` | `unauthorized` |
| `channel_not_found`, `thread_not_found`, `file_not_found`, `file_deleted`, `user_not_found` | `not_found` |
| `ratelimited` | `rate_limited`, with the delay its `Retry-After` names |
| `invalid_cursor`, `invalid_limit`, `limit_required`, `invalid_ts_latest`, `invalid_ts_oldest`, `invalid_arg_name`, `invalid_array_arg`, `unknown_type` | `invalid_input` |
| any other name | `internal` |

Every name the pinned document lists for a selected read is either in this
table or one the test leaves to `internal` on purpose, a request the engine
formed wrongly or a failure on Slack's side, and a source refresh that adds a
name fails the test until it is classified. An answer whose `ok` is absent or
not the boolean, or that is not a JSON object, is `upstream_protocol`. A
`ratelimited` answer, like a `429`, is sent once more when the delay it names
fits the invocation's deadline ([Limits](local-catalog-provider.md#limits)).
`file.download` answers bytes, not Slack's envelope, and the writes are
checked by their guard ([Messages](#messages)), so none of them declares the
envelope.

## Authentication

A bot token (`xoxb-…`), sent as `Authorization: Bearer <token>`, under the
profile `slack.bot`. The pinned document's one security scheme, `slackAuth`, is
OAuth 2.0 with an authorization code; the provider does not run that flow, so
install the Slack app in the workspace and use the bot token it issues. The
operations need the read scopes the pinned document names:
`channels:read`, `groups:read`, `im:read` and `mpim:read` for
`conversations.list`, and `channels:history`, `groups:history`, `im:history` and
`mpim:history` for the two message reads, each for the conversation types read,
`users:read` for `users.list`, `team:read` for `team.info`, `emoji:read` for
`emoji.list` and `files:read` for `files.list` and `files.info`; `auth.test` needs
none. The writes need `chat:write` for `chat.postMessage` and `im:write` for
`conversations.open`, with `mpim:write` to open a multi-person conversation.
A bot reads a channel's history only after it has joined that channel.

Connecting proves the token with `GET /api/auth.test` (`auth_test`). Its
`user_id`, the bot user's id, is the credential's subject, of kind
`slack.user`. Slack answers a token it does not accept with `200` and
`"ok": false`, which carries no `user_id`; the connection is then refused as an
unreadable answer (`protocol`), not as an invalid credential. Slack returns
granted scopes in the `x-oauth-scopes` response header, which the provider does
not read, so the profile declares no `scopes` read and no `minimum_scopes`; a
missing scope shows as `"error": "missing_scope"` on the read that needs it.

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "slack",
  "provider": "slack",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://slack.com/api",
  "auth": {
    "profile": "slack.bot",
    "header": "Authorization",
    "bearer": true,
    "label": "Slack bot token",
    "identity": {"path": "auth.test", "kind": "slack.user", "subject_pointer": "/user_id"}
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/slack/operations.json"
}
```

Connect with the credential document `{"token": "<bot token>"}`.

## Reads and search on a user-token connection

A bot token reads only the conversations the bot is a member of: Slack answers
`conversations.history` and `conversations.replies` on any other channel with
`not_in_channel`. A user token (`xoxp-…`) reads every conversation that user
can read, and only a user token can search: the pinned `search.messages`
requires the scope `search:read`, which Slack grants to a user token, not a bot
token. A catalog connection holds one profile and an instance holds one connection, so
these reads are a second Slack instance: the same bundle and `api_base`, the profile `slack.user` with
a user token, the same `auth.test` identity read — whose `user_id` is then the
person's id — and the user selection set. It selects the three conversation
reads, `users.list` and `search.messages`, and no write, so the user
instance never posts, edits or deletes, and the bot instance never
exposes search. Nothing in the local configuration ties a connection's profile
to the bundle's recorded `auth_profile` (`slack.bot`), so this needs no change
to the local host.

| id | pinned `operationId` | request | user scopes | paging |
|---|---|---|---|---|
| `conversations.list` | `conversations_list` | `GET /api/conversations.list` | `channels:read`, `groups:read`, and `im:read` and `mpim:read` for those types | `cursor`, `limit` (1–1000) |
| `conversations.history` | `conversations_history` | `GET /api/conversations.history` | `channels:history`, `groups:history`, `im:history` or `mpim:history`, by the conversation's type | `cursor`, `limit` |
| `conversations.replies` | `conversations_replies` | `GET /api/conversations.replies` | `channels:history`, `groups:history`, `im:history` or `mpim:history`, by the conversation's type | `cursor`, `limit` |
| `users.list` | `users_list` | `GET /api/users.list` | `users:read` | `cursor`, `limit` |
| `search.messages` | `search_messages` | `GET /api/search.messages` | `search:read` | `page` and `count` (1–100), not a cursor |

- **The conversation reads and `users.list`** take the same parameters, bounds
  and end condition as on the bot instance ([above](#the-shipped-selection-set)):
  `conversations.list` bounds `limit` to 1–1000, `conversations.history`
  requires `channel` and `conversations.replies` `channel` and `ts`, and each
  walk ends on an empty `response_metadata.next_cursor`. List the user's
  conversations with `types` set to `public_channel,private_channel` (and
  `mpim`, `im` for those), then read each one's history. A lookup of one user
  by id or e-mail (`users.info`, `users.lookupByEmail`) is not selected on
  either connection: read the list.
- **`query`** is required for search, and an input without it is refused
  before any request. `sort` takes `score` or `timestamp` and `sort_dir` `asc`
  or `desc`; `highlight` set to `true` adds Slack's highlight markers.
- **Search paging.** Search pages by number, not by cursor: `page` selects the
  page and `count` its size. The pinned `count` description says "Maximum of
  `100`", so the selection bounds `count` to 1–100, and 0, 101 or a value that
  is not an integer is refused before any request. The pinned answer is the
  document's default success template, so the document states neither the
  matches nor where the paging totals are; read them from Slack's answer.
- **The user's reach is the user's.** The connection reads what that person
  can read in Slack, the private channels and direct messages they are in
  included. Grant only the user scopes above.

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "slack-search",
  "provider": "slack",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://slack.com/api",
  "auth": {
    "profile": "slack.user",
    "header": "Authorization",
    "bearer": true,
    "label": "Slack user token",
    "identity": {"path": "auth.test", "kind": "slack.user", "subject_pointer": "/user_id"}
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/slack/user-operations.json"
}
```

Connect with the credential document `{"token": "<user token>"}`.

## Configure the two instances

Each instance names one of the two native configurations above, saved as `slack-bot.json` and
`slack-user.json`, so it pins its entry as the
[catalog provider guide](local-catalog-provider.md#bind-the-provider-to-the-local-cli)
describes: print each file's bootstrap, hash the provider, then add both instances to a
`connectors-local/5` configuration. The bot instance permits the writes too, so it selects
private protocol two; the user instance has no write and stays on protocol one.

```sh
target/release/connectors-catalog-provider --local-config /absolute/path/slack-bot.json --print-local-bootstrap
target/release/connectors-catalog-provider --local-config /absolute/path/slack-user.json --print-local-bootstrap
sha256sum target/release/connectors-catalog-provider
```

```toml
[connectors.slack]
enabled = true

[connectors.slack.instances.bot]
alias = "slack"
instance_id = "slack"
adapter_id = "catalog"
configuration_revision = "REPLACE_FROM_BOT_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/2"
startup = "on-demand"
restart = "never"

[connectors.slack.instances.bot.permissions]
profiles = ["slack.bot"]
operations = ["conversations.list", "conversations.history", "conversations.replies", "users.list", "team.info", "emoji.list", "auth.test", "files.list", "files.info", "file.download", "chat.postMessage", "conversations.open", "chat.update", "chat.delete"]

[connectors.slack.instances.bot.executable]
path = "/absolute/path/connectors-catalog-provider"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/slack-bot.json"]

[connectors.slack.instances.search]
alias = "slack-search"
instance_id = "slack-search"
adapter_id = "catalog"
configuration_revision = "REPLACE_FROM_USER_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/1"
startup = "on-demand"
restart = "never"

[connectors.slack.instances.search.permissions]
profiles = ["slack.user"]
operations = ["search.messages", "conversations.list", "conversations.history", "conversations.replies", "users.list"]

[connectors.slack.instances.search.executable]
path = "/absolute/path/connectors-catalog-provider"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/slack-user.json"]
```

`instance_id` is each native file's `instance`. The connector name `slack` is a name only: a
pinned instance's connector is not checked against a declaration. `--connector slack --instance
bot` and `--connector slack --instance search` select them:

```console
$ connectors --output json setup check | jq -c '[.result.prerequisites[]|{name,state}]'
[{"name":"configuration","state":"ready"},{"name":"metadata","state":"ready"},{"name":"artifact:slack","state":"ready"},{"name":"artifact:slack-search","state":"ready"},{"name":"keyring","state":"ready"},{"name":"persistent_custody_qualification","state":"ready"}]
$ connectors --output json instances list | jq -c '[.. | objects | select(.instance_id) | {connector, instance, instance_id}]'
[{"connector":"slack","instance":"bot","instance_id":"slack"},{"connector":"slack","instance":"search","instance_id":"slack-search"}]
```

Connect each with its own token, entered at the hidden prompt:

```sh
connectors --output json connections connect --connector slack --instance bot --profile slack.bot --credential-prompt
connectors --output json connections connect --connector slack --instance search --profile slack.user --credential-prompt
```

A token Slack later refuses makes its connection `stale` until `connections reconnect` publishes
a new one ([When Slack refuses a token](#when-slack-refuses-a-token)).

## When Slack refuses a token

Slack answers a revoked or invalid token with `"ok": false` and `invalid_auth` (or
`token_revoked`), which the provider refuses as `unauthorized`. The connection then turns
`stale`: it stays listed, revalidation cannot help, and `connections reconnect` replaces the
token under the same connection reference and revision. In this run the fixture stopped
accepting the bot token, then accepted it again:

```console
$ connectors operations invoke --connector slack --instance bot --connection conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b --operation conversations.list --schema 2771ae0cbcfa853c69da89e1603d017e397120ce4ffd0891c23ed0d0b5f3f365 --revision 87a55803d552e77cbfd7ed338bf86133549150a2f65b4d9812ca1328df1acce6 --input-json '{}'
{"error":{"code":"failure","data":{"code":"service_failure","kind":"operational","next_action":"repair_connection","service_code":"unauthorized","service_reason":"invalid_auth","stage":"dispatch"}},"ok":false}
$ connectors connections list --connector slack --instance bot | jq -c '[.result.connections[]|{connection,revision,state}]'
[{"connection":"conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b","revision":"691cd61d-0a34-4ba9-8ab2-f499f90c71f6","state":"stale"}]
$ connectors connections revalidate --connector slack --instance bot --connection conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b --expected-revision 691cd61d-0a34-4ba9-8ab2-f499f90c71f6
{"error":{"code":"failure","data":{"code":"not_granted","kind":"operational","next_action":"repair_connection","stage":"admission"}},"ok":false}
$ connectors connections list --connector slack --instance bot | jq -c '[.result.connections[]|{connection,revision,state}]'
[{"connection":"conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b","revision":"691cd61d-0a34-4ba9-8ab2-f499f90c71f6","state":"stale"}]
$ printf '{"token":"xoxb-fixture-token"}' | connectors connections reconnect --connector slack --instance bot --expected-revision 691cd61d-0a34-4ba9-8ab2-f499f90c71f6 --credential-stdin | jq -c '.result.connection.summary|{connector,instance,connection,revision,state}'
{"connector":"slack","instance":"bot","connection":"conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b","revision":"691cd61d-0a34-4ba9-8ab2-f499f90c71f6","state":"ready"}
$ connectors connections list --connector slack --instance bot | jq -c '[.result.connections[]|{connection,revision,state}]'
[{"connection":"conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b","revision":"691cd61d-0a34-4ba9-8ab2-f499f90c71f6","state":"ready"}]
```

## Moving a local install off a credential-only instance

This section covers a local install that already reads Slack through the catalog provider with
three legacy `[adapters.<alias>]` entries: `slack-bot` (profile `slack.bot`) serves the ten bot
reads, `slack-search` (profile `slack.user`) serves `search.messages` and the user reads, and an
older `slack`, which serves no operations, still holds a connection. It moves the three entries to
instances of the connector `slack`, enables the two writes on the bot instance, and retires the old
instance without touching the credentials the other two use.

The outputs below come from a run against a scratch `--config` and `--state-dir`, a private
Secret Service and a local HTTPS fixture standing in for `https://slack.com/api` (synthetic token,
synthetic ids), with the native files in `~`. The commands are shown without `--config`,
`--state-dir` and `--output json`, and answers are cut down with the `jq` filter shown. A
connection id, revision or incarnation in your install differs: take each from the command before
it.

### Move the entries to instances

`setup migrate` keeps Slack entries where they are: the catalog provider ships no Slack
connector declaration.

```console
$ connectors setup migrate --mode dry-run | jq -c '[.result.entries[]|{connector,instance,disposition,reason}]'
[{"connector":"catalog","instance":"slack","disposition":"kept","reason":"connector_undeclared"},{"connector":"catalog","instance":"slack-bot","disposition":"kept","reason":"connector_undeclared"},{"connector":"catalog","instance":"slack-search","disposition":"kept","reason":"connector_undeclared"}]
```

Each entry keeps working as it is, selected as `--connector catalog --instance <alias>` or by the
deprecated `--adapter <alias>`. To name it as a Slack instance, rewrite it by hand: change
`format` to `"connectors-local/5"`, add `[connectors.slack]` with `enabled = true`, and rename
each legacy table `[adapters.<alias>]` to `[connectors.slack.instances.<instance>]` with `alias =
"<alias>"` as its first key, the `.permissions` and `.executable` tables with it. Every pin stays
as it was, so the connections, the approval policy and the custody items stay valid. Here the
instances are `bot` (`slack-bot`), `search` (`slack-search`) and `legacy` (`slack`):

```toml
[connectors.slack]
enabled = true

[connectors.slack.instances.bot]
alias = "slack-bot"
instance_id = "slack-bot"
adapter_id = "catalog"
# configuration_revision, protocol, private_protocol, startup, restart and the
# .permissions and .executable tables exactly as in the legacy [adapters.slack-bot]
```

```console
$ connectors setup check | jq -c '[.result.prerequisites[]|{name,state}]'
[{"name":"configuration","state":"ready"},{"name":"metadata","state":"ready"},{"name":"artifact:slack","state":"ready"},{"name":"artifact:slack-bot","state":"ready"},{"name":"artifact:slack-search","state":"ready"},{"name":"keyring","state":"ready"},{"name":"persistent_custody_qualification","state":"ready"}]
$ connectors instances list | jq -c '[.. | objects | select(.instance_id) | {connector, instance, instance_id}]'
[{"connector":"slack","instance":"legacy","instance_id":"slack"},{"connector":"slack","instance":"bot","instance_id":"slack-bot"},{"connector":"slack","instance":"search","instance_id":"slack-search"}]
$ connectors connections list --connector slack --instance bot | jq -c '[.result.connections[]|{connector,instance,connection,revision,state}]'
[{"connector":"slack","instance":"bot","connection":"conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b","revision":"8da8d006-a30a-41ae-b1d4-25030548c793","state":"ready"}]
```

### Enable `chat.postMessage` and `conversations.open` on `bot`

Three things gate a write:

- **The native configuration's selection contains it.** The shipped
  [`operations.json`](../adapters/catalog/providers/slack/operations.json) has the ten reads and both
  writes. A selection holding only the ten reads has to be replaced by it.
- **The instance selects private protocol two.** Under `connectors-private/1` the provider
  offers its reads only.
- **The instance permits both ids.** An id missing from `permissions.operations` is denied.

The native configuration must stay owner-only (mode `0600`): a file other users can read is
refused as `invalid_configuration` when the provider loads it.

First check which selection the native configuration names and the revision it gives:

```console
$ jq -r '.operations_file | split("/") | last' ~/slack-bot.json
bot-reads.json
$ connectors-catalog-provider --local-config ~/slack-bot.json --print-local-bootstrap | jq -r .configuration_revision
29905332e2bb4fb92993fb5a5d5db2f6102475fde0b0fdbd2ac1b1588211236d
```

That selection is a ten-read file, so point `operations_file` at the shipped `operations.json` and
print the revision again:

```console
$ connectors-catalog-provider --local-config ~/slack-bot.json --print-local-bootstrap | jq -r .configuration_revision
c83a994655247e5ca6c4806cece66436a0ed52062ea95bab223dc8ccb51788c5
```

The revision digests the selected operations, so it moved. If `operations_file` already names the
shipped file, it does not move, and only the instance changes. Then change the instance: the new
revision, protocol two, and the two write ids. As a diff of the configuration used here:

```diff
-configuration_revision = "29905332e2bb4fb92993fb5a5d5db2f6102475fde0b0fdbd2ac1b1588211236d"
+configuration_revision = "c83a994655247e5ca6c4806cece66436a0ed52062ea95bab223dc8ccb51788c5"
-private_protocol = "connectors-private/1"
+private_protocol = "connectors-private/2"
-operations = ["conversations.list", "conversations.history", "conversations.replies", "users.list", "team.info", "emoji.list", "auth.test", "files.list", "files.info", "file.download"]
+operations = ["conversations.list", "conversations.history", "conversations.replies", "users.list", "team.info", "emoji.list", "auth.test", "files.list", "files.info", "file.download", "chat.postMessage", "conversations.open"]
```

When the revision moved, the existing connection is `pending` and the page is `stale`:

```console
$ connectors connections list --connector slack --instance bot | jq -c '{stale: .result.stale, connections: [.result.connections[]|{connection,revision,state}]}'
{"stale":true,"connections":[{"connection":"conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b","revision":"8da8d006-a30a-41ae-b1d4-25030548c793","state":"pending"}]}
```

Revalidate. The instance id, `adapter_id` (`catalog`), the provider authority and the profile
are unchanged, so revalidation upgrades the connection in place from its saved credential, with
the same identity and no token entered, and stops the process still running the earlier entry
([configuration upgrades](local-connection-registry.md#configuration-upgrades)):

```console
$ connectors connections revalidate --connector slack --instance bot --connection conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b --expected-revision 8da8d006-a30a-41ae-b1d4-25030548c793 | jq -c '.result.connection|{identity: .external_identity, summary: (.summary|{connector,instance,connection,revision,state})}'
{"identity":{"kind":"slack.user","subject":"U0000000001"},"summary":{"connector":"slack","instance":"bot","connection":"conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b","revision":"691cd61d-0a34-4ba9-8ab2-f499f90c71f6","state":"ready"}}
$ connectors operations list --connector slack --instance bot | jq -c '[.result.operations[].id]'
["conversations.list","conversations.history","conversations.replies","users.list","team.info","emoji.list","auth.test","files.list","files.info","chat.postMessage","conversations.open","file.download"]
```

When the revision did not move, the connection stays `ready` and keeps its revision; stop the
instance's running process with the exact values `instances status` gives (`instances stop
--expected-revision --host-incarnation --child-incarnation`) so that the next command starts it
under the edited entry.

#### The approval policy

A write needs a signing key ([approval keys](local-approval-keys.md)), an authenticated clock
([the clock](local-clock.md)) and a published policy naming it ([approvals](local-approvals.md)).
The clock is a top-level table of the configuration:

```toml
[approval_clock]
format = "roughtime-quorum/1"
quorum = 2
max_rate_error_ppm = 10000

[[approval_clock.sources]]
address = "51.81.57.55:2002"
public_key = "AW5uAoTSTDfG5NfY1bTh08GUnOqlRb+HVhbJ3ODJvsE="
version = "roughtime-0x8000000c"

[[approval_clock.sources]]
address = "148.113.255.71:2002"
public_key = "iBVjxg/1j7y1+kQUTBYdTabxCppesU/07D4PMDJk2WA="
version = "roughtime-0x8000000c"
```

[The clock](local-clock.md) lists the four public sources and how many a quorum needs.

```console
$ connectors approvals key-init --connector slack --instance bot | jq -c '.result|{connector,instance,keys: [.issuer.keys[]|{key_id,state}]}'
{"connector":"slack","instance":"bot","keys":[{"key_id":"f0bfff44-1cde-46f3-8d2d-fca4522ef5d1","state":"active"}]}
$ connectors approvals policy-status --connector slack --instance bot | jq -c '.result|{connector,instance,policy}'
{"connector":"slack","instance":"bot","policy":null}
$ connectors approvals policy-set --connector slack --instance bot --input-file ~/private/slack-policy.json | jq -c '.result.policy|{instance_id,operations,revision}'
{"instance_id":"slack-bot","operations":["chat.postMessage","conversations.open"],"revision":1}
```

`~/private/slack-policy.json` holds `{"operations":["chat.postMessage","conversations.open"]}`.
The policy records the configuration revision, descriptor revision, executable and clock it was
published under; after any of them changes, publish it again with
`--expected-revision <current>`.

#### Giving an approval

Each write is approved by digest over its exact input, once. `approvals prepare` resolves the
subject without the clock, the provider or the credential. Take `--schema` and `--revision` from
`operations describe --connector slack --instance bot --operation chat.postMessage`. The input is
the JSON body under `body`:

```console
$ connectors approvals prepare --connector slack --instance bot --connection conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b --operation chat.postMessage --schema fd40561da56fd3a5b2f1cd9fe47106914c45c80dc7cc27dc4bdca4f9a45efae2 --revision 87a55803d552e77cbfd7ed338bf86133549150a2f65b4d9812ca1328df1acce6 --input-file ~/private/post.json | jq -c '.result.preparation|{subject_sha256, presentation}'
{"subject_sha256":"5da23cd1ba119d43b499bb9689ab76e210e6dfbeec7dbda0ac65e025d299bf5d","presentation":{"connection":"conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b","descriptor_revision":"87a55803d552e77cbfd7ed338bf86133549150a2f65b4d9812ca1328df1acce6","instance":"slack-bot","operation":"chat.postMessage"}}
```

`~/private/post.json` holds `{"body":{"channel":"C0000000001","text":"migration check"}}`.
Issuing needs a fresh clock sample, so run `approvals clock-check` first. Approving the subject
publishes a proof file, which the invocation then spends; the fixture answered the post:

```console
$ connectors approvals clock-check --connector slack --instance bot | jq -c '.result.observation.sources|map({address,result})'
[{"address":"51.81.57.55:2002","result":"agreed"},{"address":"148.113.255.71:2002","result":"agreed"}]
$ connectors approvals issue --connector slack --instance bot --connection conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b --operation chat.postMessage --schema fd40561da56fd3a5b2f1cd9fe47106914c45c80dc7cc27dc4bdca4f9a45efae2 --revision 87a55803d552e77cbfd7ed338bf86133549150a2f65b4d9812ca1328df1acce6 --input-file ~/private/post.json --approve-subject 5da23cd1ba119d43b499bb9689ab76e210e6dfbeec7dbda0ac65e025d299bf5d --proof-output ~/private/proof-post-1.json | jq -c '.result|{connector,instance,disposition,subject_sha256}'
{"connector":"slack","instance":"bot","disposition":"published","subject_sha256":"5da23cd1ba119d43b499bb9689ab76e210e6dfbeec7dbda0ac65e025d299bf5d"}
$ connectors operations invoke --connector slack --instance bot --connection conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b --operation chat.postMessage --schema fd40561da56fd3a5b2f1cd9fe47106914c45c80dc7cc27dc4bdca4f9a45efae2 --revision 87a55803d552e77cbfd7ed338bf86133549150a2f65b4d9812ca1328df1acce6 --input-file ~/private/post.json --approval-file ~/private/proof-post-1.json --idempotency-key post-1 | jq -c '.result|{classification: .mutation.classification, status: .result.status, body: .result.body}'
{"classification":"applied","status":200,"body":{"channel":"C0000000001","message":{"text":"migration check","ts":"1780000700.000700"},"ok":true,"ts":"1780000700.000700"}}
```

The invocation sends a real message, which [Messages](#messages) describes.

### Retire the credential-only `legacy` instance

The connections of `bot` and `search` do not share a credential with that of `legacy`. A
connection belongs to one instance, and each stored credential version belongs to exactly one
connection. Revoking a connection retires only its own versions. Check that before removing
anything.

1. **Prove the survivors.** List them, then revalidate each. Revalidation reads the connection's
   saved credential from custody and runs the identity read, with nothing entered, so `ready` means
   its own custody item is there. A read on `bot` shows the connection in use:

   ```console
$ connectors connections list --connector slack --instance bot | jq -c '[.result.connections[]|{connection,revision,state}]'
[{"connection":"conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b","revision":"691cd61d-0a34-4ba9-8ab2-f499f90c71f6","state":"ready"}]
$ connectors connections list --connector slack --instance search | jq -c '[.result.connections[]|{connection,revision,state}]'
[{"connection":"conn_352bbc77ccfb465fbef6d1648d68811a_c019ec9863e6448fa9d13fb2ad322cd8","revision":"87b27e15-155e-4ac5-953b-22b00189b442","state":"ready"}]
$ connectors connections revalidate --connector slack --instance bot --connection conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b --expected-revision 691cd61d-0a34-4ba9-8ab2-f499f90c71f6 | jq -c '.result.connection|{state:.summary.state,revision:.summary.revision,identity:.external_identity}'
{"state":"ready","revision":"691cd61d-0a34-4ba9-8ab2-f499f90c71f6","identity":{"kind":"slack.user","subject":"U0000000001"}}
$ connectors connections revalidate --connector slack --instance search --connection conn_352bbc77ccfb465fbef6d1648d68811a_c019ec9863e6448fa9d13fb2ad322cd8 --expected-revision 87b27e15-155e-4ac5-953b-22b00189b442 | jq -c '.result.connection|{state:.summary.state,revision:.summary.revision,identity:.external_identity}'
{"state":"ready","revision":"87b27e15-155e-4ac5-953b-22b00189b442","identity":{"kind":"slack.user","subject":"U0000000001"}}
$ connectors operations invoke --connector slack --instance bot --connection conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b --operation conversations.list --schema 2771ae0cbcfa853c69da89e1603d017e397120ce4ffd0891c23ed0d0b5f3f365 --revision 87a55803d552e77cbfd7ed338bf86133549150a2f65b4d9812ca1328df1acce6 --input-json '{}' | jq -c '.result.result|{status,body}'
{"status":200,"body":{"channels":[{"id":"C0000000001","name":"general"}],"ok":true,"response_metadata":{"next_cursor":""}}}
   ```

   `pending` is expired validation evidence, not a missing credential (60 seconds unless the profile
   sets `evidence_lifetime_ms`), and revalidation renews it. Stop here if either survivor does not
   end `ready`.

2. **List the old connection** with its revision:

   ```console
$ connectors connections list --connector slack --instance legacy | jq -c '[.result.connections[]|{connection,revision,state}]'
[{"connection":"conn_352bbc77ccfb465fbef6d1648d68811a_f544efc6c39448a59eac20bb7bb901a6","revision":"b92de992-263a-4a62-b8b0-0f1190bbbe2b","state":"ready"}]
   ```

3. **Revoke it.** Revocation is terminal and local. It asks nothing of Slack
   (`provider_outcome` `not_requested`), so the token stays valid there; revoke it in the Slack
   app if it is to stop working.

   ```console
$ connectors connections revoke --connector slack --instance legacy --connection conn_352bbc77ccfb465fbef6d1648d68811a_f544efc6c39448a59eac20bb7bb901a6 --expected-revision b92de992-263a-4a62-b8b0-0f1190bbbe2b | jq -c '.result|{connector,instance,connection,local_state,provider_outcome}'
{"connector":"slack","instance":"legacy","connection":"conn_352bbc77ccfb465fbef6d1648d68811a_f544efc6c39448a59eac20bb7bb901a6","local_state":"revoked","provider_outcome":"not_requested"}
   ```

4. **Check again.** The old connection is `revoked`, and the survivors still revalidate to `ready`:

   ```console
$ connectors connections list --connector slack --instance legacy | jq -c '[.result.connections[]|{connection,state}]'
[{"connection":"conn_352bbc77ccfb465fbef6d1648d68811a_f544efc6c39448a59eac20bb7bb901a6","state":"revoked"}]
$ connectors connections revalidate --connector slack --instance bot --connection conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b --expected-revision 691cd61d-0a34-4ba9-8ab2-f499f90c71f6 | jq -r '.result.connection.summary.state'
ready
$ connectors connections revalidate --connector slack --instance search --connection conn_352bbc77ccfb465fbef6d1648d68811a_c019ec9863e6448fa9d13fb2ad322cd8 --expected-revision 87b27e15-155e-4ac5-953b-22b00189b442 | jq -r '.result.connection.summary.state'
ready
   ```

5. **Remove `[connectors.slack.instances.legacy]`** and its `.permissions` and `.executable`
   tables from the configuration, then check:

   ```console
$ connectors setup check | jq -c '[.result.prerequisites[]|{name,state}]'
[{"name":"configuration","state":"ready"},{"name":"metadata","state":"ready"},{"name":"artifact:slack-bot","state":"ready"},{"name":"artifact:slack-search","state":"ready"},{"name":"keyring","state":"ready"},{"name":"persistent_custody_qualification","state":"ready"}]
$ connectors instances list | jq -c '[.. | objects | select(.instance_id) | {connector, instance, instance_id}]'
[{"connector":"slack","instance":"bot","instance_id":"slack-bot"},{"connector":"slack","instance":"search","instance_id":"slack-search"}]
$ connectors connections list --connector slack --instance legacy
{"error":{"code":"failure","data":{"code":"not_found","kind":"operational","next_action":"check_configuration","stage":"admission"}},"ok":false}
$ connectors connections revalidate --connector slack --instance bot --connection conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b --expected-revision 691cd61d-0a34-4ba9-8ab2-f499f90c71f6 | jq -r '.result.connection.summary.state'
ready
$ connectors connections revalidate --connector slack --instance search --connection conn_352bbc77ccfb465fbef6d1648d68811a_c019ec9863e6448fa9d13fb2ad322cd8 --expected-revision 87b27e15-155e-4ac5-953b-22b00189b442 | jq -r '.result.connection.summary.state'
ready
$ connectors approvals policy-status --connector slack --instance bot | jq -c '.result.policy.operations'
["chat.postMessage","conversations.open"]
   ```

Revoke before removing the instance. Once it is gone, every connection command answers
`not_found` for `--connector slack --instance legacy`, as above. A connection left unrevoked then
can no longer be revoked through the CLI, and its metadata and custody item stay behind. Revoked
credential versions are kept for at least 24 hours after retirement before they may be deleted
([retirement](local-connection-registry.md#read-use-and-retirement)). This run does not establish
whether that deletion still happens once the instance is removed.

### `owner_build_mismatch` after a CLI upgrade

The owner is the background `connectors` process that serves a state directory. When the CLI is
upgraded while an owner started by the previous build still runs, every command that would talk to
that owner is refused with `owner_build_mismatch`, stage `readiness`, `next_action: stop_owner`
([owner contract](../contracts/cli/v1alpha1/owner.md)), `instances stop` and
`connections revalidate` included. Passive reads of the store such as `connections list` still
answer.

`connectors setup owner-stop`, run with the same `--config` and `--state-dir`, asks that owner to
drain and exit and waits up to 30 seconds for it; it answers `"disposition": "stopped"`, or
`"no_owner"` when none runs, and never signals a process
([owner contract](../contracts/cli/v1alpha1/owner.md#stopping-an-owner),
[CLI guide](../website/docs/guides/set-up-the-local-cli.md#after-an-upgrade)). An idle owner also
exits by itself after 10 minutes with no connected client and no work in flight. The next command
that needs an owner starts one from the current build:

```console
$ connectors setup owner-stop | jq -c '.result.disposition'
"stopped"
$ connectors connections revalidate --connector slack --instance bot --connection conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b --expected-revision 691cd61d-0a34-4ba9-8ab2-f499f90c71f6 | jq -r '.result.connection.summary.state'
ready
$ connectors instances status --connector slack --instance bot | jq -r .result.observation.state
ready
```

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/slack.rs`)
  with synthetic ids and messages: the exact request of each read, including
  the `oldest`/`latest` window and the bearer header, with no `token` query
  parameter or header, the returned body bytes, a two-page walk of each list to
  an empty `next_cursor`, each classified `ok: false` error name and an
  unnamed one refused with its code and Slack's `error` as the reason, a
  `ratelimited` answer sent once more only when its `Retry-After` fits the
  deadline, an answer without a boolean `ok` refused as unreadable, every JSON
  read of both sets refused on a token Slack answers `invalid_auth`, the
  `auth.test` identity read and its refusal, and, on a second instance with a
  user token, search, the three conversation reads and `users.list`, a
  private channel the bot is not in included. The file reads are checked the same way: a two-page
  `files.list` walk, `files.info` of one file, and the download of a
  non-UTF-8 body from a fixture file host admitted with and without the
  credential, refused when the connection does not admit the host or the URL
  leaves `/files-pri/`; the fixture serves its own origin in place of
  `files.slack.com`. The writes are checked through prepare and commit on
  private protocol two: the `auth.test` read and then one JSON `POST` with the
  body as supplied, the `channel` and `ts` of the answer, refusal without an
  approval or with a key outside the closed body, refusal before the write when
  `auth.test` answers `ok: false`, a dropped connection and a `500` left
  unknown and sent once, an `ok: false` answer left unknown, and a direct
  message opened and posted to. The host's approval and attempt record are
  not part of these tests. No live Slack workspace has been called.
- The engine parses and re-serialises the body. The fixture's bodies are
  compact JSON with sorted keys, and the test asserts the returned body is
  those exact bytes; a Slack body with other spacing or key order is returned
  as equal JSON, not as its exact bytes.
- The provider does not walk pages itself, and does not bound `limit` on the
  message reads or `users.list`: the pinned document states no maximum for
  them.
- Only the bot set's ten reads and four writes and the user set's five reads
  are selected. One user by id or e-mail (`users.info`, `users.lookupByEmail`),
  file upload and delete, reactions and every other method of the document are
  not selected.
  A file upload is a binary request body, which the catalog does not send.

## Credential parameters

Four read methods of the pinned document declare the credential as a
**required** parameter: `auth.test` as a required `token` header, and
`team.info`, `emoji.list` and `search.messages` as a required `token` query
parameter. `withhold` refuses a parameter the document requires
("withholds `token`, which is required"); selected as they are, the three
would declare the token as a required input sent in the query, and
`auth.test` is refused as needing a header parameter the transport does not
carry.

Their selections name `token` in `credential` instead
([the selection format](local-catalog-provider.md)): the token the connection
already sends in its `Authorization` header is then removed from the operation
whether or not the document requires it, before the required-header refusal,
so it is never declared, never required and never sent. A credential name must
be a query or header parameter of the operation that the selection does not
also bound, mark `required`, withhold or read through a guard; anything else
is refused when the selection loads. `withhold` keeps its meaning.
`adapters/catalog/tests/slack.rs` pins that each of the four ships this way and
only this way, and `adapters/catalog/tests/selection_credential.rs` pins the
field itself.
