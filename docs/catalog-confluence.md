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

The same reads, CQL search and the space reads are also served from Confluence
Cloud REST v1, under a second provider id, `confluence-v1`, built from a source
document this repository writes itself; see [REST v1](#rest-v1). A token that
carries only classic scopes reads Confluence through v1 alone; [Moving a local
install onto REST v1](#moving-a-local-install-onto-rest-v1) adds it to an install that runs the
v2 adapter.

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
  applies. Send `space-id` (one id, several as a JSON array of ids, or several
  as one comma-separated string),
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

## REST v1

Atlassian's REST v1 OpenAPI document declares no licence, so it is not vendored,
copied or fetched. Instead
[`adapters/atlassian/contracts/confluence-rest-v1/confluence-rest-v1.yaml`](../adapters/atlassian/contracts/confluence-rest-v1/README.md)
is an OpenAPI 3.1 document this repository writes itself from Atlassian's public
REST v1 reference pages. Each operation names the page it was written from in
`externalDocs.url`, and the document holds only the paths, query parameters and
response fields the reads below rely on. Its server path is the site's `/wiki`
root, so `/rest/api/search` is recorded as `/wiki/rest/api/search`. The catalog
pipeline compiles it into the `confluence-v1` bundle like any pinned source:

```sh
cargo run --locked -p connectors-build -- catalog \
  --provider confluence-v1 \
  --source adapters/atlassian/contracts/confluence-rest-v1/confluence-rest-v1.yaml \
  --directory adapters/catalog/generated/bundles \
  --auth-profile atlassian.basic
```

It carries seven operations and no unsupported one.
`adapters/catalog/tests/bundle_drift.rs` refuses a committed bundle a fresh run
over the authored document would not reproduce byte for byte.

[`adapters/catalog/providers/confluence-v1/operations.json`](../adapters/catalog/providers/confluence-v1/operations.json)
exposes seven reads, each `effect: read`. An id that REST v2 also serves means
the same read there, so a caller's id is the same whichever API serves it;
`spaces.list`, `space.get` and `content.search` exist under v1 only. The
Atlassian ESS model names both sets (`ConfluenceV1Read`, `ConfluenceV2Read` in
`adapters/atlassian/spec/ess/domains/documents.yaml`), and
`adapters/catalog/tests/confluence_v1.rs` holds each selection set to it.

| id | `operationId` | request | paging parameters | end condition | time filter or deltas |
|---|---|---|---|---|---|
| `spaces.list` | `getSpaces` | `GET /wiki/rest/api/space` with `spaceKey`, `type`, `status` | `start`, `limit` | `_links.next` absent | none |
| `space.get` | `getSpace` | `GET /wiki/rest/api/space/{spaceKey}` | single item | n/a | none |
| `space.pages` | `getContent` | `GET /wiki/rest/api/content` with `spaceKey`, `type` (`page` or `blogpost`), `orderby` | `start`, `limit` | `_links.next` absent | none; deltas come from `pages.changed` |
| `pages.changed` | `searchContentByCQL` | `GET /wiki/rest/api/content/search` with `cql` (required), `expand` | `cursor`, `limit` | `_links.next` absent | in the query: `lastmodified >= "<cutoff>"` |
| `page.get` | `getContentById` | `GET /wiki/rest/api/content/{id}` with `expand=body.storage` | single item | n/a | none; deltas come from `pages.changed` |
| `page.comments` | `getContentComments` | `GET /wiki/rest/api/content/{id}/child/comment` with `location`, `expand` | `start`, `limit` | `_links.next` absent | none; deltas come from `pages.changed` |
| `content.search` | `searchByCQL` | `GET /wiki/rest/api/search` with `cql` (required) | `cursor`, `limit` | `_links.next` absent | in the query |

- **Following a page.** Send the first page without `start` or `cursor`. While
  another page follows, `_links.next` is a path relative to `_links.base` whose
  query holds the next page's position. For the next page of a list
  (`spaces.list`, `space.pages`, `page.comments`) send the `start` from that
  link; for a CQL read (`pages.changed`, `content.search`) send its `cursor`.
  Keep the other parameters unchanged. A page without `_links.next` is the last.
  The selection set bounds `limit` to 1..=100 on each list, so a value outside it
  is refused before any request; the reference states no maximum, and the site
  may return fewer items than asked.
- **`pages.changed`** is the delta read, and the time window is part of the CQL
  query rather than a stop rule: for one space's pages changed since a time, send
  `cql` as `space = DOCS and type = page and lastmodified >= "2026-09-01" order by
  lastmodified desc` and `expand=version`. Each item's last modification is
  `version.when`. Walk to the last page.
- **`content.search`** answers the CQL query with one result per match: `title`,
  `excerpt`, `url` (relative to `/wiki`), `entityType`, `lastModified` and the
  matched `content`. `totalSize` counts every match. A query without `cql` is
  refused before any request.
- **`space.pages`** takes the space key (`spaceKey`, such as `DOCS`), not the
  numeric id REST v2 takes, and `type` (`page` when absent).
- **`page.get`** takes the content `id`. Send `expand=body.storage` to receive the
  body in storage format under `body.storage.value`.
- **`page.comments`** takes the content `id`; `location=footer` limits it to
  footer comments, and `expand=body.storage` adds their bodies.

The configuration differs from REST v2 in `provider`, `operations_file` and the
access read, which is a v1 space list:

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "confluence-cloud-v1",
  "provider": "confluence-v1",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://your-domain.atlassian.net/wiki",
  "auth": {
    "profile": "atlassian.basic",
    "scheme": "basic",
    "header": "Authorization",
    "bearer": false,
    "account_label": "Account email",
    "label": "API token",
    "identity": {"path": "rest/api/user/current", "kind": "atlassian.account", "subject_pointer": "/accountId"},
    "access": {"path": "rest/api/space", "query": {"limit": "1"}}
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/confluence-v1/operations.json"
}
```

Verified against a local HTTPS fixture in the v1 response shape only
(`adapters/catalog/tests/confluence_v1.rs`): each read's exact request and basic
header, the returned body, and a two-page walk of each list to the page without
`_links.next`. No live Confluence Cloud site has been read through REST v1 here,
so the authored paths, parameters and fields rest on the reference pages they
name and have not been checked against a site. The page anchors in
`externalDocs.url` were not checked live.

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
    "identity": {"path": "rest/api/user/current", "kind": "atlassian.account", "subject_pointer": "/accountId"},
    "access": {"path": "api/v2/pages", "query": {"limit": "1"}}
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/confluence/operations.json"
}
```

`access` makes validation also read one page through the v2 API. An Atlassian
API token whose scopes allow the v1 identity read but not the v2 page reads
then refuses to connect as `insufficient_scope` instead of connecting and
answering `unauthorized` on every read (observed 2026-10-06, #102). Create the
token with the Confluence page scopes the shipped reads need.

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

## Moving a local install onto REST v1

This section covers a local install that already reads Confluence through the catalog provider
with adapter `confluence`, on the REST v2 provider (`confluence`, profile `atlassian.basic`). It
adds a second adapter, `confluence-v1`, on the REST v1 provider, connects it with the same
credential, reads through it, and says how to retire `confluence` if it is no longer wanted. The
`confluence` entry stays unless you retire it; the two adapters run side by side.

The outputs below come from a run against a scratch `--config` and `--state-dir`, a private
Secret Service and a local HTTPS fixture standing in for the Confluence site (synthetic email,
token, account id and space ids). The commands are shown without `--config`, `--state-dir` and
`--output json`; long answers are cut down with the `jq` filter shown. A connection id, revision or
schema in your install differs: take each from the command before it. The fixture answers by path
and does not check the `Authorization` header; the header each v1 read sends is checked in
`adapters/catalog/tests/confluence_v1.rs`. Nothing in this section was run against a live site.

### Which provider a token needs

Atlassian answers a REST v2 read with `401` and the message `Unauthorized; scope does not match`
when the token does not carry the scopes v2 needs, while the REST v1 reads answer `200` with the
same token; a token that carries only classic scopes behaves this way. In the run, the
`confluence` connection was made while the fixture still answered v2 reads, and the fixture was
then switched to answer v2 with that `401` and v1 with `200`. A v2 read on the existing connection
is then refused as `unauthorized`, with Atlassian's message as `service_reason`:

```console
$ connectors operations invoke --adapter confluence --connection conn_45354c2a8bca4b7ba52d1ca57c317711_24003a1419404feeac4d71fd71fa1e13 --operation space.pages --schema ad5ebd68ea3cd7e786974a08520c8e0c590a60bf4fb2cdd1a951ef78fc08d476 --revision cb622835084ffe1f256ddc6dafcdb01d8dc38fd4f0a826fd6254becd0a0765b5 --input-json '{"id":"65538"}'
{"error":{"code":"failure","data":{"code":"service_failure","kind":"operational","next_action":"repair_connection","service_code":"unauthorized","service_reason":"Unauthorized; scope does not match","stage":"dispatch"}},"ok":false}
```

Revalidation reads one v2 page as its access read, so it is refused too, and the connection then
waits for a credential:

```console
$ connectors connections revalidate --adapter confluence --connection conn_45354c2a8bca4b7ba52d1ca57c317711_24003a1419404feeac4d71fd71fa1e13 --expected-revision 91dbf8ef-60cb-4c75-8135-7bfbd0bbf5d3
{"error":{"code":"failure","data":{"code":"not_granted","kind":"operational","next_action":"repair_connection","stage":"admission"}},"ok":false}
$ connectors connections list --adapter confluence | jq -c '[.result.connections[]|{connection,revision,state}]'
[{"connection":"conn_45354c2a8bca4b7ba52d1ca57c317711_24003a1419404feeac4d71fd71fa1e13","revision":"91dbf8ef-60cb-4c75-8135-7bfbd0bbf5d3","state":"reauthorization_required"}]
```

A token with these answers reads Confluence through `confluence-v1`, whose access read is the v1
space list. A token whose v2 reads answer `200` can stay on `confluence`; `spaces.list`,
`space.get` and `content.search` are served by `confluence-v1` only.

### 1. Write the native configuration

Write the configuration [REST v1](#rest-v1) shows to its own file, with your site's `/wiki` root as
`api_base` and absolute paths into your checkout or release for `bundle_directory` and
`operations_file`:

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "confluence-cloud-v1",
  "provider": "confluence-v1",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://your-domain.atlassian.net/wiki",
  "auth": {
    "profile": "atlassian.basic",
    "scheme": "basic",
    "header": "Authorization",
    "bearer": false,
    "account_label": "Account email",
    "label": "API token",
    "identity": {"path": "rest/api/user/current", "kind": "atlassian.account", "subject_pointer": "/accountId"},
    "access": {"path": "rest/api/space", "query": {"limit": "1"}}
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/confluence-v1/operations.json"
}
```

Its `instance` must differ from the `confluence` adapter's (`confluence-cloud` in the REST v2
example). The run's file also named the fixture's certificate as `ca_file`; a Confluence Cloud site
needs none. The file must stay owner-only (mode `0600`): a file other users can read is refused as
`invalid_configuration` when the provider loads it.

```console
$ stat -c %a confluence-v1.json
600
```

### 2. Print its configuration revision

`--local-config` takes an absolute path; a relative one is refused:

```console
$ connectors-catalog-provider --local-config confluence-v1.json --print-local-bootstrap
InvalidConfiguration
$ connectors-catalog-provider --local-config "$PWD/confluence-v1.json" --print-local-bootstrap | jq -r .configuration_revision
219744f75efd0f60840c20cf0400f8655bb3aa5d3c5c6346b3254645aa3ac132
```

The revision digests the configuration, so yours differs; print it again after any edit to the
file.

### 3. Add the `[adapters.confluence-v1]` entry

Add the entry beside `[adapters.confluence]`, which stays as it is. The entry carries the revision
from step 2, the shared profile and the seven read ids; an id missing from
`permissions.operations` is denied. `connectors-private/1` is enough, because every
`confluence-v1` operation is a read. The entry of the run, with its two paths and the executable's
digest written as placeholders:

```toml
[adapters.confluence-v1]
instance_id = "confluence-cloud-v1"
adapter_id = "catalog"
configuration_revision = "219744f75efd0f60840c20cf0400f8655bb3aa5d3c5c6346b3254645aa3ac132"
protocol = "v1alpha1"
private_protocol = "connectors-private/1"
startup = "on-demand"
restart = "never"

[adapters.confluence-v1.permissions]
profiles = ["atlassian.basic"]
operations = ["spaces.list", "space.get", "space.pages", "pages.changed", "page.get", "page.comments", "content.search"]

[adapters.confluence-v1.executable]
path = "/absolute/path/connectors-catalog-provider"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/confluence-v1.json"]
```

`instance_id` is the native configuration's `instance`; `path` and `sha256` are the same as in
`[adapters.confluence.executable]` when both adapters run the same provider executable.

```console
$ connectors setup check | jq -c '[.result.prerequisites[]|{name,state}]'
[{"name":"configuration","state":"ready"},{"name":"metadata","state":"ready"},{"name":"artifact:confluence","state":"ready"},{"name":"artifact:confluence-v1","state":"ready"},{"name":"keyring","state":"ready"},{"name":"persistent_custody_qualification","state":"ready"}]
```

### 4. Connect and read

Connect with the credential document the `confluence` adapter uses,
`{"account": "<email>", "token": "<API token>"}`, and the same profile. The connection names the
same Atlassian account as the `confluence` one:

```console
$ connectors connections connect --adapter confluence-v1 --profile atlassian.basic --credential-file ~/private/atlassian.json
{"ok":true,"result":{"connection":{"external_identity":{"kind":"atlassian.account","subject":"557058:00000000-0000-0000-0000-000000000001"},"observed_at_ms":1792082151063,"source":"authority","stale":false,"summary":{"adapter":"confluence-v1","connection":"conn_45354c2a8bca4b7ba52d1ca57c317711_cc802ec777de45a2a56a33c5263f6642","instance_id":"confluence-cloud-v1","profile":"atlassian.basic","revision":"bfeb3248-84c3-4fd6-99ac-14969c623322","state":"ready"},"valid_until_ms":1792082210212}}}
$ connectors operations list --adapter confluence-v1 | jq -c '[.result.operations[].id]'
["spaces.list","space.get","space.pages","pages.changed","page.get","page.comments","content.search"]
```

A read needs no approval. Take `--schema` and `--revision` from `operations describe`, then invoke:

```console
$ connectors operations describe --adapter confluence-v1 --operation spaces.list | jq -c '.result|{schema,revision}'
{"schema":"5439e900f48a6baede73aa22e2274f618faea7f24197b0d02899fe7861191ea9","revision":"2e132d5cc5287cd6781cd2b6a209affc3a81fbfcd5fe95a4bf1004e32a1c7e5d"}
$ connectors operations invoke --adapter confluence-v1 --connection conn_45354c2a8bca4b7ba52d1ca57c317711_cc802ec777de45a2a56a33c5263f6642 --operation spaces.list --schema 5439e900f48a6baede73aa22e2274f618faea7f24197b0d02899fe7861191ea9 --revision 2e132d5cc5287cd6781cd2b6a209affc3a81fbfcd5fe95a4bf1004e32a1c7e5d --input-json '{"limit":25}' | jq -c '.result.result|{status,body}'
{"status":200,"body":{"_links":{"base":"https://your-domain.atlassian.net/wiki","context":"/wiki"},"limit":25,"results":[{"_links":{"webui":"/spaces/DOCS"},"id":65538,"key":"DOCS","name":"Documentation","status":"current","type":"global"},{"_links":{"webui":"/spaces/OPS"},"id":98305,"key":"OPS","name":"Operations","status":"current","type":"global"}],"size":2,"start":0}}
```

Validation evidence lasts 60 seconds; after that a read is refused until `connections revalidate`
renews it ([the catalog provider guide](local-catalog-provider.md)). Note that `space.pages` takes
a space key under v1 (`spaceKey`, such as `DOCS`) where v2 takes the numeric `id`.

### 5. Retire `confluence` (optional)

Removing the REST v2 adapter is optional; it changes nothing for `confluence-v1`, whose connection
holds its own credential version. To retire it, revoke each of its connections first, then remove
`[adapters.confluence]` and its `.permissions` and `.executable` tables from the configuration.
Revocation is local and asks nothing of Atlassian (`provider_outcome` `not_requested`); the API
token stays valid and keeps serving `confluence-v1`.

```console
$ connectors connections revoke --adapter confluence --connection conn_45354c2a8bca4b7ba52d1ca57c317711_24003a1419404feeac4d71fd71fa1e13 --expected-revision 91dbf8ef-60cb-4c75-8135-7bfbd0bbf5d3
{"ok":true,"result":{"adapter":"confluence","connection":"conn_45354c2a8bca4b7ba52d1ca57c317711_24003a1419404feeac4d71fd71fa1e13","local_state":"revoked","provider_outcome":"not_requested","revision":"85fd7771-2812-44f0-a20b-8fe9b5a0ae83"}}
```

After removing the entry:

```console
$ connectors setup check | jq -c '[.result.prerequisites[]|{name,state}]'
[{"name":"configuration","state":"ready"},{"name":"metadata","state":"ready"},{"name":"artifact:confluence-v1","state":"ready"},{"name":"keyring","state":"ready"},{"name":"persistent_custody_qualification","state":"ready"}]
$ connectors adapters list | jq -c '[.result.adapters[]|{adapter,instance_id}]'
[{"adapter":"confluence-v1","instance_id":"confluence-cloud-v1"}]
$ connectors connections list --adapter confluence
{"error":{"code":"failure","data":{"code":"not_found","kind":"operational","next_action":"check_configuration","stage":"admission"}},"ok":false}
$ connectors connections revalidate --adapter confluence-v1 --connection conn_45354c2a8bca4b7ba52d1ca57c317711_cc802ec777de45a2a56a33c5263f6642 --expected-revision bfeb3248-84c3-4fd6-99ac-14969c623322 | jq -r '.result.connection.summary.state'
ready
```

Revoke before removing the entry: once it is gone, every connection command answers `not_found`
for `--adapter confluence`, as above, and a connection left unrevoked can no longer be revoked
through the CLI.

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
- The pinned document declares some query parameters as arrays: `id`,
  `space-id` (both of integers) and `status` on `pages.changed`, and `status` on
  `space.pages`, `page.get` and `page.comments`. Each takes a JSON array, sent
  as one `name=value` pair per element in the order given, or one value (a
  comma-separated string included), sent as one pair as before. For `id` and
  `space-id` that value must be an integer or a string of comma-separated
  integers, such as `"65538,98305"`; anything else is refused as
  `invalid_input` before any request. See
  [array and required query parameters](local-catalog-provider.md#array-and-required-query-parameters).
- The provider does not walk pages itself. It sends a read answered `429` once more after
  the delay its `Retry-After` names, when that wait leaves the second request
  time of its own before the invocation deadline, and otherwise returns it as a `rate_limited` refusal carrying that
  delay as `retry_after_seconds`
  ([Limits](local-catalog-provider.md#limits)).
