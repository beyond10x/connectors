# Slack through the catalog provider

The catalog provider reads Slack conversations — the channels a bot token can
see, a channel's messages and one thread — and the workspace's users from the
pinned Slack Web API document. Nothing here is Slack-specific code: the
Swagger 2.0 document is projected into OpenAPI 3.1.0, the projection is
compiled into a bundle, a reviewed selection set exposes four reads, and the
engine described in
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
  --replace
```

`--replace` is needed because the committed index already carries `slack`;
without it the second command is refused with "provider is already indexed".
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
exposes four reads, each `effect: read`, and nothing else; there are no
writes. A selection id is the Slack method name. `adapters/catalog/tests/slack.rs`
pins this exact id list and each id's `operationId` and path in the pinned
document, so a renamed or dropped id, or a source operation that moved, fails
the gate. The bundle refuses at load any `operation_id` the projection lacks.

Every list returns one page per call. The provider returns `status`, `body` and
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
  the credential as an optional `token` query parameter. The selections
  withhold it, so the token travels only in the `Authorization` header and an
  input carrying `token` is refused before any request. Where the document
  marks `token` required, the method cannot be selected
  ([below](#methods-that-cannot-be-selected-yet)).

Every other query parameter the projection declares for an operation is
accepted by name; one it does not declare is refused before any request.

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
and `users:read` for `users.list`.
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

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/slack.rs`)
  with synthetic ids and messages: the exact request of each read, including
  the `oldest`/`latest` window and the bearer header, the returned body bytes,
  a two-page walk of each list to an empty `next_cursor`, an `ok: false`
  answer returned as `200`, the `auth.test` identity read and its refusal, and
  the load refusal of the four methods whose `token` is required. No live
  Slack workspace has been called.
- The engine parses and re-serialises the body. The fixture's bodies are
  compact JSON with sorted keys, and the test asserts the returned body is
  those exact bytes; a Slack body with other spacing or key order is returned
  as equal JSON, not as its exact bytes.
- The provider does not walk pages itself, and does not bound `limit` on the
  message reads or `users.list`: the pinned document states no maximum for
  them.
- Only these four reads are selected. `auth.test`, `team.info`, `emoji.list`
  and `search.messages` cannot be selected yet
  ([below](#methods-that-cannot-be-selected-yet)); message writes, files,
  reactions and every other method of the document are not selected.

## Methods that cannot be selected yet

Four read methods of the pinned document cannot be selected without a change
to the catalog engine (`adapters/catalog/src/lib.rs`). They declare the
credential as a **required** parameter:

| method | pinned `operationId` | `token` in the pinned document | needed for |
|---|---|---|---|
| `auth.test` | `auth_test` | required header | the token's identity and workspace as an operation (`slack.info`) |
| `team.info` | `team_info` | required query parameter, scope `team:read` | workspace name, domain and id (`slack.info`) |
| `emoji.list` | `emoji_list` | required query parameter, scope `emoji:read` | the workspace's custom emoji |
| `search.messages` | `search_messages` | required query parameter, scope `search:read`, user token | message search |

The engine withholds only a parameter nothing requires: withholding `token`
from these selections is refused at load ("withholds `token`, which is
required"), and not withholding it would declare the bot token as a required
input sent in the query, so none is shipped. `auth.test` is refused earlier, as
needing a header parameter the transport does not carry.
`adapters/catalog/tests/slack.rs` pins both refusals and that none of the four is
shipped, so selecting one fails that test; change it and this section together.

`auth.test` still runs as the connection's identity read:
`connections revalidate` repeats it, and the saved connection records its
`user_id`. It is not an operation whose body a caller reads.

The change that would admit them is one selection field, with no change to
the local host or any shared contract: a list of the parameters through which
the pinned document passes the credential the connection already sends in its
`Authorization` header. Each named parameter must be declared by the
operation, in the query or a header, and must not be bounded, marked
`required`, withheld or read by a guard. It is removed from the operation
whether or not the document requires it, and the removal runs before the
required-header refusal, so `auth.test` loads as well. `withhold` keeps its
meaning.

Search then needs no host change either. A catalog connection holds one
profile, so search is a second Slack connection: the same bundle, profile
`slack.user`, a user token (`xoxp-…`) with `search:read`, the same `auth.test`
identity read (whose `user_id` is then the person's id), and an operations file
selecting `search.messages` alone, so the bot connection never exposes it.
Nothing in the local configuration ties a connection's profile to the
bundle's recorded `auth_profile`. Its paging is by `page` and `count` (at most
100 per the pinned document), not by cursor.
