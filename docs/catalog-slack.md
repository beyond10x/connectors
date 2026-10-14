# Slack through the catalog provider

The catalog provider reads Slack conversations — the channels a bot token can
see, a channel's messages and one thread — the workspace's users, the
workspace itself, its custom emoji, the token's identity and the files the token
can see, with their content, from the pinned Slack Web API document, posts
messages, a direct message included, under approval, and, on a
second connection with a user token, searches messages. Nothing here is
Slack-specific code: the Swagger 2.0 document is projected into OpenAPI 3.1.0, the projection is compiled into a
bundle, two reviewed selection sets expose ten reads and two writes for a bot
token and search for a user token, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them.
Configuration, connection, approval and invocation work as described there;
this page covers what differs for Slack.

## Source and bundle

The pinned source is
[`adapters/slack/upstream/slack_web_openapi_v2_without_examples.json`](../adapters/slack/upstream/README.md),
SHA-256 `8b92da26a3c5b11d20042a9f36d81f1fa6fc9382c5ddc471babb68b91936bc3a`:
`slackapi/slack-api-specs` at commit `bc08db49`, Swagger `2.0`, `info.version`
`1.7.0`, 174 operations. It is projected under the rule table of
`crates/connectors-catalog/src/swagger.rs` (`swagger2-openapi/1`), and the
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
exposes `search.messages` alone, for a user token
([Search](#search-on-a-user-token-connection)). A selection id is the Slack
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
  document states no member beyond `ok`; Slack's answer is returned as it
  came.
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

## Errors are answers

Slack reports most failures of a read — `channel_not_found`, `not_in_channel`,
`missing_scope`, `invalid_auth` — as HTTP `200` with `"ok": false` and an
`error` string. The provider classifies by HTTP status, so such an answer is
returned as a successful invocation with `status` `200` and Slack's body. Read
`body.ok` before reading a page. A rate-limited read is HTTP `429` with
`Retry-After` and is handled as for every catalog read
([Limits](local-catalog-provider.md#limits)).

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

## Search on a user-token connection

Slack searches messages only for a user token (`xoxp-…`): the pinned
`search.messages` requires the scope `search:read`, which Slack grants to a
user token, not a bot token. A catalog connection holds one profile, so search
is a second Slack connection: the same bundle and `api_base`, the profile
`slack.user` with a user token, the same `auth.test` identity read — whose
`user_id` is then the person's id — and the user selection set, which selects
`search.messages` alone, so the bot connection never exposes it and the search
connection exposes nothing else. Nothing in the local configuration ties a
connection's profile to the bundle's recorded `auth_profile` (`slack.bot`), so
this needs no change to the local host.

| id | pinned `operationId` | request | scope | paging |
|---|---|---|---|---|
| `search.messages` | `search_messages` | `GET /api/search.messages` | `search:read`, user token | `page` and `count` (1–100), not a cursor |

- **`query`** is required, and an input without it is refused before any
  request. `sort` takes `score` or `timestamp` and `sort_dir` `asc` or `desc`;
  `highlight` set to `true` adds Slack's highlight markers.
- **Paging.** Search pages by number, not by cursor: `page` selects the page
  and `count` its size. The pinned `count` description says "Maximum of
  `100`", so the selection bounds `count` to 1–100, and 0, 101 or a value that
  is not an integer is refused before any request. The pinned answer is the
  document's default success template, so the document states neither the
  matches nor where the paging totals are; read them from Slack's answer.

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

## Moving a local install off a credential-only adapter

This section covers a local install that already reads Slack through the catalog provider:
adapter `slack-bot` (profile `slack.bot`) serves the ten bot reads, adapter `slack-search`
(profile `slack.user`) serves `search.messages`, and an older adapter `slack`, which serves no
operations, still holds connections. It enables the two writes on `slack-bot`, retires `slack`
without touching the credentials the other two use, and says what to do when an upgraded CLI meets
an owner started by the previous build.

The outputs below come from a run against a scratch `--config` and `--state-dir`, a private
Secret Service and a local HTTPS fixture standing in for `https://slack.com/api` (synthetic token,
synthetic ids). The commands are shown without `--config`, `--state-dir` and `--output json`; long
answers are cut down with the `jq` filter shown. A connection id, revision or incarnation in your
install differs: take each from the command before it.

### Enable `chat.postMessage` and `conversations.open` on `slack-bot`

Three things gate a write:

- **The native configuration's selection contains it.** The shipped
  [`operations.json`](../adapters/catalog/providers/slack/operations.json) has the ten reads and both
  writes. A selection holding only the ten reads has to be replaced by it.
- **The adapter entry selects private protocol two.** Under `connectors-private/1` the provider
  offers its reads only.
- **The entry permits both ids.** An id missing from `permissions.operations` is denied.

The native configuration must stay owner-only (mode `0600`): a file other users can read is
refused as `invalid_configuration` when the provider loads it.

First check which revision the entry runs under and which selection the native configuration names:

```console
$ grep -A3 "^\[adapters.slack-bot\]" config.toml | grep configuration_revision
configuration_revision = "b4b56da145dc78757a1e2c0eee1c0a451ced77268e502f22a7d2121cb504c792"
$ connectors-catalog-provider --local-config slack-bot.json --print-local-bootstrap | jq -r .configuration_revision
b4b56da145dc78757a1e2c0eee1c0a451ced77268e502f22a7d2121cb504c792
$ jq -r .operations_file slack-bot.json
~/.cache/connectors-w20261014b/slack-migration/run/bot-reads.json
```

That selection is a ten-read file, so point `operations_file` at the shipped `operations.json` and
print the revision again:

```console
$ connectors-catalog-provider --local-config slack-bot.json --print-local-bootstrap | jq -r .configuration_revision
45d968e5336a983cdfa69595911151e63297584ef10169e31214b64f5dce4bee
```

The revision digests the selected operations, so it moved. If `operations_file` already names the
shipped file, it does not move, and only the adapter entry changes. Then change the adapter entry:
the new revision, protocol two, and the two write ids. As a diff of the configuration used here:

```diff
-configuration_revision = "b4b56da145dc78757a1e2c0eee1c0a451ced77268e502f22a7d2121cb504c792"
+configuration_revision = "45d968e5336a983cdfa69595911151e63297584ef10169e31214b64f5dce4bee"
-private_protocol = "connectors-private/1"
+private_protocol = "connectors-private/2"
-operations = ["conversations.list", "conversations.history", "conversations.replies", "users.list", "team.info", "emoji.list", "auth.test", "files.list", "files.info", "file.download"]
+operations = ["conversations.list", "conversations.history", "conversations.replies", "users.list", "team.info", "emoji.list", "auth.test", "files.list", "files.info", "file.download", "chat.postMessage", "conversations.open"]
```

When the revision moved, the existing connection is `pending` and the page is `stale`:

```console
$ connectors connections list --adapter slack-bot
{"ok":true,"result":{"connections":[{"adapter":"slack-bot","connection":"conn_c4789c790b6b4fdf944637d74f5317d7_3345b2425b5b47efb18a10301c8feba1","instance_id":"slack-bot","profile":"slack.bot","revision":"c838183e-bc2e-4e9e-8340-f5b5944b592a","state":"pending"}],"observed_at_ms":1791961826537,"source":"authority","stale":true,"valid_until_ms":1791961826537}}
```

An adapter child started before the edit keeps running the old entry, and while it runs,
`connections revalidate` is refused, even when the revision did not move. The same refusal answered
a change of `private_protocol` alone:

```console
$ connectors connections revalidate --adapter slack-bot --connection conn_c4789c790b6b4fdf944637d74f5317d7_3345b2425b5b47efb18a10301c8feba1 --expected-revision c838183e-bc2e-4e9e-8340-f5b5944b592a
{"error":{"code":"failure","data":{"code":"lifecycle_conflict","kind":"operational","next_action":"retry_status","stage":"admission"}},"ok":false}
```

So after editing the entry, stop the running child with the exact values `adapters status` gives:

```console
$ connectors adapters status --adapter slack-bot
{"ok":true,"result":{"observation":{"adapter":"slack-bot","child_incarnation":"65a272ad-ea37-46cb-8cd6-eedd18a726c6","configuration_revision":"b4b56da145dc78757a1e2c0eee1c0a451ced77268e502f22a7d2121cb504c792","host_incarnation":"b1bd440f-6cbb-4895-9387-f7a8702ae714","observed_at_ms":1791961843836,"state":"ready"}}}
$ connectors adapters stop --adapter slack-bot --expected-revision b4b56da145dc78757a1e2c0eee1c0a451ced77268e502f22a7d2121cb504c792 --host-incarnation b1bd440f-6cbb-4895-9387-f7a8702ae714 --child-incarnation 65a272ad-ea37-46cb-8cd6-eedd18a726c6
{"ok":true,"result":{"observation":{"adapter":"slack-bot","configuration_revision":"b4b56da145dc78757a1e2c0eee1c0a451ced77268e502f22a7d2121cb504c792","host_incarnation":"b1bd440f-6cbb-4895-9387-f7a8702ae714","observed_at_ms":1791961850631,"state":"suppressed"}}}
```

Then revalidate. The instance, the adapter id (`catalog`), the provider authority and the profile
are unchanged, so revalidation upgrades the connection in place from its saved credential, with
the same identity and no token entered
([configuration upgrades](local-connection-registry.md#configuration-upgrades)):

```console
$ connectors connections revalidate --adapter slack-bot --connection conn_c4789c790b6b4fdf944637d74f5317d7_3345b2425b5b47efb18a10301c8feba1 --expected-revision c838183e-bc2e-4e9e-8340-f5b5944b592a
{"ok":true,"result":{"connection":{"external_identity":{"kind":"slack.user","subject":"U0000000001"},"observed_at_ms":1791961856801,"source":"authority","stale":false,"summary":{"adapter":"slack-bot","connection":"conn_c4789c790b6b4fdf944637d74f5317d7_3345b2425b5b47efb18a10301c8feba1","instance_id":"slack-bot","profile":"slack.bot","revision":"9559e85e-0df7-4808-ad1d-1b74b0d821f1","state":"ready"},"valid_until_ms":1791961916689}}}
$ connectors operations list --adapter slack-bot | jq -c '[.result.operations[].id]'
["conversations.list","conversations.history","conversations.replies","users.list","team.info","emoji.list","auth.test","files.list","files.info","chat.postMessage","conversations.open","file.download"]
```

When the revision did not move, the connection stays `ready` and keeps its revision: stop the child
as above, and no upgrade happens.

#### The approval policy

A write needs a signing key ([approval keys](local-approval-keys.md)), an authenticated clock
([the clock](local-clock.md)) and a published policy naming it ([approvals](local-approvals.md)).
The clock is a top-level table of the configuration:

```toml
[approval_clock]
format = "roughtime-clock/1"
address = "192.36.143.134:2002"
public_key = "S3AzfZJ5CjSdkJ21ZJGbxqdYP/SoE8fXKY0+aicsehI="
max_rate_error_ppm = 10000
```

```console
$ connectors approvals key-init --adapter slack-bot
{"ok":true,"result":{"adapter":"slack-bot","issuer":{"audience":"connectors.approval/a78448c8-e165-49d6-9c48-8c75059b154c","issuer":"connectors.local-issuer/a78448c8-e165-49d6-9c48-8c75059b154c","keys":[{"key_id":"b5ce5563-d1c2-42ce-a9a6-5f3e51b93c7e","public_key":"KdG6LVTO3-zt7m0fLwjoWc21Kxu2akWjkJ7TjHnA7Sk","state":"active"}],"revision":"915784f0-b489-4a53-b214-f92b8dd05dfb"}}}
$ connectors approvals policy-status --adapter slack-bot
{"ok":true,"result":{"adapter":"slack-bot","policy":null}}
$ connectors approvals policy-set --adapter slack-bot --input-file ~/private/slack-policy.json
{"ok":true,"result":{"adapter":"slack-bot","policy":{"instance_id":"slack-bot","issuer_id":"a78448c8-e165-49d6-9c48-8c75059b154c","operations":["chat.postMessage","conversations.open"],"owner_uid":1000,"policy_id":"f5c48325-633d-497a-a6da-f12b1efd34a9","revision":1,"selection":{"clock_configuration_sha256":"9075017ceec92b29c7ec5d5b4e98364d39dd767ed602dadd2aca6797e5496df9","configuration_revision":"45d968e5336a983cdfa69595911151e63297584ef10169e31214b64f5dce4bee","descriptor_revision":"7c95c126b092dcfb17f60a1fc88fc4d29965e58c132ba740d282206ade542074","executable_selection":"0faf637b88e5f5f047af137d0303688b5352b7b1de341bb58d0d867265949069"}}}}
```

`~/private/slack-policy.json` holds `{"operations":["chat.postMessage","conversations.open"]}`.
The policy records the configuration revision, descriptor revision, executable and clock it was
published under; after any of them changes, publish it again with
`--expected-revision <current>`.

#### Giving an approval

Each write is approved by digest over its exact input, once. `approvals prepare` resolves the
subject without the clock, the provider or the credential. Take `--schema` and `--revision` from
`operations describe --adapter slack-bot --operation chat.postMessage`. The input is the JSON
body under `body`:

```console
$ connectors approvals prepare --adapter slack-bot --connection conn_c4789c790b6b4fdf944637d74f5317d7_3345b2425b5b47efb18a10301c8feba1 --operation chat.postMessage --schema 9788097415fc1581bd915c8a110908821e3ae8b6920057a5260605303141b19b --revision 7c95c126b092dcfb17f60a1fc88fc4d29965e58c132ba740d282206ade542074 --input-file ~/private/post.json
{"ok":true,"result":{"adapter":"slack-bot","preparation":{"approval_policy":{"audience":"connectors.approval/a78448c8-e165-49d6-9c48-8c75059b154c","issuer":"connectors.local-issuer/a78448c8-e165-49d6-9c48-8c75059b154c","max_lifetime_seconds":300,"mode":"required"},"presentation":{"connection":"conn_c4789c790b6b4fdf944637d74f5317d7_3345b2425b5b47efb18a10301c8feba1","descriptor_revision":"7c95c126b092dcfb17f60a1fc88fc4d29965e58c132ba740d282206ade542074","instance":"slack-bot","operation":"chat.postMessage"},"subject":{"approval_mode":"required","authority":{"current_authority":{"id":"connectors.local-policy/f5c48325-633d-497a-a6da-f12b1efd34a9/1","sha256":"5a4479ef13c096a714e66d7f3cd1cbe67ec28a207bb5eb5d367e6cdc407952a8"},"executor":null,"scope":{"caller":"connectors.local-owner/a78448c8-e165-49d6-9c48-8c75059b154c/1000","executor":null,"realm":null,"tenant":null}},"canonicalization":"adapter-v1-canonical-json","format":"connectors.approval-subject/v1","input_sha256":"d2e5f6d34dcd6565d73b5795f797ec5205a091487e659e6503aaff05750b564e","origin":{"authority_ref":"slack-bot","kind":"direct"},"route":null,"target":{"configuration_revision":"45d968e5336a983cdfa69595911151e63297584ef10169e31214b64f5dce4bee","connection":"conn_c4789c790b6b4fdf944637d74f5317d7_3345b2425b5b47efb18a10301c8feba1","connection_revision":"9559e85e-0df7-4808-ad1d-1b74b0d821f1","contract":"operations/v1alpha1","descriptor_revision":"7c95c126b092dcfb17f60a1fc88fc4d29965e58c132ba740d282206ade542074","instance":"slack-bot","operation":"chat.postMessage","profile":"mutation"}},"subject_sha256":"6ad1f21f231cfc1d63eafe7b7f954461f60938ec50e4925c33c96a2a106baedd"}}}
```

`~/private/post.json` holds `{"body":{"channel":"C0000000001","text":"migration check"}}`.
Approving the subject publishes a proof file, which the invocation then spends:

```sh
connectors approvals issue --adapter slack-bot --connection "$connection" \
  --operation chat.postMessage --schema "$schema" --revision "$revision" \
  --input-file ~/private/post.json --approve-subject "$subject_sha256" \
  --proof-output ~/private/proof-post-1.json
connectors operations invoke --adapter slack-bot --connection "$connection" \
  --operation chat.postMessage --schema "$schema" --revision "$revision" \
  --input-file ~/private/post.json --approval-file ~/private/proof-post-1.json \
  --idempotency-key post-1
```

These two were not completed in the run. Issuing needs a fresh clock sample, and the configured
clock source did not answer from the host the run used:

```console
$ connectors approvals clock-check --adapter slack-bot
{"error":{"code":"failure","data":{"code":"unavailable","kind":"operational","next_action":"check_configuration","stage":"observation"}},"ok":false}
$ connectors approvals issue --adapter slack-bot --connection conn_c4789c790b6b4fdf944637d74f5317d7_3345b2425b5b47efb18a10301c8feba1 --operation chat.postMessage --schema 9788097415fc1581bd915c8a110908821e3ae8b6920057a5260605303141b19b --revision 7c95c126b092dcfb17f60a1fc88fc4d29965e58c132ba740d282206ade542074 --input-file ~/private/post.json --approve-subject 6ad1f21f231cfc1d63eafe7b7f954461f60938ec50e4925c33c96a2a106baedd --proof-output ~/private/proof-post-1.json
{"error":{"code":"failure","data":{"code":"unavailable","kind":"operational","next_action":"retry_status","stage":"readiness"}},"ok":false}
```

Run `approvals clock-check` first; issue only once it answers with an observation. The invocation
sends a real message, which [Messages](#messages) describes.

### Retire the credential-only `slack` adapter

The connections of `slack-bot` and `slack-search` do not share a credential with those of `slack`.
A connection belongs to one instance and one adapter id, and each stored credential version belongs
to exactly one connection. Revoking a connection retires only its own versions. Check that before
removing anything.

1. **Prove the survivors.** List them, then revalidate each. Revalidation reads the connection's
   saved credential from custody and runs the identity read, with nothing entered, so `ready` means
   its own custody item is there. A read on `slack-bot` shows the connection in use:

   ```console
   $ connectors connections list --adapter slack-bot | jq -c '[.result.connections[]|{connection,revision,state}]'
   [{"connection":"conn_c4789c790b6b4fdf944637d74f5317d7_3345b2425b5b47efb18a10301c8feba1","revision":"9559e85e-0df7-4808-ad1d-1b74b0d821f1","state":"ready"}]
   $ connectors connections list --adapter slack-search | jq -c '[.result.connections[]|{connection,revision,state}]'
   [{"connection":"conn_c4789c790b6b4fdf944637d74f5317d7_86777214c6be441794c9de42108c2a51","revision":"70c4078a-4ae0-4a01-9a91-06a4f40d205e","state":"pending"}]
   $ connectors connections revalidate --adapter slack-bot --connection conn_c4789c790b6b4fdf944637d74f5317d7_3345b2425b5b47efb18a10301c8feba1 --expected-revision 9559e85e-0df7-4808-ad1d-1b74b0d821f1 | jq -c '.result.connection|{state:.summary.state,revision:.summary.revision,identity:.external_identity}'
   {"state":"ready","revision":"9559e85e-0df7-4808-ad1d-1b74b0d821f1","identity":{"kind":"slack.user","subject":"U0000000001"}}
   $ connectors connections revalidate --adapter slack-search --connection conn_c4789c790b6b4fdf944637d74f5317d7_86777214c6be441794c9de42108c2a51 --expected-revision 70c4078a-4ae0-4a01-9a91-06a4f40d205e | jq -c '.result.connection|{state:.summary.state,revision:.summary.revision,identity:.external_identity}'
   {"state":"ready","revision":"70c4078a-4ae0-4a01-9a91-06a4f40d205e","identity":{"kind":"slack.user","subject":"U0000000001"}}
   $ connectors operations invoke --adapter slack-bot --connection conn_c4789c790b6b4fdf944637d74f5317d7_3345b2425b5b47efb18a10301c8feba1 --operation conversations.list --schema 78615e7dd183198086634e30537945d10a710dbdd26bb420c483716723fae8a0 --revision 7c95c126b092dcfb17f60a1fc88fc4d29965e58c132ba740d282206ade542074 --input-json '{}' | jq -c '.result.result|{status,body}'
   {"status":200,"body":{"channels":[{"id":"C0000000001","name":"general"}],"ok":true,"response_metadata":{"next_cursor":""}}}
   ```

   `pending` is expired validation evidence, not a missing credential (60 seconds unless the profile
   sets `evidence_lifetime_ms`), and revalidation renews it. Stop here if either survivor does not
   end `ready`.

2. **List the old connections** with their revisions:

   ```console
   $ connectors connections list --adapter slack | jq -c '[.result.connections[]|{connection,revision,state}]'
   [{"connection":"conn_c4789c790b6b4fdf944637d74f5317d7_3b913c5d2ecc44ff9b872060a553ecb8","revision":"d7308969-5b4f-4a21-80f2-2abeba0fd669","state":"pending"},{"connection":"conn_c4789c790b6b4fdf944637d74f5317d7_55b6b83832a3435eaff87937ce462f7d","revision":"639ca8a6-6768-4e19-a38b-bc45cf9f39ce","state":"pending"},{"connection":"conn_c4789c790b6b4fdf944637d74f5317d7_ddd028aecf0a4c87ae384a5bfdb8bd92","revision":"239a22a7-b7e5-4b61-ba45-016d2bd6e4d6","state":"pending"}]
   ```

3. **Revoke each one.** Revocation is terminal and local. It asks nothing of Slack
   (`provider_outcome` `not_requested`), so the token stays valid there; revoke it in the Slack
   app if it is to stop working.

   ```console
   $ connectors connections revoke --adapter slack --connection conn_c4789c790b6b4fdf944637d74f5317d7_3b913c5d2ecc44ff9b872060a553ecb8 --expected-revision d7308969-5b4f-4a21-80f2-2abeba0fd669
   {"ok":true,"result":{"adapter":"slack","connection":"conn_c4789c790b6b4fdf944637d74f5317d7_3b913c5d2ecc44ff9b872060a553ecb8","local_state":"revoked","provider_outcome":"not_requested","revision":"1a85150f-7f22-4d31-a64f-01d909ed0081"}}
   $ connectors connections revoke --adapter slack --connection conn_c4789c790b6b4fdf944637d74f5317d7_55b6b83832a3435eaff87937ce462f7d --expected-revision 639ca8a6-6768-4e19-a38b-bc45cf9f39ce
   {"ok":true,"result":{"adapter":"slack","connection":"conn_c4789c790b6b4fdf944637d74f5317d7_55b6b83832a3435eaff87937ce462f7d","local_state":"revoked","provider_outcome":"not_requested","revision":"0a2e3881-cf0e-46fb-b4e6-10066667f520"}}
   $ connectors connections revoke --adapter slack --connection conn_c4789c790b6b4fdf944637d74f5317d7_ddd028aecf0a4c87ae384a5bfdb8bd92 --expected-revision 239a22a7-b7e5-4b61-ba45-016d2bd6e4d6
   {"ok":true,"result":{"adapter":"slack","connection":"conn_c4789c790b6b4fdf944637d74f5317d7_ddd028aecf0a4c87ae384a5bfdb8bd92","local_state":"revoked","provider_outcome":"not_requested","revision":"eeb740a9-d080-400d-b4da-cd5648b2ed9d"}}
   ```

4. **Check again.** The old connections are `revoked`, and the survivors still revalidate to `ready`:

   ```console
   $ connectors connections list --adapter slack | jq -c '[.result.connections[]|{connection,state}]'
   [{"connection":"conn_c4789c790b6b4fdf944637d74f5317d7_3b913c5d2ecc44ff9b872060a553ecb8","state":"revoked"},{"connection":"conn_c4789c790b6b4fdf944637d74f5317d7_55b6b83832a3435eaff87937ce462f7d","state":"revoked"},{"connection":"conn_c4789c790b6b4fdf944637d74f5317d7_ddd028aecf0a4c87ae384a5bfdb8bd92","state":"revoked"}]
   $ connectors connections revalidate --adapter slack-bot --connection conn_c4789c790b6b4fdf944637d74f5317d7_3345b2425b5b47efb18a10301c8feba1 --expected-revision 9559e85e-0df7-4808-ad1d-1b74b0d821f1 | jq -r '.result.connection.summary.state'
   ready
   $ connectors connections revalidate --adapter slack-search --connection conn_c4789c790b6b4fdf944637d74f5317d7_86777214c6be441794c9de42108c2a51 --expected-revision 70c4078a-4ae0-4a01-9a91-06a4f40d205e | jq -r '.result.connection.summary.state'
   ready
   ```

5. **Remove `[adapters.slack]`** and its `.permissions` and `.executable` tables from the
   configuration, then check:

   ```console
   $ connectors setup check | jq -c '[.result.prerequisites[]|{name,state}]'
   [{"name":"configuration","state":"ready"},{"name":"metadata","state":"ready"},{"name":"artifact:slack-bot","state":"ready"},{"name":"artifact:slack-search","state":"ready"},{"name":"keyring","state":"ready"},{"name":"persistent_custody_qualification","state":"ready"}]
   $ connectors adapters list | jq -c '[.result.adapters[]|{adapter,instance_id}]'
   [{"adapter":"slack-bot","instance_id":"slack-bot"},{"adapter":"slack-search","instance_id":"slack-search"}]
   $ connectors connections list --adapter slack
   {"error":{"code":"failure","data":{"code":"not_found","kind":"operational","next_action":"check_configuration","stage":"admission"}},"ok":false}
   $ connectors connections revalidate --adapter slack-bot --connection conn_c4789c790b6b4fdf944637d74f5317d7_3345b2425b5b47efb18a10301c8feba1 --expected-revision 9559e85e-0df7-4808-ad1d-1b74b0d821f1 | jq -r '.result.connection.summary.state'
   ready
   $ connectors connections revalidate --adapter slack-search --connection conn_c4789c790b6b4fdf944637d74f5317d7_86777214c6be441794c9de42108c2a51 --expected-revision 70c4078a-4ae0-4a01-9a91-06a4f40d205e | jq -r '.result.connection.summary.state'
   ready
   $ connectors approvals policy-status --adapter slack-bot | jq -c '.result.policy.operations'
   ["chat.postMessage","conversations.open"]
   ```

Revoke before removing the entry. Once the entry is gone, every connection command answers
`not_found` for `--adapter slack`, as above. A connection left unrevoked then can no longer be
revoked through the CLI, and its metadata and custody item stay behind. Revoked credential versions
are kept for at least 24 hours after retirement before they may be deleted
([retirement](local-connection-registry.md#read-use-and-retirement)). This run does not establish
whether that deletion still happens once the entry is removed.

### `owner_build_mismatch` after a CLI upgrade

The owner is the background `connectors` process that serves a state directory. When the CLI is
upgraded while an owner started by the previous build still runs, every command that would talk to
that owner is refused, `adapters stop` and `connections revalidate` included. Passive reads of the
store such as `connections list` still answer:

```console
$ connectors adapters status --adapter slack-bot
{"error":{"code":"failure","data":{"code":"owner_build_mismatch","kind":"operational","next_action":"stop_owner","stage":"readiness"}},"ok":false}
$ connectors adapters stop --adapter slack-bot --expected-revision 45d968e5336a983cdfa69595911151e63297584ef10169e31214b64f5dce4bee --host-incarnation b1bd440f-6cbb-4895-9387-f7a8702ae714 --child-incarnation 905d552e-87a7-46ae-bdae-a6f31ef7ccf2
{"error":{"code":"failure","data":{"code":"owner_build_mismatch","kind":"operational","next_action":"stop_owner","stage":"readiness"}},"ok":false}
$ connectors connections revalidate --adapter slack-bot --connection conn_c4789c790b6b4fdf944637d74f5317d7_3345b2425b5b47efb18a10301c8feba1 --expected-revision 9559e85e-0df7-4808-ad1d-1b74b0d821f1
{"error":{"code":"failure","data":{"code":"owner_build_mismatch","kind":"operational","next_action":"stop_owner","stage":"readiness"}},"ok":false}
```

`connectors setup owner-stop`, run with the same `--config` and `--state-dir`, asks that owner to
drain and exit and waits up to 30 seconds for it; it answers `"disposition": "stopped"`, or
`"no_owner"` when none runs, and never signals a process
([owner contract](../contracts/cli/v1alpha1/owner.md#stopping-an-owner),
[CLI guide](../website/docs/guides/set-up-the-local-cli.md#after-an-upgrade)). An idle owner also
exits by itself after 10 minutes with no connected client and no work in flight.

The next command that needs an owner starts one from the current build. In this run, made before
`setup owner-stop` existed, the older owner was stopped by its process instead:

```console
$ connectors connections revalidate --adapter slack-bot --connection conn_c4789c790b6b4fdf944637d74f5317d7_3345b2425b5b47efb18a10301c8feba1 --expected-revision 9559e85e-0df7-4808-ad1d-1b74b0d821f1 | jq -r '.result.connection.summary.state'
ready
$ connectors adapters status --adapter slack-bot | jq -r .result.observation.state
ready
```

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/slack.rs`)
  with synthetic ids and messages: the exact request of each read, including
  the `oldest`/`latest` window and the bearer header, with no `token` query
  parameter or header, the returned body bytes, a two-page walk of each list to
  an empty `next_cursor`, an `ok: false` answer returned as `200`, the
  `auth.test` identity read and its refusal, and search on a second connection
  with a user token. The file reads are checked the same way: a two-page
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
- Only these eleven reads and two writes are selected. One user by id or
  e-mail (`users.info`, `users.lookupByEmail`), message edit and delete
  (`chat.update`, `chat.delete`), file upload and delete, reactions and every
  other method of the document are not selected.
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
