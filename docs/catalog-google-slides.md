# Google Slides through the catalog provider

The catalog provider reads the structure of Google Slides presentations — the
presentation, one page, and a page's thumbnail URL — and creates and updates
presentations under approval, from the pinned Google Slides API v1 Discovery
document. Nothing here is Slides-specific code: the Discovery document is
projected into OpenAPI, the projection is compiled into a bundle, a reviewed
selection set exposes three reads and two guarded writes, and the engine
described in [the catalog provider guide](local-catalog-provider.md) binds and
sends them.
Configuration, connection, approval and invocation work as described there; this
page covers what differs for Slides. Authentication is the `oauth2_refresh`
profile `google.oauth`, as for [Google Drive](catalog-google-drive.md); see
[Authentication](#authentication).

## Source and bundle

The pinned source is
[`adapters/google/upstream/slides/slides-api.json`](../adapters/google/upstream/slides/README.md),
SHA-256 `49fa379e29948696029ec8c4b334939f0bd6e74d9ef27650f523682283c1d3f1`,
Discovery `revision` `20260921`. It is projected under the rule table of
`crates/connectors-catalog/src/discovery.rs`, and the bundle is compiled from
the projection with the derivation recorded:

```sh
cargo run --locked -p connectors-build -- discovery \
  --source adapters/google/upstream/slides/slides-api.json \
  --out adapters/google/generated/slides.openapi.json
cargo run --locked -p connectors-build -- catalog \
  --provider google-slides \
  --source adapters/google/generated/slides.openapi.json \
  --derived-from adapters/google/upstream/slides/slides-api.json \
  --directory adapters/catalog/generated/bundles \
  --auth-profile google.oauth \
  --replace
```

`--replace` is needed because the committed index already carries
`google-slides`; without it the second command is refused with "provider is
already indexed". The first command writes `slides.openapi.json` and its projection record
`slides.openapi.projection.json`: all 5 methods of the document are projected
and none is excluded. The document's service path is empty, so the projection's
one server is `https://slides.googleapis.com` and every path carries its own
`/v1`. The Discovery path `v1/presentations/{+presentationId}` uses reserved
expansion; the projection writes it as `/v1/presentations/{presentationId}` and
lists the rewrite in its record. The bundle carries all 5 operations and names
none unsupported. `adapters/catalog/tests/bundle_drift.rs` regenerates the
projection and its record from the pinned document and the bundle from the
projection, and refuses any of them, or the index, that a fresh run would not
reproduce byte for byte.

## The shipped selection set

[`adapters/catalog/providers/google-slides/operations.json`](../adapters/catalog/providers/google-slides/operations.json)
exposes three reads and the document's two writes, and nothing else. The reads
are `effect: read`; `presentations.create` and `presentations.batchUpdate` are
`effect: write` (see [Writes](#writes)). A selection id is the Discovery method
id without its `slides.` prefix. `adapters/catalog/tests/google_slides.rs` pins
this exact id list and each id's Discovery id, path and effect, so a renamed or
dropped id, or a method that moved, fails the gate. The bundle refuses at load
any `operation_id` the projection lacks.

The provider returns `status`, `body` and `provenance`; `body` is the Slides
answer unchanged. None of the three reads is paged.

| id | Discovery id | request | answer |
|---|---|---|---|
| `presentations.get` | `slides.presentations.get` | `GET /v1/presentations/{presentationId}` | the `Presentation`: `slides`, `layouts`, `masters` and their page elements |
| `presentations.pages.get` | `slides.presentations.pages.get` | `GET /v1/presentations/{presentationId}/pages/{pageObjectId}` | one `Page` and its page elements |
| `presentations.pages.getThumbnail` | `slides.presentations.pages.getThumbnail` | `GET /v1/presentations/{presentationId}/pages/{pageObjectId}/thumbnail` | a `Thumbnail`: `contentUrl`, `width`, `height` |

- **Path values.** `presentationId` and `pageObjectId` are sent as one path
  segment each. A value containing `/` is percent-escaped (`a/b` is sent as
  `a%2Fb`) and never split into two segments, including for the reserved
  expansion of `presentations.get`; Google then answers such an id itself.
- **`presentations.pages.getThumbnail`** returns JSON with a `contentUrl`. The
  provider does not fetch the image. The pinned document says that URL is
  tagged with the requesting account, lasts about 30 minutes, and gives anyone
  holding it the requester's access to the image, so treat it as a credential.
  `thumbnailProperties.mimeType` (only `PNG`) and
  `thumbnailProperties.thumbnailSize` (`SMALL`, `MEDIUM`, `LARGE`,
  `WIDTH2000_PX`) are sent under those exact names.
- **`commentsViewMode`** is accepted on `presentations.get` and
  `presentations.pages.get`, as the document declares it.

Every other query parameter the projection declares for an operation is accepted
by name, including the document-wide `fields`; one it does not declare is
refused before any request.

## Response size and `fields`

A whole presentation can be large. The response limit applies to every read: an
answer over 4 MiB (`connectors_core::RESPONSE_LIMIT`) is refused as `capacity`,
not truncated. Narrow the answer with `fields`, Google's partial-response
selector: for example
`{"presentationId": "<id>", "fields": "presentationId,title,slides.objectId"}`
lists the slide ids only, and each slide can then be read with
`presentations.pages.get`.

## Writes

| id | Discovery id | request | guard |
|---|---|---|---|
| `presentations.create` | `slides.presentations.create` | `POST /v1/presentations` | none |
| `presentations.batchUpdate` | `slides.presentations.batchUpdate` | `POST /v1/presentations/{presentationId}:batchUpdate` | preflight `presentations.get`: `revisionId` must equal the body's `writeControl.requiredRevisionId` |

They run on a separate write instance with the write scope; see
[Authentication](#authentication). Both are required-approval mutations, like
every catalog write: select
`private_protocol = "connectors-private/2"`, permit them in the adapter's
operation permissions, name them in the approval policy, and invoke each with a
proof issued for its exact input, as the
[guarded merge guide](local-gitlab-merge.md) walks through for GitLab. The
approval subject carries the digest of the whole input, `body` included, so a
proof issued for one body is refused for any other. A write offered without an
approval is refused before the provider sends anything.

- **`presentations.create`** takes a `body` and nothing else. The pinned
  document says Google uses only its `title` and, if given, its
  `presentationId`, and ignores every other field. It carries no guard: there is
  nothing to compare before a presentation exists. Media upload is not
  projected; the body is JSON metadata only.
- **`presentations.batchUpdate`** takes `presentationId` and a `body` with
  `requests` and `writeControl.requiredRevisionId`. Read the current
  `revisionId` with `presentations.get` (narrowed with
  `{"fields": "revisionId"}`) and pin it there:

  ```json
  {"presentationId": "<id>", "body": {"requests": [{"createSlide": {}}], "writeControl": {"requiredRevisionId": "<revisionId>"}}}
  ```

  An input that pins no revision is refused before any Slides request.
  Otherwise the provider reads the presentation once and, when its
  `revisionId` differs from the pinned one, refuses with no POST sent. Google checks `requiredRevisionId` again when it applies the
  requests, and applies them all or none. After the POST the acknowledgement's
  `presentationId` must equal the input's; otherwise the outcome is reported as
  unknown, never as refused. A `fields` value on `batchUpdate` must therefore
  keep `presentationId` (for example `presentationId,replies`): an answer
  narrowed without it carries no `presentationId`, and a write Google applied is
  then reported as unknown.
- **Image URLs are published.** `createImage`, `replaceImage` and
  `replaceAllShapesWithImage` take an image URL, which the pinned document says
  is saved with the image and exposed as `Image.sourceUrl`: every viewer of the
  deck can read it. Never use a `presentations.pages.getThumbnail`
  `contentUrl` as an image URL; that URL grants the requester's access to
  whoever holds it, and the provider forwards the request unchanged.
- **Comment requests.** `insertComment`, `deleteComment`, `addCommentReply`,
  `deleteCommentReply` and `updateCommentPost` are Developer Preview in the
  pinned document. Their updates can fail on a 200: the answer's
  `commentUpdateState` is then `ALL_FAILED_UNKNOWN_REASON`, and the engine
  reports the write applied, because it compares only `presentationId`. A
  caller that sends comment requests must read `commentUpdateState` in the
  answer; "all or none" above does not cover them.
- The preflight reads the whole presentation, with no `fields`, so it is subject
  to the 4 MiB response limit: a presentation whose full answer exceeds it
  cannot be updated through this selection, and the write is refused before
  dispatch. The pinned document says `revisionId` is populated only for a user
  with edit access. A view-only connection's `batchUpdate` therefore fails its
  preflight as `upstream_protocol`, because the answer carries no `revisionId`,
  and nothing is written; that code here means missing edit access, not a
  provider fault.

## Authentication

Slides uses Google OAuth: the `oauth2_refresh` scheme under the profile
`google.oauth`, as described in
[the catalog provider guide](local-catalog-provider.md). The protected entry is
`{"client_id":"...","client_secret":"...","refresh_token":"..."}` for an
installed-app OAuth client. The provider exchanges it at `token_url` for an
access token and sends that as `Authorization: Bearer <access token>`. The
identity comes from the token answer's `id_token` (`identity.source: id_token`).
`minimum_scopes` asks for the Slides read-only scope, which the pinned document
accepts for all three reads. `authorize_url` and `requested_scopes` are never
called by the provider; they are handed to the host for obtaining the entry by
consent.

The read-only scope does not cover the writes. The pinned document accepts
`https://www.googleapis.com/auth/presentations` for both, and
not `presentations.readonly`. An instance that writes names
`https://www.googleapis.com/auth/presentations` in both `minimum_scopes` and
`requested_scopes`; it grants the reads too. With that configuration, a stored
refresh token that was granted only the read-only scope fails validation as
insufficient scope.

Writes use a separate instance. Configure a second instance id, such as
`google-slides-write`: a copy of the configuration below with these values
changed, as its own adapter entry in the host configuration with the writes in
its operation permissions and `private_protocol = "connectors-private/2"`:

```json
{
  "instance": "google-slides-write",
  "auth": {
    "minimum_scopes": ["https://www.googleapis.com/auth/presentations"],
    "requested_scopes": ["openid", "https://www.googleapis.com/auth/presentations"]
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
  "instance": "google-slides",
  "provider": "google-slides",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://slides.googleapis.com",
  "auth": {
    "profile": "google.oauth",
    "scheme": "oauth2_refresh",
    "header": "Authorization",
    "bearer": true,
    "label": "Google refresh token",
    "identity": {"source": "id_token", "kind": "google.user"},
    "minimum_scopes": ["https://www.googleapis.com/auth/presentations.readonly"],
    "token_url": "https://oauth2.googleapis.com/token",
    "authorize_url": "https://accounts.google.com/o/oauth2/auth",
    "requested_scopes": ["openid", "https://www.googleapis.com/auth/presentations.readonly"]
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/google-slides/operations.json"
}
```

## Limits

- Verified against a local HTTPS fixture only
  (`adapters/catalog/tests/google_slides.rs`): the guide's configuration with the
  fixture as API and token host, one token exchange, the exact request of each
  read with the exchanged bearer, the returned body as JSON, and a `/` in every
  path parameter sent as one escaped segment. No live presentation has been
  read; live evidence belongs to `story:catalog-google-live-deck-read`.
- The writes are verified against the same fixture through the host's
  prepare/commit exchange: the preflight read, a stale or missing pinned
  revision refused with no POST, and the exact POST body. The approval binding
  is verified with the host's approval signer and verifier against a subject
  built from the provider's descriptor, not through the CLI and owner. No live
  presentation has been created or changed.
- The engine parses and re-serialises the body, so it is returned as equal JSON,
  not as Google's exact bytes.
- The provider does not retry on `429`; a rate-limited read is returned as a
  refusal.
