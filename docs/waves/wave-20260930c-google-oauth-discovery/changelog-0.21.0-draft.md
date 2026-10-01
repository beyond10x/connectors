## 0.21.0 — 2026-10-01

### Added

- Google providers for the catalog: `google-drive`, `google-slides`, `google-calendar` and
  `google-gmail`, compiled from Google Discovery documents pinned from
  `googleapis/google-api-go-client` (BSD-3-Clause). Reads: Drive about, files, export and
  changes; Slides presentations and pages; Calendar calendar list and events; Gmail profile,
  messages, threads, history, labels and attachments. Guarded writes: Drive metadata create,
  update and copy; Slides create and batchUpdate; Calendar insert, patch and delete; Gmail drafts
  create and send (no direct `messages.send`). Guides: `docs/catalog-google-*.md`.
- `connectors-build discovery` projects a Discovery document into OpenAPI 3.0.3, refusing every
  construct outside its rule table by JSON pointer; `connectors-build catalog --derived-from`
  records the derivation in the bundle.
- Auth scheme `oauth2_refresh`: the stored entry is `{client_id, client_secret, refresh_token}`,
  exchanged for an access token per call and cached in memory. `connections connect
  --credential-file <Google Desktop client JSON>` obtains it by browser consent (loopback redirect,
  PKCE S256); without a terminal the consent address is printed as `connectors: consent-url <url>`.
- Catalog selections: query parameters declared as form-exploded arrays take a JSON array (one
  pair per element); `required` marks parameters a provider needs; `rate_limit_reasons` reports a
  quota 403 as `rate_limited`; `body_keys` closes a write body.

### Changed

- An unauthorized `operations invoke` of a read reports `repair_connection`; connect, repair,
  revalidate and guarded writes keep their previous next action.
- An empty text response body is `""`, not `null`.

### Compatibility

- Every GitLab, Jira and Confluence bundle is rebuilt, so their configuration and descriptor
  revisions move: copy the new `configuration_revision` into the adapter entry, connect an
  instance that was already connected under a new instance id, and issue write approval policies
  again.
- A repeated query parameter still accepts the comma-joined string form.

### Limits

- A Google OAuth app in Testing issues refresh tokens that expire after 7 days.
- The engine does not check enum values (for example Calendar `sendUpdates`, Gmail `format`).
- Drive `files.update` on a shared-drive file is not supported yet.
- Not run against live Google yet; the live read of a deck waits on a Desktop OAuth client.
