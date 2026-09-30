# Gmail through the catalog provider

The catalog provider reads Gmail — the mailbox profile, messages, threads, the
change history and labels — and writes mail through drafts under approval,
from the pinned Gmail API v1 Discovery document. Nothing here is Gmail-specific
code: the Discovery document is projected into OpenAPI, the projection is
compiled into a bundle, a reviewed selection set exposes seven reads and two
draft writes, and the engine described in
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
media-upload paths of six methods are listed there as excluded too, among them
those of `gmail.users.drafts.create` and `gmail.users.drafts.send`; the two
draft writes take a JSON body only. The document's service path is empty, so the projection's one
server is `https://gmail.googleapis.com` and every path carries its own
`/gmail/v1`. The bundle carries all 78 projected operations and names none
unsupported. `adapters/catalog/tests/bundle_drift.rs` regenerates the
projection and its record from the pinned document and the bundle from the
projection, and refuses any of them, or the index, that a fresh run would not
reproduce byte for byte.

## The shipped selection set

[`adapters/catalog/providers/google-gmail/operations.json`](../adapters/catalog/providers/google-gmail/operations.json)
exposes seven reads and two draft writes, and nothing else. The reads are
`effect: read`; `users.drafts.create` and `users.drafts.send` are
`effect: write` (see [Writes](#writes)). No other write of the document —
labels, settings, trash, message import — is selected, and
`users.messages.send` is not selected: mail leaves only as a draft that exists,
can be read, and is sent by id. A selection id is the Discovery method id
without its `gmail.` prefix. `adapters/catalog/tests/google_gmail.rs` pins this
exact id list and each id's Discovery id, path and effect, and that no selection
exposes `gmail.users.messages.send`, so a renamed, dropped or added id, or a
method that moved, fails the gate. The bundle refuses at load any
`operation_id` the projection lacks.

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

## Writes

| id | Discovery id | request | guard |
|---|---|---|---|
| `users.drafts.create` | `gmail.users.drafts.create` | `POST /gmail/v1/users/{userId}/drafts` | none |
| `users.drafts.send` | `gmail.users.drafts.send` | `POST /gmail/v1/users/{userId}/drafts/send` | preflight `users.drafts.get`: the draft's `message.id` must equal the input's `messageId` |

Gmail is written through drafts only: a draft is created, can be read, and is
then sent by id, so the approval to send binds a message the issuer has read.
`users.messages.send` is not selected; it would send a message that never
existed as a draft, bound only by the digest of its own input.

They run on a separate write instance with the compose scope; see
[Authentication](#authentication). Both are required-approval mutations, like
every catalog write: select `private_protocol = "connectors-private/2"`, permit
them in the adapter's operation permissions, name them in the approval policy,
and invoke each with a proof issued for its exact input, as the
[guarded merge guide](local-gitlab-merge.md) walks through for GitLab. The
approval subject carries the digest of the whole input, `body` included, so a
proof issued for one input is refused for any other. What the issuer is shown
is the instance, operation, connection and descriptor revision, not the input
(`crates/connectors-host/src/local/owner/approval_issuance.rs`): read the input
file, and the message it names, before approving. A write offered without an
approval is refused before the provider sends anything.

- **`users.drafts.create`** takes `userId` (`me`) and a `body` whose
  `message.raw` is the whole message: RFC 5322 text (the pinned document says
  RFC 2822), encoded base64url. It sends nothing. The headers inside that
  message are what a later send uses: `To`, `Cc` and `Bcc` are its recipients,
  `Subject` and `From` what they see. `body.message.threadId` files the draft
  in an existing thread, which the pinned document says also needs matching
  `References` and `In-Reply-To` headers and the same `Subject`. A create is not
  idempotent: the same input twice makes a second draft. It carries no guard:
  there is nothing to compare before a draft exists. The answer is the draft:
  its `id` and its `message.id`, the two values a send pins.

  ```json
  {"userId": "me", "body": {"message": {"raw": "<base64url RFC 5322 message>"}}}
  ```

  To approve it, decode `body.message.raw` from the input file and read the
  headers and body: the approval binds exactly those bytes.
- **`users.drafts.send`** sends an existing draft to every recipient in its
  `To`, `Cc` and `Bcc` headers. A sent message cannot be recalled. It takes
  `userId`, the draft id as `body.id`, and `messageId`, the draft's
  `message.id` the issuer read:

  ```json
  {"userId": "me", "messageId": "<the draft's message.id>", "body": {"id": "<draft id>"}}
  ```

  `messageId` is compared, never sent: the POST body is `body` as given. Read
  the message before approving: `users.messages.get` with that `messageId` and
  `{"format": "raw"}` on the read instance returns it as one base64url string
  in `raw`. The pinned document calls a message id immutable, so a draft whose
  content changed carries a different `message.id` under the same draft id.
  **A changed draft sends nothing.** The provider reads the draft once with
  `users.drafts.get`; when its `message.id` differs from `messageId`, the send
  is refused with no POST. An input without `messageId` or without `body.id` is
  refused before any Gmail request, and a draft that no longer exists is refused
  after its read (`guard target was not found before dispatch`).
- **The send accepts only `{"id": "<draft id>"}` as its `body`.** Google's
  drafts guide says a `body.message` given to a send replaces the draft's
  content before it is sent, which the preflight, comparing the stored draft,
  would not see. The selection closes the body with `body_keys: ["id"]` (see
  [the catalog provider guide](local-catalog-provider.md)): its declared input
  schema admits `id` and nothing else, and a `body.message`, or any other body
  key, is refused as `invalid_input` before any request.
- The preflight and the POST are two requests, and Gmail's send takes no
  precondition: a draft edited between them is sent as edited. Nothing is
  compared after dispatch, because the send answers with the sent message,
  and whether that keeps the draft's `message.id` is neither documented nor
  observed. A `2xx` is reported applied.
- The approval input is capped at 256 KiB (`TARGET_LIMIT` in
  `approval_issuance.rs`), so a draft whose encoded message makes the input
  larger cannot be created under approval. The preflight reads the whole draft,
  so it is also subject to the 4 MiB response limit.

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
consent. `authorize_url` is `https://accounts.google.com/o/oauth2/auth`, the
`auth_uri` Google writes into a downloaded client file, which the client-file
connect compares byte for byte.

The read-only scope does not cover the writes. The pinned document accepts
`https://www.googleapis.com/auth/gmail.compose` for both writes, and not
`gmail.readonly`. It accepts both scopes for `users.drafts.get`, which the
send's preflight reads, so the compose scope alone covers the whole send.
Of the seven reads the compose scope covers only
`users.getProfile`, so the write instance does not read messages, threads,
history or labels; read with the read instance. At
Google the compose scope also permits `users.messages.send`; this provider
does not expose it. With the write configuration, a stored refresh token that
was granted only the read-only scope fails validation as insufficient scope.

Writes use a separate instance. Configure a second instance id,
`google-gmail-write`: a copy of the configuration below with these values
changed, as its own adapter entry in the host configuration with the writes in
its operation permissions and `private_protocol = "connectors-private/2"`:

```json
{
  "instance": "google-gmail-write",
  "auth": {
    "minimum_scopes": ["https://www.googleapis.com/auth/gmail.compose"],
    "requested_scopes": ["openid", "https://www.googleapis.com/auth/gmail.compose"]
  }
}
```

Then `connections connect` that instance with the Google client file. An
existing read-only connection cannot be widened in place:
`connections repair` cannot add a scope, because the configuration revision and
the profile, whose `minimum_scopes` the scope changes, are part of the
connection binding. Changing the scopes of the read instance itself leaves its
connection bound to the old configuration, and a new connection on that same
instance is refused while the old one exists.

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
    "authorize_url": "https://accounts.google.com/o/oauth2/auth",
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
- The writes are verified against the same fixture on the guide's write
  instance through the host's prepare/commit exchange: the send's preflight
  read, a changed draft, a missing `messageId` or `body.id`, a send body with
  any key but `id` and a missing draft refused with no POST, and the exact POST
  body of each write. The approval
  binding is verified with the host's approval signer and verifier against a
  subject built from the provider's descriptor, not through the CLI and owner.
  The write instance's scope check and its separate acquisition are verified
  through the provider's bootstrap and the host's connection registry. No live
  draft has been created or sent. That a changed draft carries a new
  `message.id` is inferred from the pinned document calling message ids
  immutable, and that `users.messages.get` reads a draft's message from Gmail
  keeping drafts as messages labelled `DRAFT`; neither has been observed live.
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
