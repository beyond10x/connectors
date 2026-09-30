# Google Drive through the catalog provider

The catalog provider reads Google Drive — the signed-in user, file metadata,
exported document text and the change feed — from the pinned Google Drive API v3
Discovery document. Nothing here is Drive-specific code: the Discovery document
is projected into OpenAPI, the projection is compiled into a bundle, a reviewed
selection set exposes six reads, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them.
Configuration, connection, approval and invocation work as described there; this
page covers what differs for Drive. Authentication is the `oauth2_refresh`
profile `google.oauth`; see [Authentication](#authentication).

## Source and bundle

The pinned source is
[`adapters/google/upstream/drive/drive-api.json`](../adapters/google/upstream/drive/README.md),
SHA-256 `50a01d9b5134f16b4782098bcc6694aa4edf810993432022aba49237b5c83048`,
Discovery `revision` `20260916`. A Discovery document is not OpenAPI, so it is
projected first, under the rule table of `crates/connectors-catalog/src/discovery.rs`,
and the bundle is compiled from the projection with the derivation recorded:

```sh
cargo run --locked -p connectors-build -- discovery \
  --source adapters/google/upstream/drive/drive-api.json \
  --out adapters/google/generated/drive.openapi.json
cargo run --locked -p connectors-build -- catalog \
  --provider google-drive \
  --source adapters/google/generated/drive.openapi.json \
  --derived-from adapters/google/upstream/drive/drive-api.json \
  --directory adapters/catalog/generated/bundles \
  --auth-profile google.oauth \
  --replace
```

`--replace` is needed because the committed index already carries
`google-drive`; without it the second command is refused with "provider is
already indexed". The first command writes `drive.openapi.json` and its projection record
`drive.openapi.projection.json`: all 64 methods of the document are projected
and none is excluded. The projection's one server is
`https://www.googleapis.com/drive/v3`, and the bundle records each path below it,
so Discovery's `files` is recorded as `/drive/v3/files`. The bundle carries all
64 operations and names none unsupported. `adapters/catalog/tests/bundle_drift.rs`
regenerates the projection and its record from the pinned document and the
bundle from the projection, and refuses any of them, or the index, that a fresh
run would not reproduce byte for byte.

## The shipped selection set

[`adapters/catalog/providers/google-drive/operations.json`](../adapters/catalog/providers/google-drive/operations.json)
exposes six reads and nothing else. Each is `effect: read`; there are no writes.
A selection id is the Discovery method id without its `drive.` prefix.
`adapters/catalog/tests/google_drive.rs` pins this exact id list and each id's
Discovery id and path, so a renamed or dropped id, or a method that moved, fails
the gate. The bundle refuses at load any `operation_id` the projection lacks.

Every list returns one page per call. The provider returns `status`, `body` and
`provenance`; `body` is Drive's answer unchanged, and the fields that decide the
end of a walk are in it.

| id | Discovery id | request | paging parameters | end condition | time filter or deltas |
|---|---|---|---|---|---|
| `about.get` | `drive.about.get` | `GET /drive/v3/about` with `fields` | single item | n/a | none |
| `files.list` | `drive.files.list` | `GET /drive/v3/files` with `q` | `pageToken`, `pageSize` | `nextPageToken` absent | a `q` term such as `modifiedTime > '<RFC 3339>'`; the change feed is `changes.list` |
| `files.get` | `drive.files.get` | `GET /drive/v3/files/{fileId}` | single item | n/a | none; deltas come from `changes.list` |
| `files.export` | `drive.files.export` | `GET /drive/v3/files/{fileId}/export` with `mimeType` (required) | single item | n/a | none; deltas come from `changes.list` |
| `changes.getStartPageToken` | `drive.changes.getStartPageToken` | `GET /drive/v3/changes/startPageToken` | single item | n/a | returns `startPageToken`, the first baseline |
| `changes.list` | `drive.changes.list` | `GET /drive/v3/changes` with `pageToken` (required) | `pageToken`, `pageSize` | `newStartPageToken` present | from the baseline in `pageToken` to `newStartPageToken`, the next baseline |

- **`about.get`.** Drive answers this method only when `fields` names what to
  return, for example `user,storageQuota`. The projection does not mark `fields`
  required, so the selection does (`"required": ["fields"]`): the declared input
  schema requires it, and a read without it is refused as `invalid_input`
  before any request.
- **Following a page of `files.list`.** Send the first page without `pageToken`.
  For the next page, send the previous page's `nextPageToken` as `pageToken`,
  with the other parameters unchanged. A page without `nextPageToken` is the
  last. `q` is Drive's search query (for example `trashed = false`).
- **`fields` and the end conditions.** `fields` selects the parts of the answer
  Drive returns, and a token it leaves out is not returned. The end conditions
  above and below hold only when `fields`, if given, includes the tokens: include
  `nextPageToken` for `files.list` (for example
  `nextPageToken,files(id,name)`), and both `nextPageToken` and
  `newStartPageToken` for `changes.list` (for example
  `nextPageToken,newStartPageToken,changes`). Without them the first page looks
  like the last, and a delta walk ends with no new baseline.
- **`files.get`** reads metadata only. The media download (`alt=media`) is not
  projected, so `alt` is refused before any request; use `files.export` for the
  text of a Google Workspace document.
- **`files.export`** takes the file's `fileId` and the target `mimeType`, such as
  `text/plain`. The projection declares its answer `application/octet-stream`,
  so the selection declares `response: text` and the body is returned as a JSON
  string. An empty export (for example an empty spreadsheet as `text/csv`) is
  returned as `""`. Use a text `mimeType`: bytes that are not UTF-8 are refused as
  `upstream_protocol`. The response limit applies: an answer over 4 MiB
  (`connectors_core::RESPONSE_LIMIT`) is refused as `capacity`.
- **Deltas.** Read a baseline once with `changes.getStartPageToken` and keep its
  `startPageToken`. To take the changes since then, send it as `pageToken` to
  `changes.list`; while a page carries `nextPageToken`, send that as the next
  `pageToken`. The page that carries `newStartPageToken` is the last; keep that
  token as the baseline for the next walk. `pageToken` is required, so a
  `changes.list` without it is refused before any request.
- **`pageSize`.** The pinned document states a minimum of 1 and a maximum of 1000
  for both lists; the selection set bounds `pageSize` to that range, so 0, 1001
  or a value that is not an integer is refused as `invalid_input` before any
  request.

Every other query parameter the projection declares for an operation is accepted
by name, including the document-wide `fields`; one it does not declare is
refused before any request.

## Authentication

Drive uses Google OAuth: the `oauth2_refresh` scheme under the profile
`google.oauth`, as described in
[the catalog provider guide](local-catalog-provider.md). The protected entry is
`{"client_id":"...","client_secret":"...","refresh_token":"..."}` for an
installed-app OAuth client. The provider exchanges it at `token_url` for an
access token and sends that as `Authorization: Bearer <access token>`. The
identity comes from the token answer's `id_token` (`identity.source: id_token`),
and `minimum_scopes` asks for the Drive read-only scope. `authorize_url` and
`requested_scopes` are never called by the provider; they are handed to the host
for obtaining the entry by consent.

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "google-drive",
  "provider": "google-drive",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://www.googleapis.com/drive/v3",
  "auth": {
    "profile": "google.oauth",
    "scheme": "oauth2_refresh",
    "header": "Authorization",
    "bearer": true,
    "label": "Google refresh token",
    "identity": {"source": "id_token", "kind": "google.user"},
    "minimum_scopes": ["https://www.googleapis.com/auth/drive.readonly"],
    "token_url": "https://oauth2.googleapis.com/token",
    "authorize_url": "https://accounts.google.com/o/oauth2/v2/auth",
    "requested_scopes": ["openid", "https://www.googleapis.com/auth/drive.readonly"]
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/google-drive/operations.json"
}
```

## Limits

- Verified against a local HTTPS fixture only
  (`adapters/catalog/tests/google_drive.rs`): the guide's configuration with the
  fixture as API and token host, one token exchange, the exact request of each
  read with the exchanged bearer, the returned body as JSON (the export as
  text), a two-page walk of `files.list` to a page without `nextPageToken` and of
  `changes.list` to `newStartPageToken`, and the `pageSize` bounds. No live Drive
  account has been read; live evidence belongs to
  `story:catalog-google-live-deck-read`.
- The engine parses and re-serialises a JSON body, so it is returned as equal
  JSON, not as Drive's exact bytes.
- The provider does not walk pages itself and does not retry on `429`; a
  rate-limited read is returned as a refusal.
- Drive also signals an exceeded quota as a `403` whose `error.errors[].domain`
  is `usageLimits` and whose `reason` is `userRateLimitExceeded`,
  `rateLimitExceeded` or, for an exhausted daily project quota,
  `dailyLimitExceeded`. Every selection names all three in
  `rate_limit_reasons`, so
  such a `403` reaches the caller as `rate_limited`, like a `429`; every other
  `403` is `forbidden`.
- Declaring `required` on `about.get` changed its declared input schema, and
  so the descriptor revision; an approval policy bound to the earlier revision
  must be issued again. Adding `rate_limit_reasons` changed the selection set,
  and so the configuration revision of a Drive instance.
