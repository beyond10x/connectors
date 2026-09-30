# Google Drive through the catalog provider

The catalog provider reads Google Drive — the signed-in user, file metadata,
exported document text and the change feed — and creates, changes and copies
file metadata, from the pinned Google Drive API v3 Discovery document. Nothing
here is Drive-specific code: the Discovery document is projected into OpenAPI,
the projection is compiled into a bundle, a reviewed selection set exposes six
reads and three guarded metadata writes ([Writes](#writes)), and the engine described in
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
exposes six reads, each `effect: read`, and three writes, each `effect: write`
([Writes](#writes)).
A selection id is the Discovery method id without its `drive.` prefix.
`adapters/catalog/tests/google_drive.rs` pins this exact id list, each id's
effect, Discovery id, method and path, and the update's guard, so a renamed or
dropped id, or a method that moved, fails the gate. The bundle refuses at load any `operation_id` the projection lacks.

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
  required, so the engine does not enforce it; a read without it is sent and
  Drive refuses it.
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
  returned as `null` today, not `""`; returning `""` is
  `story:catalog-engine-provider-refusal-shapes`. Use a text `mimeType`: bytes that are not UTF-8 are refused as
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

## Writes

Three writes change Drive metadata. Each is a required-approval mutation: it
runs only through the host's approval, audit and mutation coordinator, needs
`private_protocol = "connectors-private/2"` and an approval policy naming it,
and is prepared, approved and invoked as
[the guarded merge guide](local-gitlab-merge.md) describes. The approval binds
the whole input — every `body` member, the target `fileId`, the pinned `version`
and every query parameter — so a proof issued for one body is refused for any
other. Without a proof the write is refused before the provider is asked
anything. Writes need the write scope; see [Authentication](#authentication).

| id | Discovery id | request | guard |
|---|---|---|---|
| `files.create` | `drive.files.create` | `POST /drive/v3/files` with a metadata `body` | none; the approval binds the whole body |
| `files.update` | `drive.files.update` | `PATCH /drive/v3/files/{fileId}` with a metadata `body` | preflight `files.get`: `/version` equals the input `version` |
| `files.copy` | `drive.files.copy` | `POST /drive/v3/files/{fileId}/copy` with a metadata `body` | none; the approval binds the whole body |

- **Metadata only.** The projection excludes every media upload path and the
  `uploadType` parameter (`crates/connectors-catalog/src/discovery.rs`), so no
  write carries file content. `files.create` makes a Google Workspace file (a
  `mimeType` such as `application/vnd.google-apps.document`), a folder
  (`application/vnd.google-apps.folder`) or a shortcut
  (`application/vnd.google-apps.shortcut` with `shortcutDetails.targetId`). The
  `body` is Drive's File resource, passed through as supplied and checked only
  by Drive.
- **Create and copy carry no guard.** Nothing exists to compare before a create,
  and a guard compares a value read before the write for equality only. The
  approval, bound to the whole body, is their only check; a create or copy made
  again under a new approval makes another file.
- **`files.update` is pinned to `version`.** Its input is `fileId`, `version`
  (the file's current version, from `files.get` with `fields` including
  `version`), `fields` and `body`. Before the PATCH the provider sends one
  `files.get` for `fileId` with the same `fields`, and refuses before any write
  unless the answer's `version` equals the input `version`; the attempt is then
  `not_attempted`. Drive returns `version` only when `fields` asks for it, so the
  guard reads `fields` from the input: an update without `fields` is refused as
  `invalid_input` before any request, and one whose `fields` leaves out
  `version` sends the preflight, gets no `version` back and writes nothing. Use
  `fields` such as `id,name,version`: the same `fields` selects the PATCH's
  answer, which then carries the new version to pin next.
- **Race boundary.** Drive offers no precondition on `files.update`, so the pin
  is checked in preflight only: a change between the preflight and the PATCH is
  not detected, and the guard compares nothing afterwards. This is the accepted
  boundary described under `guard` in
  [the catalog provider guide](local-catalog-provider.md), which also gives the
  `refused`, `applied` and `unknown` classification of the answer.

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

### The write scope

The writes need `https://www.googleapis.com/auth/drive.file`: access to the
files this OAuth client created or that the user opened with it, and to no
other file. The read scope stays, so the reads and each update's preflight still
see every file; Drive refuses a write to a file outside `drive.file`. The write
configuration is the one above with `drive.file` added to both
`minimum_scopes` and `requested_scopes`:

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
    "minimum_scopes": ["https://www.googleapis.com/auth/drive.readonly", "https://www.googleapis.com/auth/drive.file"],
    "token_url": "https://oauth2.googleapis.com/token",
    "authorize_url": "https://accounts.google.com/o/oauth2/v2/auth",
    "requested_scopes": ["openid", "https://www.googleapis.com/auth/drive.readonly", "https://www.googleapis.com/auth/drive.file"]
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/google-drive/operations.json"
}
```

Validation checks the granted scopes against `minimum_scopes`, so an entry
consented for the read scope alone is refused as insufficient scope under this
configuration. The configuration change also changes the bootstrap's
`configuration_revision`; take the new one from `--print-local-bootstrap` for
the adapter entry. To add the write scope to an existing connection, replace
its entry with one consented for both scopes through `connections repair`: the
same Google account keeps the connection's identity, and a repair whose entry
still lacks `drive.file` fails and leaves the prior credential selected
([the connection registry](local-connection-registry.md)).

## Limits

- Verified against a local HTTPS fixture only
  (`adapters/catalog/tests/google_drive.rs`): the guide's configuration with the
  fixture as API and token host, one token exchange, the exact request of each
  read with the exchanged bearer, the returned body as JSON (the export as
  text), a two-page walk of `files.list` to a page without `nextPageToken` and of
  `changes.list` to `newStartPageToken`, and the `pageSize` bounds. No live Drive
  account has been read; live evidence belongs to
  `story:catalog-google-live-deck-read`.
- The writes are verified against the same fixture through a private protocol
  two child, not through the CLI: the exact request and body of each write, the
  update's one preflight, a stale `version` refused with no PATCH sent, an
  update without `version` or `fields` refused with no request, the read
  transport refusing every write id, and validation refusing a grant without
  `drive.file`. That an approval for one input is refused for another is checked
  on the host's approval verification with each write's input, not through a
  CLI journey. No live Drive file has been written.
- The engine parses and re-serialises a JSON body, so it is returned as equal
  JSON, not as Drive's exact bytes.
- The provider does not walk pages itself and does not retry on `429`; a
  rate-limited read is returned as a refusal.
- Drive also signals an exceeded quota as a `403` whose `error.errors[].domain`
  is `usageLimits` and whose `reason` is `userRateLimitExceeded` or
  `rateLimitExceeded`. Today every `403` reaches the caller as `forbidden`, so
  a quota refusal cannot be told apart from a permission denial by its code.
  Reporting it as `rate_limited` is
  `story:catalog-engine-provider-refusal-shapes`.
