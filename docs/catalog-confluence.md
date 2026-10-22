# Confluence Cloud through the catalog provider

The catalog provider reads Confluence Cloud pages, their bodies and their footer
comments from the pinned Confluence Cloud REST v2 OpenAPI document. Nothing here
is Confluence-specific code: the pinned document is compiled into a bundle, a
reviewed selection set exposes four reads, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them.
Connection and invocation work as described there; this page configures the
`confluence` and `confluence-v1` connectors and covers what differs for
Confluence. Connector, instance and connection are defined in
[the README](../README.md#vocabulary). Jira and Confluence share
one Atlassian credential and one authentication profile, `atlassian.basic`; see
[Authentication](#authentication) and [the Jira guide](catalog-jira.md).

The same reads, CQL search and the space reads are also served from Confluence
Cloud REST v1, under a second connector, `confluence-v1`, built from a source
document this repository writes itself; see [REST v1](#rest-v1). A token that
carries only classic scopes reads Confluence through v1 alone; [Moving a local
install onto REST v1](#moving-a-local-install-onto-rest-v1) adds it to an install that runs the
v2 connector.

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

The set declares no page feed. A `datasource.feed/v1alpha1` binding under
profile `confluence-pages/1` is specified in
[confluence-v1.md](../adapters/catalog/contracts/feed/v1alpha1/confluence-v1.md)
and held to the family's suite over a test fixture, but it is not in the shipped
selection, so `feed.containers` and `feed.items` are not `confluence-v1`
operations: the connector definition format the shipped selections round-trip
through cannot carry it yet. Only REST v1 can serve it; REST v2 lists pages with
no time filter to resume by.

The `confluence-v1` connector
([`adapters/catalog/providers/confluence-v1/connector.json`](../adapters/catalog/providers/confluence-v1/connector.json))
declares the same `atlassian.basic` profile as `confluence`; only its access
read differs, a v1 space list. An instance names the same `/wiki` root:

```toml
[connectors.confluence-v1]
enabled = true

[connectors.confluence-v1.instances.work]
endpoint = "https://your-domain.atlassian.net/wiki"
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
and identity kind are the same for both connectors; only the identity read
differs, because each product answers it at its own path. Both name the same
subject, the Atlassian `accountId`, so one credential connects an instance of
each as one identity (`atlassian.account`).

The pinned v2 document has no read that names the current user, so the
Confluence identity read is `GET /wiki/rest/api/user/current`, a REST v1 read
that is not part of the pinned document and not an operation of this provider;
it is only the profile's identity probe. For that reason the endpoint is the
site's `/wiki` root, not `/wiki/api/v2`. Atlassian API tokens expose no scope
list, so the profile declares no `scopes` read and no `minimum_scopes`.

The `confluence` connector declares that profile with an access read of one
v2 page
([`adapters/catalog/providers/confluence/connector.json`](../adapters/catalog/providers/confluence/connector.json))
and no default endpoint: an instance names its site's `/wiki` root. Add to the
host configuration (`~/.config/connectors/config.toml`, format
`connectors-local/5`), beside the `jira` connector of
[the Jira guide](catalog-jira.md#authentication):

```toml
[connectors.confluence]
enabled = true

[connectors.confluence.instances.work]
endpoint = "https://your-domain.atlassian.net/wiki"
```

The bundle, the four reads, the profile and the provider executable are
derived from the shipped connector:

```console
$ connectors instances list
{"adapters":[{"adapter":"confluence.work","adapter_id":"catalog","connector":"confluence","instance":"work","instance_id":"confluence.work","restart":"never","startup":"on-demand"}],"source":"configuration"}
```

The access read makes validation also read one page through the v2 API. An
Atlassian API token whose scopes allow the v1 identity read but not the v2 page
reads then refuses to connect as `insufficient_scope` instead of connecting and
answering `unauthorized` on every read (observed 2026-10-06, #102). Create the
token with the Confluence page scopes the shipped reads need.

Connect each instance with the same credential document
`{"account": "<email>", "token": "<API token>"}` and the same profile; every
request then carries `Authorization: Basic base64(<email>:<API token>)`:

```sh
connectors connections connect --connector jira --instance work \
  --profile atlassian.basic --credential-file ~/private/atlassian.json
connectors connections connect --connector confluence --instance work \
  --profile atlassian.basic --credential-file ~/private/atlassian.json
```

An installation configured before connectors were declared converts its
entries with `connectors setup migrate`, which keeps each instance id and
revision; the pinned form it writes is described in
[the catalog provider guide](local-catalog-provider.md).

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

No connector declaration fixes `request_prefix`, so a gateway instance is not
derived: it is a pinned instance, whose native configuration file is written as
[the catalog provider guide](local-catalog-provider.md) describes, with the
site form's fields and these values.

A service-account API token uses the same `atlassian.basic` profile as on the
site, with the service account's email as the account and the credential
document `{"account": "<service account email>", "token": "<API token>"}`. The
same credential then connects Jira and Confluence as one identity. This form was
checked live for Jira on 2026-09-30, not yet for Confluence. The gateway form
reads no page at validation:

```json
{
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
  }
}
```

An OAuth 2.0 (3LO) access token is sent as a bearer token, under a profile the
`confluence` connector does not declare, with the credential document, expiry
and scopes as in the Jira guide. **Not verified live.**

```json
{
  "api_base": "https://api.atlassian.com/ex/confluence/your-cloud-id/wiki",
  "request_prefix": "/ex/confluence/your-cloud-id",
  "auth": {
    "profile": "atlassian.bearer",
    "header": "Authorization",
    "bearer": true,
    "label": "Atlassian access token",
    "identity": {"path": "rest/api/user/current", "kind": "atlassian.account", "subject_pointer": "/accountId"}
  }
}
```

## Moving a local install onto REST v1

This section covers a local install that already reads Confluence through the `confluence`
connector (REST v2, profile `atlassian.basic`). It adds an instance of the `confluence-v1`
connector, connects it with the same credential, reads through it, and says how to retire the
`confluence` instance if it is no longer wanted. The two connectors' instances run side by side.

### Which connector a token needs

Atlassian answers a REST v2 read with `401` and the message `Unauthorized; scope does not match`
when the token does not carry the scopes v2 needs, while the REST v1 reads answer `200` with the
same token; a token that carries only classic scopes behaves this way. A v2 read on an existing
`confluence` connection is then refused as `unauthorized`, with Atlassian's message as
`service_reason`. This answer was observed on 2026-10-15 against a local HTTPS fixture standing in
for the site, not against a live site:

```json
{"error":{"code":"failure","data":{"code":"service_failure","kind":"operational","next_action":"repair_connection","service_code":"unauthorized","service_reason":"Unauthorized; scope does not match","stage":"dispatch"}},"ok":false}
```

The refusal leaves the connection in the `stale` state, which `connections list` shows: the
provider refused its credential, and no read is admitted on it until a reconnect publishes a new
one. A token with these answers reads Confluence through `confluence-v1`, whose access read is the
v1 space list. A token whose v2 reads answer `200` can stay on `confluence`; `spaces.list`,
`space.get` and `content.search` are served by `confluence-v1` only. One instance can also
choose between the two each time it validates: see
[One instance for either API](#one-instance-for-either-api).

### 1. Add the `confluence-v1` instance

Add the connector and an instance with your site's `/wiki` root, beside the `confluence` one,
which stays as it is:

```toml
[connectors.confluence-v1]
enabled = true

[connectors.confluence-v1.instances.work]
endpoint = "https://your-domain.atlassian.net/wiki"
```

Nothing else is written: the bundle, the seven reads, the shared profile with its v1 access read
and the provider executable come from the shipped connector, and every `confluence-v1` operation
is a read. Both instances are then configured:

```console
$ connectors instances list
{"adapters":[{"adapter":"confluence-v1.work","adapter_id":"catalog","connector":"confluence-v1","instance":"work","instance_id":"confluence-v1.work","restart":"never","startup":"on-demand"},{"adapter":"confluence.work","adapter_id":"catalog","connector":"confluence","instance":"work","instance_id":"confluence.work","restart":"never","startup":"on-demand"}],"source":"configuration"}
```

### 2. Connect and read

Connect with the credential document the `confluence` instance uses,
`{"account": "<email>", "token": "<API token>"}`, and the same profile. The connection names the
same Atlassian account as the `confluence` one:

```sh
connectors connections connect --connector confluence-v1 --instance work \
  --profile atlassian.basic --credential-file ~/private/atlassian.json
connectors operations list --connector confluence-v1 --instance work
```

The operations are `spaces.list`, `space.get`, `space.pages`, `pages.changed`, `page.get`,
`page.comments` and `content.search`. A read needs no approval: take `--schema` and `--revision`
from `operations describe`, then invoke, here with the connection id `connections connect`
answered:

```sh
connectors operations describe --connector confluence-v1 --instance work --operation spaces.list
connectors operations invoke --connector confluence-v1 --instance work \
  --connection "$connection" --operation spaces.list \
  --schema "$schema" --revision "$revision" --input-json '{"limit":25}'
```

Validation evidence lasts 60 seconds; after that a read is refused until `connections revalidate`
renews it ([the catalog provider guide](local-catalog-provider.md)). Note that `space.pages` takes
a space key under v1 (`spaceKey`, such as `DOCS`) where v2 takes the numeric `id`.

The connect, describe and invoke above have not been run against a live site; each v1 read's
request and answer are checked against a local HTTPS fixture in
`adapters/catalog/tests/confluence_v1.rs`.

### 3. Retire the `confluence` instance (optional)

Removing the REST v2 instance is optional; it changes nothing for `confluence-v1.work`, whose
connection holds its own credential version. To retire it, revoke each of its connections first,
then remove `[connectors.confluence.instances.work]`, and `[connectors.confluence]` with it when
it holds no other instance. Revocation is local and asks nothing of Atlassian
(`provider_outcome` `not_requested`); the API token stays valid and keeps serving
`confluence-v1`.

```sh
connectors connections revoke --connector confluence --instance work \
  --connection "$connection" --expected-revision "$revision"
```

Revoke before removing the instance: once it is gone, every connection command answers
`not_found` for `--connector confluence --instance work`, and a connection left unrevoked can no
longer be revoked through the CLI. With only the `confluence-v1` instance left:

```console
$ connectors instances list
{"adapters":[{"adapter":"confluence-v1.work","adapter_id":"catalog","connector":"confluence-v1","instance":"work","instance_id":"confluence-v1.work","restart":"never","startup":"on-demand"}],"source":"configuration"}
$ connectors connections list --connector confluence --instance work
failure: {"code":"not_found","kind":"operational","next_action":"check_configuration","stage":"admission"}
```

## One instance for either API

An instance of `confluence` or `confluence-v1` reads through one API, so the token decides which
connector to configure. One instance can instead bind its operations, each time it validates, to
the API its credential reads: REST v2 when the v2 access read answers, REST v1 when v2 refuses
that read with `401` and `scope does not match` and the v1 access read answers. An id means the
same read under either API, so a caller asks for `page.get` and receives the page through the
API the connection is bound to. The pairing is configuration, the `bindings` member of
[the catalog provider guide](local-catalog-provider.md#alternative-bindings); the provider
names no Confluence API.

### 1. Write the native configuration

Such an instance is pinned: write its native configuration as
[the catalog provider guide](local-catalog-provider.md#configure-the-provider) describes, with
`provider` `confluence`, your site's `/wiki` root as `api_base`, the shipped `atlassian.basic`
profile of [Authentication](#authentication) as `auth`, with its v2 access read, and the shipped
`providers/confluence/operations.json`. Then add this member, which names REST v1 as the
alternative, with its shipped selection set and the v1 space list as its access read:

```json
{
  "bindings": {
    "id": "v2",
    "next": {"status": 401, "reason": "scope does not match"},
    "alternatives": [
      {
        "id": "v1",
        "provider": "confluence-v1",
        "operations_file": "/absolute/path/adapters/catalog/providers/confluence-v1/operations.json",
        "access": {"path": "rest/api/space", "query": {"limit": "1"}}
      }
    ]
  }
}
```

Every read of both selection sets is a read, and a configuration with bindings serves no write.

### 2. Bind it to the CLI

Pin the instance as [Bind the provider to the local CLI](local-catalog-provider.md#bind-the-provider-to-the-local-cli)
shows, as its own instance beside any `confluence.work` or `confluence-v1.work`, for example
`[connectors.confluence.instances.either]` with the alias `confluence-either`. Its
`permissions.operations` names the ids of both bindings: `content.search`, `page.comments`,
`page.get`, `pages.changed`, `space.get`, `space.pages` and `spaces.list`. An instance that
existed before keeps its connections; the new one is connected on its own.

### 3. Connect and read

Connect with the credential document the other instances use. The connection names the binding
its validation selected, `"binding": "v1"` or `"binding": "v2"`:

```sh
connectors connections connect --connector confluence --instance either \
  --profile atlassian.basic --credential-file ~/private/atlassian.json
```

Describe the operations through that connection with `--connection`, and take `--schema` and
`--revision` from it: without `--connection` the instance is described by its first binding,
v2. An id the bound API does not serve, such as `spaces.list` on a v2-bound connection, is
`not_found` before any request.

```sh
connectors operations list --connector confluence --instance either --connection "$connection"
connectors operations describe --connector confluence --instance either \
  --connection "$connection" --operation page.get
connectors operations invoke --connector confluence --instance either \
  --connection "$connection" --operation page.get \
  --schema "$schema" --revision "$revision" --input-json '{"id":"1001"}'
```

The input follows the bound API: `page.get` takes `body-format` under v2 and `expand` under v1,
as the description's schema says.

When v2 and v1 both refuse, nothing connects, and the `insufficient_scope` refusal names both
answers: `binding v2 answered 401 (Unauthorized; scope does not match); binding v1 answered 403`.
A wrong password is refused at the identity read, and no other API is tried.

### 4. When the token changes

Each validation starts again from v2. A `connections revalidate` that finds v2 answering a token
whose scopes were extended binds the connection to v2 and says so:
`"rebound": {"from": "v1", "to": "v2"}`. A read whose revision was described under the earlier
binding is then `stale_description`; describe again with `--connection`.

```sh
connectors connections revalidate --connector confluence --instance either \
  --connection "$connection" --expected-revision "$connection_revision"
```

Verified against a local HTTPS fixture only (`adapters/catalog/tests/confluence_binding.rs`):
each binding's probes and reads, the refusals above, and the connect, describe, invoke and
rebind through the CLI. No live Confluence site has been read through such an instance.

## Limits

- Verified against a local HTTPS fixture only
  (`adapters/catalog/tests/confluence.rs`): the exact request of each read,
  including `body-format=storage` and the basic header, the returned body as
  JSON, a two-page walk of each list to a missing or null `_links.next`, and a
  `pages.changed` walk that stops at the cutoff while a further page is still
  linked. No live Confluence Cloud site has been read, and the identity read has
  not been run against one.
- [One instance for either API](#one-instance-for-either-api) needs a pinned native
  configuration: the shipped declarations name no bindings, so an instance configured by its
  name alone reads through the one API of its connector.
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
