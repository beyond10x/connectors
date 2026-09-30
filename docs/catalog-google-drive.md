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

Three writes create, change and copy Drive files through their metadata. None
uploads content, but a metadata write is not harmless: one `files.update` can
also move a file, trash it, lock it or remove the access it inherits, and a
create or copy can set who may share, copy or download the result (see the
table). The table's last column names the effects most likely to matter; it is
not exhaustive, because the `body` is Drive's File resource and every writable
member of it reaches Drive. Each is a
required-approval mutation: it runs only through the host's approval, audit and
mutation coordinator, needs `private_protocol = "connectors-private/2"` and an
approval policy naming it, and is prepared, approved and invoked as
[the guarded merge guide](local-gitlab-merge.md) describes. Without a proof the
write is refused before the provider is asked anything.

The approval binds the whole input by its digest: the subject's `input_sha256`
is the SHA-256 of the canonical input — every `body` member, the target
`fileId`, the pinned `version` and every query parameter — so a proof issued for
one input is refused for any other. The subject shows only that digest; the
presentation `approvals prepare` returns names the instance, operation,
connection and descriptor revision, not the input
(`crates/connectors-host/src/local/owner/approval_issuance.rs`). Read the input
file itself before approving — the whole input, not only the members the table
names: its query parameters and `body` decide what the write does. Writes run on
a separate write instance with the write scope; see
[The write scope](#the-write-scope).

| id | Discovery id | request | guard | beyond naming and describing a file |
|---|---|---|---|---|
| `files.create` | `drive.files.create` | `POST /drive/v3/files` with a metadata `body` | none; the approval binds the whole body | `body.parents` places it, and without it the file goes directly into the user's My Drive; `body.writersCanShare` (whether writers may change its permissions) and `body.copyRequiresWriterPermission` (whether readers and commenters may copy, print or download it) set who may share or copy it; `body.contentRestrictions` locks it (`readOnly`: no new revision, no comments, no title change; `ownerRestricted`: only its owner may lift the lock); `body.downloadRestrictions` restricts downloading it; `ignoreDefaultVisibility` skips the domain's default visibility |
| `files.update` | `drive.files.update` | `PATCH /drive/v3/files/{fileId}` with a metadata `body` | preflight `files.get`: `/version` equals the input `version` | `addParents` and `removeParents` move it; `body.trashed` trashes it; `body.inheritedPermissionsDisabled` removes the access it inherits from its folders; `body.writersCanShare` and `body.copyRequiresWriterPermission` change who may share or copy it; `body.contentRestrictions` locks or unlocks it (`readOnly`, `ownerRestricted`); `body.downloadRestrictions` restricts downloading it |
| `files.copy` | `drive.files.copy` | `POST /drive/v3/files/{fileId}/copy` with a metadata `body` | none; the approval binds the whole body | `body.parents` places the copy, and without it the copy inherits any discoverable parent of the source file — the source's folder and the access it gives, not My Drive as a create would; `copyComments` copies the source's open comments, other people's words included; `body.writersCanShare` (whether writers may change its permissions) and `body.copyRequiresWriterPermission` (whether readers and commenters may copy, print or download it) set who may share or copy it; `body.contentRestrictions` locks it (`readOnly`, `ownerRestricted`); `body.downloadRestrictions` restricts downloading it; `ignoreDefaultVisibility` skips the domain's default visibility |

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
- **A name is not unique within a folder.** Drive keeps any number of files of
  the same name in one folder (the pinned Discovery document, `File.name`:
  "This isn't necessarily unique within a folder"). A create or copy under a
  name that already exists there is not refused and replaces nothing: it makes
  a second file of that name, and a later `files.list` by `name` finds both.
  Identify a file by its `id`, which a write's answer carries unless `fields`
  leaves it out.
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
  answer, which then carries the new version to pin next. The declared input
  schema does not yet mark `fields` required, although the update refuses
  without it; declaring it is `story:catalog-selection-required-parameters`.
- **Shared-drive files are not supported by `files.update` yet.** A caller
  updating a shared-drive file sends `supportsAllDrives`, and the PATCH carries
  it, but the preflight `files.get` is sent with `fileId` and `fields` only: a
  guard value is required, so an optional `supportsAllDrives` cannot be passed
  to the preflight. Google documents that a request for a shared-drive item
  needs `supportsAllDrives=true`, so the preflight cannot read such a file and
  the update is refused before any write (inferred from the documentation, not
  run against live Drive). Optional preflight values are
  `story:catalog-guard-optional-preflight-values`. `files.create` and
  `files.copy` have no preflight and pass `supportsAllDrives` through.
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
    "authorize_url": "https://accounts.google.com/o/oauth2/auth",
    "requested_scopes": ["openid", "https://www.googleapis.com/auth/drive.readonly"]
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/google-drive/operations.json"
}
```

### The write scope

The writes need `https://www.googleapis.com/auth/drive.file`. Google documents
it as "See, edit, create, and delete only the specific Google Drive files you
use with this app" (the pinned Discovery document; see also
[Choose Google Drive API scopes](https://developers.google.com/workspace/drive/api/guides/api-specific-auth)).
That is Google's stated scope semantics, not a behaviour this repository has
verified: no live Drive file has been written, and whether a token holding both
scopes may, for example, copy any readable file or create a file in a folder the
app did not create is not known here.

Writes use a separate instance. Configure a second instance id, such as
`google-drive-write`: the read configuration above with that instance id and
`drive.file` added to both `minimum_scopes` and `requested_scopes`, as its own
adapter entry in the host configuration, with the writes in its operation
permissions and `private_protocol = "connectors-private/2"`. The read scope
stays in it, so its reads and each update's preflight still see every file:

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "google-drive-write",
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
    "authorize_url": "https://accounts.google.com/o/oauth2/auth",
    "requested_scopes": ["openid", "https://www.googleapis.com/auth/drive.readonly", "https://www.googleapis.com/auth/drive.file"]
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/google-drive/operations.json"
}
```

Take its `configuration_revision` from `--print-local-bootstrap` for its
adapter entry, then `connections connect` that instance with the Google client
file. Validation checks the granted scopes against `minimum_scopes`, so an
entry consented for the read scope alone is refused as insufficient scope on
this instance. An existing read-only connection cannot be widened in place:
`connections repair` cannot add a scope, because the configuration revision and
the profile, whose `minimum_scopes` the scope changes, are part of the
connection binding. Changing the scopes of the read instance itself leaves its
connection bound to the old configuration, and a new connection on that same
instance is refused while the old one exists.

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
