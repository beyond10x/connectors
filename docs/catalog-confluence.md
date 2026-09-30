# Confluence Cloud through the catalog provider

The catalog provider reads Confluence Cloud pages, their bodies and their footer
comments from the pinned Confluence Cloud REST v2 OpenAPI document. Nothing here
is Confluence-specific code: the pinned document is compiled into a bundle, a
reviewed selection set exposes four reads, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them.
Configuration, connection, approval and invocation work exactly as described
there; this page covers what differs for Confluence. Jira and Confluence share
one Atlassian credential and one authentication profile, `atlassian.basic`; see
[Authentication](#authentication) and [the Jira guide](catalog-jira.md).

## Source and bundle

The pinned source is
[`adapters/atlassian/upstream/confluence/confluence-v2.json`](../adapters/atlassian/upstream/confluence/README.md),
SHA-256 `edb639bbc700ee451a996acd2568e51db4ceab954449427537df30f0ce20ca08`,
retrieved on 2026-09-30 from
<https://developer.atlassian.com/cloud/confluence/openapi-v2.v3.json>. It is the
only Confluence document the provider is built from. The document's paths are
relative to its server path `/wiki/api/v2`, and the bundle records each one
below it, so `/pages` is recorded as `/wiki/api/v2/pages`. The bundle is built
with:

```sh
cargo run --locked -p connectors-build -- catalog \
  --provider confluence \
  --source adapters/atlassian/upstream/confluence/confluence-v2.json \
  --directory adapters/catalog/generated/bundles \
  --auth-profile atlassian.basic
```

It carries all 218 operations of the pinned document. 30 are named
unsupported: each is a write whose request body is a `$ref`, which the
pipeline does not resolve, and none of them is a shipped read.
`adapters/catalog/tests/bundle_drift.rs` pins that count and refuses a
committed bundle or index a fresh run would not reproduce byte for byte.

## The shipped selection set

[`adapters/catalog/providers/confluence/operations.json`](../adapters/catalog/providers/confluence/operations.json)
exposes four reads and nothing else. Each is `effect: read`; there are no
writes. `adapters/catalog/tests/confluence.rs` pins this exact id list and each
id's `operationId` and path in the pinned document, so a renamed or dropped id,
or a source operation that moved, fails the gate. The bundle refuses at load any
`operation_id` the pinned document lacks.

Every list returns one page per call. The provider returns `status`, `body` and
`provenance`; `body` is Confluence's page unchanged, and the fields that decide
the end of a walk are in it.

| id | pinned `operationId` | request | paging parameters | end condition | time filter or deltas |
|---|---|---|---|---|---|
| `pages.changed` | `getPages` | `GET /wiki/api/v2/pages` with `space-id`, `sort=-modified-date` (required), `body-format` | `cursor`, `limit` | `_links.next` absent or null, or the stop rule | no updated-since parameter; stop at the first page listing an item with `version.createdAt` older than the cutoff |
| `space.pages` | `getPagesInSpace` | `GET /wiki/api/v2/spaces/{id}/pages` | `cursor`, `limit` | `_links.next` absent or null | none; deltas come from `pages.changed` |
| `page.get` | `getPageById` | `GET /wiki/api/v2/pages/{id}` with `body-format=storage` | single item | n/a | none; deltas come from `pages.changed` |
| `page.comments` | `getPageFooterComments` | `GET /wiki/api/v2/pages/{id}/footer-comments` | `cursor`, `limit` | `_links.next` absent or null | none; deltas come from `pages.changed` |

- **Following a page.** Send the first page without `cursor`. The pinned
  document says each list's `_links.next` "contains the relative URL for the
  next set of results, using a cursor query parameter" and "will not be present
  if there is no additional data available". For the next page, send the value
  of `cursor` from that URL, with the other parameters unchanged. A page whose
  `_links.next` is absent or null is the last. The document states a default
  `limit` of 25, a minimum of 1 and a maximum of 250 for each list read; the
  selection set bounds `limit` to that range, so a value outside it is refused
  before any request.
- **`pages.changed`** is the delta read. The pinned document gives `getPages`
  no updated-since parameter, so the time window is a stop rule the caller
  applies. Send `space-id` (one id, or several as one comma-separated string),
  `sort=-modified-date` and `body-format` (`storage` for the body in storage
  format). `sort=-modified-date` is required: the stop rule only holds when the
  newest-modified pages come first, and without it the deltas are wrong. A
  selection cannot fix a parameter's value, so the caller must send it. A listed
  page's last modification is `version.createdAt`, the time its current version
  was created (schema `PageBulk.version`, `Version.createdAt`, format
  `date-time`); the schema does not require `version`. Keep the pages whose
  `version.createdAt` is at or after your cutoff, and stop at the first page
  that lists one older than it. A listed page without `version.createdAt`
  counts as changed: keep it and keep walking; only a page with a timestamp
  older than the cutoff stops the walk. Stop earlier if `_links.next` is absent
  or null. Compare the values as instants; the document writes them as
  `YYYY-MM-DDTHH:mm:ss.sssZ`.
- **`space.pages`** takes the space `id` (numeric, not the space key) and lists
  every page in the space; it has no time filter.
- **`page.get`** takes the page `id`. Send `body-format=storage` to receive the
  body in storage format under `body.storage.value`; without it the page carries
  no body. The document also accepts `atlas_doc_format` and `view`.
- **`page.comments`** takes the page `id` and lists its footer comments; send
  `body-format=storage` for their bodies. It has no time filter; re-read the
  comments of each page `pages.changed` returns.

Whether a new comment moves its page's `version.createdAt` is Confluence's
behaviour and has not been checked against a live site here. Every other query
parameter the pinned document declares for an operation is accepted by name; one
it does not declare is refused before any request.

## Authentication

Jira and Confluence use one Atlassian API token with the account email, over
HTTP basic, and one profile, `atlassian.basic`. The profile id, scheme, labels
and identity kind are the same in both configurations; only the identity read
differs, because each product answers it at its own path. Both name the same
subject, the Atlassian `accountId`, so one credential connects both adapters as
one identity (`atlassian.account`).

The pinned v2 document has no read that names the current user, so the
Confluence identity read is `GET /wiki/rest/api/user/current`, a REST v1 read
that is not part of the pinned document and not an operation of this provider;
it is only the profile's identity probe. For that reason `api_base` is the
site's `/wiki` root, not `/wiki/api/v2`. Atlassian API tokens expose no scope
list, so the profile declares no `scopes` read and no `minimum_scopes`.

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "confluence-cloud",
  "provider": "confluence",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://your-domain.atlassian.net/wiki",
  "auth": {
    "profile": "atlassian.basic",
    "scheme": "basic",
    "header": "Authorization",
    "bearer": false,
    "account_label": "Account email",
    "label": "API token",
    "identity": {"path": "rest/api/user/current", "kind": "atlassian.account", "subject_pointer": "/accountId"}
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/confluence/operations.json"
}
```

Connect each adapter with the same credential document
`{"account": "<email>", "token": "<API token>"}` and the same profile; every
request then carries `Authorization: Basic base64(<email>:<API token>)`:

```sh
target/release/connectors --output json connections connect --adapter jira --profile atlassian.basic --credential-file /owner-only/atlassian.json
target/release/connectors --output json connections connect --adapter confluence --profile atlassian.basic --credential-file /owner-only/atlassian.json
```

### Through the API gateway

Service-account API tokens and OAuth 2.0 (3LO) access tokens work only through
the Atlassian API gateway, `https://api.atlassian.com/ex/confluence/<cloud id>/…`.
The `/wiki` root follows the gateway part, so write
`https://api.atlassian.com/ex/confluence/<cloud id>/wiki` as `api_base` and name
the gateway part in `request_prefix`. The cloud id is the same one Jira uses on
that site (see [the Jira guide](catalog-jira.md#through-the-api-gateway)). The
pinned paths (`/wiki/api/v2/…`) are checked against what follows the prefix, and
every request goes to the full `api_base`, the identity read included:
`GET /ex/confluence/<cloud id>/wiki/rest/api/user/current`.

A service-account API token uses the same `atlassian.basic` profile as on the
site, with the service account's email as the account and the credential
document `{"account": "<service account email>", "token": "<API token>"}`. The
same credential then connects Jira and Confluence as one identity. This form was
checked live for Jira on 2026-09-30, not yet for Confluence.

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "confluence-cloud",
  "provider": "confluence",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://api.atlassian.com/ex/confluence/your-cloud-id/wiki",
  "request_prefix": "/ex/confluence/your-cloud-id",
  "auth": {
    "profile": "atlassian.basic",
    "scheme": "basic",
    "header": "Authorization",
    "bearer": false,
    "account_label": "Account email",
    "label": "API token",
    "identity": {"path": "rest/api/user/current", "kind": "atlassian.account", "subject_pointer": "/accountId"}
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/confluence/operations.json"
}
```

An OAuth 2.0 (3LO) access token is sent as a bearer token, with the credential
document, expiry and scopes as in the Jira guide. **Not verified live.**

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "confluence-cloud",
  "provider": "confluence",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://api.atlassian.com/ex/confluence/your-cloud-id/wiki",
  "request_prefix": "/ex/confluence/your-cloud-id",
  "auth": {
    "profile": "atlassian.bearer",
    "header": "Authorization",
    "bearer": true,
    "label": "Atlassian access token",
    "identity": {"path": "rest/api/user/current", "kind": "atlassian.account", "subject_pointer": "/accountId"}
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/confluence/operations.json"
}
```

## Limits

- Verified against a local HTTPS fixture only
  (`adapters/catalog/tests/confluence.rs`): the exact request of each read,
  including `body-format=storage` and the basic header, the returned body as
  JSON, a two-page walk of each list to a missing or null `_links.next`, and a
  `pages.changed` walk that stops at the cutoff while a further page is still
  linked. No live Confluence Cloud site has been read, and the identity read has
  not been run against one.
- The engine parses and re-serialises the body, so it is returned as equal JSON,
  not as Confluence's exact bytes.
- The cursor is sent as given. Take it from `_links.next` percent-decoded, or the
  engine encodes it a second time.
- `space-id` is declared as an array by the pinned document; the engine sends
  one value per parameter, so pass one id or a comma-separated string.
- The provider does not walk pages itself and does not retry on `429`; a
  rate-limited read is returned as a refusal.
