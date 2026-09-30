# Gmail through the catalog provider

The catalog provider reads Gmail — the mailbox profile, messages, threads, the
change history and labels — from the pinned Gmail API v1 Discovery document.
Nothing here is Gmail-specific code: the Discovery document is projected into
OpenAPI, the projection is compiled into a bundle, a reviewed selection set
exposes seven reads, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them.
Configuration, connection, approval and invocation work as described there; this
page covers what differs for Gmail. Authentication is the `oauth2_refresh`
profile `google.oauth`, as for [Google Drive](catalog-google-drive.md); see
[Authentication](#authentication).

## Source and bundle

The pinned source is
[`adapters/google/upstream/gmail/gmail-api.json`](../adapters/google/upstream/gmail/README.md),
SHA-256 `7591d33ec87e4ef83551f71630213a89ac349becebe9bd9b3e4ce3851dede0fe`,
Discovery `revision` `20260727`. It is projected under the rule table of
`crates/connectors-catalog/src/discovery.rs`, and the bundle is compiled from
the projection with the derivation recorded:

```sh
cargo run --locked -p connectors-build -- discovery \
  --source adapters/google/upstream/gmail/gmail-api.json \
  --out adapters/google/generated/gmail.openapi.json
cargo run --locked -p connectors-build -- catalog \
  --provider google-gmail \
  --source adapters/google/generated/gmail.openapi.json \
  --derived-from adapters/google/upstream/gmail/gmail-api.json \
  --directory adapters/catalog/generated/bundles \
  --auth-profile google.oauth \
  --replace
```

`--replace` is needed because the committed index already carries
`google-gmail`; without it the second command is refused with "provider is
already indexed". The first command writes `gmail.openapi.json` and its
projection record `gmail.openapi.projection.json`. Of the document's 79 methods,
78 are projected. One is excluded and listed in the record with its reason:
`gmail.users.settings.cse.identities.patch`, whose path differs from another
method's only in a parameter name, which OpenAPI 3.0.3 does not admit. The
media-upload paths of six methods are listed there as excluded too; none of
them is a read. The document's service path is empty, so the projection's one
server is `https://gmail.googleapis.com` and every path carries its own
`/gmail/v1`. The bundle carries all 78 projected operations and names none
unsupported. `adapters/catalog/tests/bundle_drift.rs` regenerates the
projection and its record from the pinned document and the bundle from the
projection, and refuses any of them, or the index, that a fresh run would not
reproduce byte for byte.

## The shipped selection set

[`adapters/catalog/providers/google-gmail/operations.json`](../adapters/catalog/providers/google-gmail/operations.json)
exposes seven reads and nothing else. Each is `effect: read`; none of the
document's writes (sending, drafts, labels, settings, trash) is selected. A
selection id is the Discovery method id without its `gmail.` prefix.
`adapters/catalog/tests/google_gmail.rs` pins this exact id list and each id's
Discovery id and path, so a renamed or dropped id, or a method that moved, fails
the gate. The bundle refuses at load any `operation_id` the projection lacks.

Every list returns one page per call. The provider returns `status`, `body` and
`provenance`; `body` is Gmail's answer unchanged, and the fields that decide the
end of a walk are in it.

| id | Discovery id | request | paging parameters | end condition | time filter or deltas |
|---|---|---|---|---|---|
| `users.getProfile` | `gmail.users.getProfile` | `GET /gmail/v1/users/{userId}/profile` | single item | n/a | returns `historyId`, the first baseline |
| `users.messages.list` | `gmail.users.messages.list` | `GET /gmail/v1/users/{userId}/messages` | `pageToken`, `maxResults` (1–500) | `nextPageToken` absent | a `q` term such as `newer_than:7d`; deltas come from `users.history.list` |
| `users.messages.get` | `gmail.users.messages.get` | `GET /gmail/v1/users/{userId}/messages/{id}` | single item | n/a | none; deltas come from `users.history.list` |
| `users.threads.list` | `gmail.users.threads.list` | `GET /gmail/v1/users/{userId}/threads` | `pageToken`, `maxResults` (1–500) | `nextPageToken` absent | a `q` term such as `newer_than:7d`; deltas come from `users.history.list` |
| `users.threads.get` | `gmail.users.threads.get` | `GET /gmail/v1/users/{userId}/threads/{id}` | single item | n/a | none; deltas come from `users.history.list` |
| `users.history.list` | `gmail.users.history.list` | `GET /gmail/v1/users/{userId}/history` with `startHistoryId` (required) | `pageToken`, `maxResults` (1–500) | `nextPageToken` absent; that page's `historyId` is the next baseline | from `startHistoryId` to the last page's `historyId` |
| `users.labels.list` | `gmail.users.labels.list` | `GET /gmail/v1/users/{userId}/labels` | single item | n/a | none |

- **`userId`** is required on every read and is `me`: the pinned document names
  it "the user's email address" and `me` "the authenticated user". The
  document's default of `me` is not applied by the engine, so send it.
- **Following a page.** Send the first page without `pageToken`. For the next
  page, send the previous page's `nextPageToken` as `pageToken`, with the other
  parameters unchanged. A page without `nextPageToken` is the last. Treat a page
  without `messages` or `threads` as empty.
- **Search and labels.** `q` is Gmail's search query (for example
  `newer_than:7d` or `from:someone@example.com`). `labelIds` is repeated: send
  an array, such as `["INBOX", "UNREAD"]`, and each element is sent as its own
  `labelIds` pair in the order given; a message must carry every label named.
  Label ids come from `users.labels.list`. `includeSpamTrash` is accepted as
  the document declares it.
- **`format`.** `users.messages.get` takes `full` (the default), `metadata`,
  `minimal` or `raw`; `users.threads.get` takes `full`, `metadata` or
  `minimal`. `metadata` returns ids, labels and headers, narrowed by `metadataHeaders`,
  which is repeated like `labelIds`. `raw` returns the whole message as one
  base64url string in `raw`. The engine does not check a value against these
  lists: another value is sent, and Gmail answers it.
- **`maxResults`.** The pinned document declares no bound for the three lists;
  each description says the maximum allowed value is 500. The selections bound
  `maxResults` to 1–500, so 0, 501 or a value that is not an integer is refused
  as `invalid_input` before any request.

Every other query parameter the projection declares for an operation is accepted
by name, including the document-wide `fields`; one it does not declare is
refused before any request.

## Deltas by `historyId`

Take a baseline once: `users.getProfile` returns the mailbox's current
`historyId` (a message or thread's own `historyId` works as well). To read what
changed since, send it as `startHistoryId` to `users.history.list` and walk the
pages; each record names the messages added, deleted or relabelled. The page
without `nextPageToken` is the last; keep its `historyId` as the baseline for
the next walk. `historyTypes` (repeated) narrows the records to
`messageAdded`, `messageDeleted`, `labelAdded` or `labelRemoved`, and `labelId`
to one label.

The pinned document says `startHistoryId` is required although it does not mark
it so; the selection declares it required, so a `users.history.list` without it
is refused as `invalid_input` before any request.

**Reset rule.** A `historyId` is typically valid for at least a week, and in
rare cases for only a few hours. An out-of-date or invalid `startHistoryId` is
answered `404`, which reaches the caller as `not_found`. On that refusal, do a
full sync: walk `users.messages.list` (or `users.threads.list`) from the first
page, read what is needed, and take a new baseline from `users.getProfile`.

## Authentication

Gmail uses Google OAuth: the `oauth2_refresh` scheme under the profile
`google.oauth`, as described in
[the catalog provider guide](local-catalog-provider.md). The protected entry is
`{"client_id":"...","client_secret":"...","refresh_token":"..."}` for an
installed-app OAuth client. The provider exchanges it at `token_url` for an
access token and sends that as `Authorization: Bearer <access token>`. The
identity comes from the token answer's `id_token` (`identity.source: id_token`).
`minimum_scopes` asks for the Gmail read-only scope, which the pinned document
accepts for all seven reads. `authorize_url` and `requested_scopes` are never
called by the provider; they are handed to the host for obtaining the entry by
consent.

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "google-gmail",
  "provider": "google-gmail",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://gmail.googleapis.com",
  "auth": {
    "profile": "google.oauth",
    "scheme": "oauth2_refresh",
    "header": "Authorization",
    "bearer": true,
    "label": "Google refresh token",
    "identity": {"source": "id_token", "kind": "google.user"},
    "minimum_scopes": ["https://www.googleapis.com/auth/gmail.readonly"],
    "token_url": "https://oauth2.googleapis.com/token",
    "authorize_url": "https://accounts.google.com/o/oauth2/v2/auth",
    "requested_scopes": ["openid", "https://www.googleapis.com/auth/gmail.readonly"]
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/google-gmail/operations.json"
}
```

## Limits

- Verified against a local HTTPS fixture only
  (`adapters/catalog/tests/google_gmail.rs`): the guide's configuration with the
  fixture as API and token host, one token exchange, the exact request of each
  read with the exchanged bearer, the returned body as JSON, a two-page walk of
  `users.messages.list` and `users.threads.list` to a page without
  `nextPageToken` and of `users.history.list` from the profile's `historyId`,
  an out-of-date `startHistoryId`'s `404` as `not_found`, `labelIds` and
  `metadataHeaders` sent as repeated pairs, each message `format`, and the
  `maxResults` bounds of the three lists. No live mailbox has been read.
- The response limit applies to every read: an answer over 4 MiB
  (`connectors_core::RESPONSE_LIMIT`) is refused as `capacity`, not truncated.
  A large message read with `raw` or `full`, or a long thread, can exceed it;
  `metadata` or `fields` narrows the answer.
- The engine parses and re-serialises the body, so it is returned as equal JSON,
  not as Gmail's exact bytes.
- The provider does not walk pages itself and does not retry. A `429`, or a
  `403` whose reason is `rateLimitExceeded` or `userRateLimitExceeded` (each
  selection names both in `rate_limit_reasons`), is returned as `rate_limited`;
  every other `403` is `forbidden`.
