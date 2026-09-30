# Google Slides through the catalog provider

The catalog provider reads the structure of Google Slides presentations — the
presentation, one page, and a page's thumbnail URL — from the pinned Google
Slides API v1 Discovery document. Nothing here is Slides-specific code: the
Discovery document is projected into OpenAPI, the projection is compiled into a
bundle, a reviewed selection set exposes three reads, and the engine described
in [the catalog provider guide](local-catalog-provider.md) binds and sends them.
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
  --auth-profile google.oauth
```

The first command writes `slides.openapi.json` and its projection record
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
exposes three reads and nothing else. Each is `effect: read`; the document's two
writes (`presentations.create`, `presentations.batchUpdate`) are not selected. A
selection id is the Discovery method id without its `slides.` prefix.
`adapters/catalog/tests/google_slides.rs` pins this exact id list and each id's
Discovery id and path, so a renamed or dropped id, or a method that moved, fails
the gate. The bundle refuses at load any `operation_id` the projection lacks.

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
    "authorize_url": "https://accounts.google.com/o/oauth2/v2/auth",
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
- The engine parses and re-serialises the body, so it is returned as equal JSON,
  not as Google's exact bytes.
- The provider does not retry on `429`; a rate-limited read is returned as a
  refusal.
