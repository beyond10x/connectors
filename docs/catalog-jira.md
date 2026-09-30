# Jira Cloud through the catalog provider

The catalog provider reads Jira Cloud issues, their comments and their changelog
from the pinned Jira Cloud platform REST v3 OpenAPI document. Nothing here is
Jira-specific code: the pinned document is compiled into a bundle, a reviewed
selection set exposes three reads, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them.
Configuration, connection, approval and invocation work exactly as described
there; this page covers what differs for Jira.

## Source and bundle

The pinned source is
[`adapters/atlassian/upstream/jira-platform-v3.json`](../adapters/atlassian/upstream/README.md),
SHA-256 `655b4790a5c8543c81755c8250c6a8b8f58ee62712168a39d628d6d927549709`,
retrieved on 2026-09-29 from
<https://developer.atlassian.com/cloud/jira/platform/swagger-v3.v3.json>.
The bundle is built with:

```sh
cargo run --locked -p connectors-build -- catalog \
  --provider jira \
  --source adapters/atlassian/upstream/jira-platform-v3.json \
  --directory adapters/catalog/generated/bundles \
  --auth-profile atlassian.basic
```

It carries all 620 operations of the pinned document, none unsupported, and
`adapters/catalog/tests/bundle_drift.rs` refuses a committed bundle or index a
fresh run would not reproduce byte for byte.

## The shipped selection set

[`adapters/catalog/providers/jira/operations.json`](../adapters/catalog/providers/jira/operations.json)
exposes three reads and nothing else. Each is `effect: read`; there are no
writes. `adapters/catalog/tests/jira.rs` pins this exact id list and each id's
`operationId` and path in the pinned document, so a renamed or dropped id, or a
source operation that moved, fails the gate. The bundle refuses at load any
`operation_id` the pinned document lacks.

Every list returns one page per call. The provider returns `status`, `body` and
`provenance`; `body` is Jira's page unchanged, and the paging fields that decide
the end of a walk are in it.

| id | pinned `operationId` | request | paging parameters | end condition | time filter or deltas |
|---|---|---|---|---|---|
| `issues.search` | `searchAndReconsileIssuesUsingJql` | `GET /rest/api/3/search/jql` | `nextPageToken`, `maxResults` | `nextPageToken` absent or null in the response | JQL `updated >= "<t>"`, e.g. `project = FIX AND updated >= "2026-09-01 00:00" ORDER BY updated ASC` |
| `issue.comments` | `getComments` | `GET /rest/api/3/issue/{issueIdOrKey}/comment` | `startAt`, `maxResults` | `startAt + len(comments) >= total` | none; deltas come from `issues.search` |
| `issue.changelog` | `getChangeLogs` | `GET /rest/api/3/issue/{issueIdOrKey}/changelog` | `startAt`, `maxResults` | `isLast: true` | none; deltas come from `issues.search` |

- **`issues.search`** replaces the deprecated `GET /rest/api/3/search`. Send the
  first page without `nextPageToken`; for each following page, send the
  `nextPageToken` the previous page returned, with the same `jql`. The last
  page carries no `nextPageToken` or a null one (the pinned schema says "this
  token will be null"), and says `isLast: true`. Never send a null token back.
  The pinned document requires a bounded query: a JQL with a search
  restriction, not only an `ORDER BY`. Jira returns only issue ids unless
  `fields` is given; send it as one comma-separated string, such as
  `"summary,updated,status"` or `"*all"`. The pinned document states at most
  5,000 issues per page and a default `maxResults` of 50, and that Jira may
  return fewer items than asked, so a short page is not an end condition.
- **`issue.comments`** takes `issueIdOrKey` and starts at `startAt=0`. The next
  page starts at `startAt` plus the number of `comments` on this page; the walk
  ends when that reaches `total`. The pinned default `maxResults` is 100.
- **`issue.changelog`** takes `issueIdOrKey` and starts at `startAt=0`. The next
  page starts at `startAt` plus the number of `values` on this page; the walk
  ends on the page whose `isLast` is `true`. The pinned default `maxResults`
  is 100.

Neither comments nor changelog offers a time filter. To take deltas, search for
issues with `updated >= "<t>"` and re-read the comments and changelog of each
returned issue. Whether a given kind of change moves an issue's `updated` is
Jira's behaviour and has not been checked against a live site here.
Every other query parameter the pinned document declares for an operation is
accepted by name; one it does not declare is refused before any request.

## Authentication

Jira Cloud API tokens use HTTP basic with the account email. Declare the basic
profile `atlassian.basic`, which Confluence uses too: one Atlassian credential
connects both, as one `atlassian.account` identity (see
[the Confluence guide](catalog-confluence.md#authentication)). The identity read
is `GET /rest/api/3/myself`, whose `accountId` names the credential's subject.
Atlassian API tokens expose no scope list, so the profile declares no `scopes`
read and no `minimum_scopes`. The profile was `jira.basic` with identity kind
`jira.user` in 0.16 and 0.17; a configuration naming it must be changed and its
connection made again.

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "jira-cloud",
  "provider": "jira",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://your-domain.atlassian.net/rest/api/3",
  "auth": {
    "profile": "atlassian.basic",
    "scheme": "basic",
    "header": "Authorization",
    "bearer": false,
    "account_label": "Account email",
    "label": "API token",
    "identity": {"path": "myself", "kind": "atlassian.account", "subject_pointer": "/accountId"}
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/jira/operations.json"
}
```

`connections connect` takes the credential document
`{"account": "<email>", "token": "<API token>"}`; every request then carries
`Authorization: Basic base64(<email>:<API token>)`.

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/jira.rs`):
  the exact request of each read, including the JQL time filter and the basic
  header, the returned body as JSON, and a two-page walk of each list to the
  end condition above. No live Jira Cloud site has been read.
- The engine parses and re-serialises the body, so it is returned as equal JSON,
  not as Jira's exact bytes.
- `fields`, `properties` and `reconcileIssues` are declared as arrays by the
  pinned document; the engine sends one value per parameter, so pass a
  comma-separated string.
- The provider does not walk pages itself, does not bound `maxResults`, and does
  not retry on `429`; a rate-limited read is returned as a refusal.
