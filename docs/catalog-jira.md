# Jira Cloud through the catalog provider

The catalog provider reads Jira Cloud issues by JQL or one by key, their comments
and their changelog, the transitions open on an issue, the issue types a project
can create and the fields each takes, users and an attachment's content; it runs
one transition on an issue, and creates, edits, comments on, links and deletes
issues, from the pinned Jira Cloud platform REST v3 OpenAPI document. Nothing here is
Jira-specific code: the pinned document is compiled into a bundle, a reviewed
selection set exposes nine reads and six guarded writes, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them.
Connection, approval and invocation work as described there; this page
configures the `jira` connector and covers what differs for Jira. Connector,
instance and connection are defined in [the README](../README.md#vocabulary).

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
exposes nine reads and six writes: `issue.transition.run`, guarded as
[Running a transition](#running-a-transition) describes, and `issue.create`,
`issue.comment.add`, `issue.edit`, `issue.link.add` and `issue.delete`, guarded as
[Issue writes](#issue-writes) describes. `adapters/catalog/tests/jira.rs` pins this exact id list and each id's
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
| `issue.get` | `getIssue` | `GET /rest/api/3/issue/{issueIdOrKey}` | none | one issue per call | none; deltas come from `issues.search` |
| `issue.transitions` | `getTransitions` | `GET /rest/api/3/issue/{issueIdOrKey}/transitions` | none | one issue per call | none |
| `issue.create_meta` | `getCreateIssueMetaIssueTypes` | `GET /rest/api/3/issue/createmeta/{projectIdOrKey}/issuetypes` | `startAt`, `maxResults` | `startAt + len(issueTypes) >= total` | none |
| `issue.create_meta.fields` | `getCreateIssueMetaIssueTypeId` | `GET /rest/api/3/issue/createmeta/{projectIdOrKey}/issuetypes/{issueTypeId}` | `startAt`, `maxResults` | `startAt + len(fields) >= total` | none |
| `users.search` | `findUsers` | `GET /rest/api/3/user/search` | `startAt`, `maxResults` | an empty page | none |
| `attachment.content` | `getAttachmentContent` | `GET /rest/api/3/attachment/content/{id}` | none | one attachment per call | none; see [Attachment content](#attachment-content) |

- **`issues.search`** replaces the deprecated `GET /rest/api/3/search`. Send the
  first page without `nextPageToken`; for each following page, send the
  `nextPageToken` the previous page returned, with the same `jql`. The last
  page carries no `nextPageToken` or a null one (the pinned schema says "this
  token will be null"), and says `isLast: true`. Never send a null token back.
  The pinned document requires a bounded query: a JQL with a search
  restriction, not only an `ORDER BY`. Jira returns only issue ids unless
  `fields` is given; send it as a JSON array, such as
  `["summary", "updated", "status"]`, or as one comma-separated string, such as
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
- **`issue.get`** takes `issueIdOrKey`, such as `FIX-1`, and returns the one
  issue. Without `fields` Jira returns all fields; `fields` takes a JSON array or
  one comma-separated string, as on `issues.search`, and the field `attachment`
  carries the issue's attachment metadata, each attachment's `id` among it, which
  `attachment.content` reads. `expand` is one comma-separated string, such as
  `renderedFields,names`: `renderedFields` adds the HTML rendering of rich-text
  fields beside their Atlassian Document Format value, and `names` the display
  name of each field. An issue Jira does not find, or that the account may not
  see, is refused as the provider's not-found. The pinned document also declares
  `updateHistory`, which adds the issue's project to the account's recently
  viewed list; it is accepted by name like every declared parameter, so leave it
  out for a read with no such effect.
- **`issue.transitions`** takes `issueIdOrKey` and lists the transitions the
  account may run on that issue in its current status. Each transition carries
  its `id`, which a transition request names, its `name`, and in `to` the
  status it leads to (`id`, `name`, `statusCategory`); `isAvailable` says
  whether it passes its conditions now. The answer is not paged. The selection
  keeps all five optional parameters the pinned document declares; each only
  narrows, expands or orders the answer:
  - `transitionId` narrows the answer to that one transition, to check it
    before running it.
  - `expand=transitions.fields` adds each transition screen's fields, with
    `required` and `allowedValues`, the input a transition with a screen needs.
  - `includeUnavailableTransitions=true` adds transitions that fail a
    condition, with `isAvailable: false`, to show why one is not offered.
  - `sortByOpsBarAndStatus=true` orders by the issue view's ops bar, then by
    status category, instead of by ops bar alone.
  - `skipRemoteOnlyCondition=true` includes transitions hidden by the *Hide
    From User Condition*. The pinned document gives it effect only for Connect
    and Forge apps with the *Administer Jira* permission, which an account
    connection (basic or bearer) is not; it is accepted by name, like every
    declared parameter, and Jira decides whether it applies. This has not been
    checked against a live site.

  A parameter the pinned document does not declare for `getTransitions`, a
  missing `issueIdOrKey`, or a boolean given as anything but `true` or `false`
  is refused as `invalid_input` before any request. An issue Jira does not
  find, or that the account may not see, is refused as the provider's
  not-found.

  ```sh
  connectors operations invoke --connector jira --instance work --connection "$connection" \
    --operation issue.transitions --schema "$schema" --revision "$revision" \
    --input-json '{"issueIdOrKey":"FIX-1","expand":"transitions.fields"}'
  ```

  sends `GET /rest/api/3/issue/FIX-1/transitions?expand=transitions.fields`.
- **`issue.create_meta`** takes `projectIdOrKey` and lists the issue types the
  account can create in that project, the input a create request needs. The next
  page starts at `startAt` plus the number of `issueTypes`; the walk ends when
  that reaches `total`. The pinned default `maxResults` is 50 and its maximum
  200.
- **`issue.create_meta.fields`** takes `projectIdOrKey` and `issueTypeId`, the
  `id` of an issue type `issue.create_meta` lists, both required, and lists the
  fields that issue type takes on `issue.create`: each with `fieldId`, `name`,
  `required`, `schema` and, where the field has a closed set, `allowedValues`.
  The next page starts at `startAt` plus the number of `fields`; the walk ends
  when that reaches `total`. The pinned default `maxResults` is 50 and its
  maximum 200. A call without `issueTypeId` is refused as `invalid_input` before
  any request.
- **`users.search`** takes `query`, matched against the start of a user's
  display name or email address, or `accountId` for an exact match; the pinned
  document requires one of `query`, `accountId` or `property` without marking
  any of them required, so Jira, not the provider, refuses a call without one.
  The body is a bare array of users. The pinned document pages the first 1,000
  matches by `startAt` and `maxResults` (default 50) and states no total, so a
  walk sends `startAt` plus the number of users returned and ends on an empty
  page. Privacy controls may withhold a user's email address, and a caller
  without the *Browse users and groups* permission gets an empty array.

Neither comments nor changelog offers a time filter. To take deltas, search for
issues with `updated >= "<t>"` and re-read the comments and changelog of each
returned issue. Whether a given kind of change moves an issue's `updated` is
Jira's behaviour and has not been checked against a live site here.
Every other query parameter the pinned document declares for an operation is
accepted by name; one it does not declare is refused before any request.

## Attachment content

`attachment.content` selects `getAttachmentContent` as a binary read
([binary reads](local-catalog-provider.md#configure-the-provider)). It takes the
attachment's `id`, as `issue.get` lists it under `fields.attachment`, and answers
the bytes in base64 (`content_base64`) with their media type (`media_type`),
length (`length`) and SHA-256 digest. An attachment over 2 MiB is refused as
`capacity`, never truncated.

The pinned document gives two answers. With `redirect` set to `false`, Jira
answers `200` with the content itself. Otherwise, the default, Jira answers `303`
with a `Location` to a download URL. The selection follows that redirect only to
`https://api.media.atlassian.com`, without the connection's credential, and only
when the connection admits that host. An instance of the `jira` connector admits
no host besides its endpoint, so on it a call without `"redirect": false` is
refused as `forbidden` when Jira redirects, with nothing sent to the media host;
send `"redirect": false`. Only a pinned instance, whose native configuration
([the catalog provider guide](local-catalog-provider.md)) can state

```json
"hosts": [{"origin": "https://api.media.atlassian.com"}]
```

follows the redirect. A redirect to any other host is refused the same way.
Adding `hosts` changes the configuration revision.

**The media host is inferred, not verified live.** The pinned document names no
redirect host; `https://api.media.atlassian.com`, Atlassian's media download
host, is an inference, and no live site has been asked. If a site redirects
elsewhere, the read is refused by name and `redirect: false` still works.

## Running a transition

`issue.transition.run` selects `doTransition`
(`POST /rest/api/3/issue/{issueIdOrKey}/transitions`), a write that needs an
approval like every catalog write
([the catalog provider guide](local-catalog-provider.md#invoke)). It runs
one transition by its id; running one by name, or walking several transitions to
reach a status, is composition over `issue.transitions` and this write, and stays
outside the catalog.

Its input is `issueIdOrKey`, two status ids the caller reads first, and the body
Jira takes:

- `current_status`: the id of the status the issue must be in now, the
  `fields.status.id` of `issue.get`.
- `target_status`: the id of the status the transition leads to, the `to.id` of
  the transition in `issue.transitions`.
- `body`: closed to `transition`, `fields` and `update`. `transition.id` names
  the transition and is required; `fields` and `update` set the fields of the
  transition's screen, such as a resolution, as Jira's `IssueUpdateDetails`
  defines them (`expand=transitions.fields` on `issue.transitions` lists them).
  `historyMetadata` and `properties`, and any other key, are refused as
  `invalid_input` before any request.

The guard reads twice before the write and once after it:

1. `getIssue` on the issue: `/fields/status/id` must be `current_status`.
2. `getTransitions` on the issue with `transitionId` set to `transition.id`.
   Jira answers it with that one transition while it is open on the issue, and
   with an empty list otherwise. `/transitions/0/id` must be `transition.id`,
   `/transitions/0/isAvailable` `true` and `/transitions/0/to/id`
   `target_status`.
3. After Jira answers the `POST` with a `2xx` (`204`, with no body), `getIssue`
   on the issue again: `/fields/status/id` must be `target_status`.

An issue in another status, an issue Jira does not find, and a transition whose
target is another status are refused as `forbidden` with the `POST` unsent, the
outcome `not_attempted`. A transition that is not open on the issue is refused
as well, with the `POST` unsent, as `protocol`: Jira's narrowed answer has no
transition at `/transitions/0`. When the read after the write finds another
status, or fails, the outcome is `unknown`: Jira may have run the transition, and
a post function or another user may have moved the issue since. Nothing is sent
again. Jira's definite refusals of the `POST` itself are `refused`: the pinned
document gives `400` for a request it finds invalid, such as a field the
transition's screen does not include, and `401`, `404`, `409` and `422`. Its
`413` is not among the statuses the engine reads as definite, so it leaves the
outcome `unknown`.

```json
{"issueIdOrKey":"FIX-1","current_status":"3","target_status":"10002",
 "body":{"transition":{"id":"31"}}}
```

Prepare, issue and invoke the write with that input as
[the guarded merge guide](local-gitlab-merge.md) describes for a merge. Approved,
it sends `GET /rest/api/3/issue/FIX-1`,
`GET /rest/api/3/issue/FIX-1/transitions?transitionId=31`,
`POST /rest/api/3/issue/FIX-1/transitions` with the body as given, and
`GET /rest/api/3/issue/FIX-1`, and answers with Jira's `204` and a `null` body.
Verified against the local fixture `adapters/catalog/tests/jira_transition_run.rs`
in the pinned document's shapes, not against a live site.

## Issue writes

Five more writes act on issues. Each needs an approval like every catalog write
([the catalog provider guide](local-catalog-provider.md#invoke)), and each body is
closed: a key it does not list is refused as `invalid_input` before any request.

| id | pinned `operationId` | request | read before | read after |
|---|---|---|---|---|
| `issue.create` | `createIssue` | `POST /rest/api/3/issue` | `getProject` on `body.fields.project.key`: `/key` must be that key | none; Jira's `201` answers `id`, `key` and `self`, and `/id` and `/key` must be present (not `null`), else the outcome is `unknown` |
| `issue.comment.add` | `addComment` | `POST /rest/api/3/issue/{issueIdOrKey}/comment` | `getIssue`: `/key` must be `issueIdOrKey` | none; Jira's `201` answers the comment, whose `/body/type` must be `doc` and whose `/id` must be present (not `null`), else the outcome is `unknown` |
| `issue.edit` | `editIssue` | `PUT /rest/api/3/issue/{issueIdOrKey}` | `getIssue`: `/key` must be `issueIdOrKey` and `/fields/updated` `updated_stamp`, else `conflict` | `getIssue`: `/key` must be `issueIdOrKey` |
| `issue.link.add` | `linkIssues` | `POST /rest/api/3/issueLink` | `getIssue` on `body.inwardIssue.key`, then on `body.outwardIssue.key`: each `/key` must be the key named | none; Jira's `201` has no body |
| `issue.delete` | `deleteIssue` | `DELETE /rest/api/3/issue/{issueIdOrKey}` | `getIssue`: `/key` must be `issueIdOrKey` and `/fields/updated` `updated_stamp`, else `conflict` | `getIssue`: only a `404` proves the issue gone |

Rich text is an Atlassian Document Format (ADF) value the caller writes and the
provider sends as given: the description of `issue.create` and `issue.edit`, and
the content of a comment. Plain text or Markdown is not converted.

- **`issue.create`** takes `body.fields` with `issuetype.id` and `summary`
  (required, non-empty), `project.key` (non-empty; the read before needs it, so
  a create without it is refused as `invalid_input`), and optionally
  `description` (ADF), `labels` (a list of strings), `priority.id`,
  `assignee.accountId` and `parent.key` (non-empty). `issue.create_meta.fields`
  lists what an issue type takes. Nothing can be compared before a create: the
  same input approved and sent twice creates two issues.
- **`issue.comment.add`** takes `issueIdOrKey` and `body.body.content`, the
  content array of an ADF document; the provider sets `body.body.type` to `doc`
  and `body.body.version` to `1`, and a caller sending either is refused.
  `body.visibility.type` (`group` or `role`) and `body.visibility.value`
  restrict who sees the comment. A comment sent twice is two comments.
- **`issue.edit`** takes `issueIdOrKey`, `updated_stamp` (the `fields.updated`
  of `issue.get`) and `body.fields` with at least one of `summary` (non-empty),
  `description` (ADF), `labels` (replacing the issue's labels), `priority` and
  `assignee`, as Jira's `IssueUpdateDetails` defines them. `notifyUsers` is
  passed through; `overrideScreenSecurity`, `overrideEditableFlag`,
  `returnIssue` and `expand` are withheld. Jira's `409` is `conflict`, the
  write refused and never sent again. The read after the edit checks the key
  only, not the fields changed.
- **`issue.link.add`** takes `body.type.name` (required, such as `Blocks`),
  `body.inwardIssue.key` and `body.outwardIssue.key`. Jira answers `201` with no
  body, also for a link that already exists, and the link is not read after the
  write: `issue.get` with `fields` `issuelinks` shows it.
- **`issue.delete`** takes `issueIdOrKey` and `updated_stamp`. `deleteSubtasks`
  is withheld, so Jira refuses an issue with subtasks (`400`, in the pinned
  document). A `204` followed by a read that still finds the issue, or a read
  that fails, leaves the outcome `unknown`.

Name issues by key: every read before compares `/key` with the value given, so
an issue id is refused with nothing written, as `conflict` on `issue.edit` and
`issue.delete` (their first read refuses every difference so) and as
`forbidden` on the others. A project or issue Jira does not find is refused as
`forbidden`, the write unsent, the outcome `not_attempted`; a stale
`updated_stamp` is refused as `conflict`.

```json
{"issueIdOrKey":"PROJ-1","updated_stamp":"2026-10-01T09:00:00.000+0000",
 "body":{"fields":{"summary":"Edited summary"}}}
```

Prepare, issue and invoke a write with its input as
[the guarded merge guide](local-gitlab-merge.md) describes for a merge. Approved,
that edit sends `GET /rest/api/3/issue/PROJ-1`, `PUT /rest/api/3/issue/PROJ-1`
with the body as given, and `GET /rest/api/3/issue/PROJ-1`, and answers with
Jira's `204` and a `null` body. Verified against the local fixture
`adapters/catalog/tests/jira_issue_writes.rs` in the pinned document's shapes, not
against a live site.

## Authentication

Jira Cloud API tokens use HTTP basic with the account email, under the profile
`atlassian.basic`, which Confluence uses too: one Atlassian credential
connects both, as one `atlassian.account` identity (see
[the Confluence guide](catalog-confluence.md#authentication)). The identity read
is `GET /rest/api/3/myself`, whose `accountId` names the credential's subject.
Atlassian API tokens expose no scope list, so the profile declares no `scopes`
read and no `minimum_scopes`. The profile was `jira.basic` with identity kind
`jira.user` in 0.16 and 0.17; a configuration naming it must be changed and its
connection made again.

The `jira` connector declares that profile
([`adapters/catalog/providers/jira/connector.json`](../adapters/catalog/providers/jira/connector.json))
and no default endpoint: an instance names its site's REST v3 root. Add to the
host configuration (`~/.config/connectors/config.toml`, format
`connectors-local/5`):

```toml
[connectors.jira]
enabled = true

[connectors.jira.instances.work]
endpoint = "https://your-domain.atlassian.net/rest/api/3"
```

The bundle, the fifteen operations, the profile and the provider executable are
derived from the shipped connector:

```console
$ connectors instances list
{"adapters":[{"adapter":"jira.work","adapter_id":"catalog","connector":"jira","instance":"work","instance_id":"jira.work","restart":"never","startup":"on-demand"}],"source":"configuration"}
```

Connect with the credential document `{"account": "<email>", "token": "<API token>"}`
in an owner-only file; every request then carries
`Authorization: Basic base64(<email>:<API token>)`:

```sh
connectors connections connect --connector jira --instance work \
  --profile atlassian.basic --credential-file ~/private/atlassian.json
```

An installation configured before connectors were declared converts its
entries with `connectors setup migrate`, which keeps each instance id and
revision; the pinned form it writes is described in
[the catalog provider guide](local-catalog-provider.md).

### Through the API gateway

Service-account API tokens and OAuth 2.0 (3LO) access tokens are not accepted on
the site URL; they work only through the Atlassian API gateway,
`https://api.atlassian.com/ex/jira/<cloud id>/…`. Write the gateway URL as
`api_base` and name its gateway part in `request_prefix`. The cloud id is the
`cloudId` of `https://your-domain.atlassian.net/_edge/tenant_info`, or the `id`
of the site in `GET https://api.atlassian.com/oauth/token/accessible-resources`
for an OAuth token. The pinned paths (`/rest/api/3/…`) are checked against what
follows the prefix, and every request, the identity read `myself` included, goes
to the full `api_base`: `GET /ex/jira/<cloud id>/rest/api/3/myself`.

No connector declaration fixes `request_prefix`, so a gateway instance is not
derived: it is a pinned instance, whose native configuration file is written as
[the catalog provider guide](local-catalog-provider.md) describes, with the
site form's fields and these values.

A service-account API token uses the same `atlassian.basic` profile as on the
site, with the service account's email as the account. Only `api_base` and
`request_prefix` change:

```json
{
  "api_base": "https://api.atlassian.com/ex/jira/your-cloud-id/rest/api/3",
  "request_prefix": "/ex/jira/your-cloud-id"
}
```

The credential document is `{"account": "<service account email>", "token": "<API token>"}`,
sent as `Authorization: Basic base64(<email>:<API token>)`. This form was checked
live on 2026-09-30: a service-account API token connected through the gateway
and `issues.search` returned issues. A user's scoped API token takes the same
form with the user's email.

An OAuth 2.0 (3LO) access token is sent as a bearer token, under a profile the
`jira` connector does not declare. **Not verified live.**

```json
{
  "api_base": "https://api.atlassian.com/ex/jira/your-cloud-id/rest/api/3",
  "request_prefix": "/ex/jira/your-cloud-id",
  "auth": {
    "profile": "atlassian.bearer",
    "header": "Authorization",
    "bearer": true,
    "label": "Atlassian access token",
    "identity": {"path": "myself", "kind": "atlassian.account", "subject_pointer": "/accountId"}
  }
}
```

The credential document is `{"token": "<access token>"}`, sent as
`Authorization: Bearer <access token>`. The provider does not refresh an OAuth
token; connect again when it expires. The token must carry the read scopes the
nine reads and `myself` need, and the write scopes the six writes need; Atlassian names them per
operation.

## Limits

- `attachment.content` is verified through the engine against synthetic answers
  (`adapters/catalog/tests/jira_attachment.rs`): the direct `200` with
  `redirect=false`, the `303` followed to the admitted media host with no
  credential, refusals for an unadmitted or other host, and an attachment over
  2 MiB refused as `capacity`. The media host is inferred
  ([Attachment content](#attachment-content)).
- Verified against a local HTTPS fixture (`adapters/catalog/tests/jira.rs`):
  the exact request of each read, including the JQL time filter and the basic
  header, the returned body as JSON, and a walk of each list to the end
  condition above. The fixture answers of `issue.get`, `issue.transitions`,
  `issue.create_meta` and `users.search` are written by hand in the shapes the
  pinned document gives (`IssueBean`, `Transitions`, `PageOfCreateMetaIssueTypes`,
  an array of `User`), not recorded from a site. Live, only the basic gateway form has been run: a
  service-account API token connected and `issues.search` returned issues
  (2026-09-30). The site form, the other reads and the OAuth form have not been
  run against a live site.
- The six writes are verified against local fixtures only
  (`adapters/catalog/tests/jira_transition_run.rs`,
  `adapters/catalog/tests/jira_issue_writes.rs`), in the pinned document's
  shapes. Rich text is ADF the caller writes; no plain-text or Markdown
  conversion is offered. `issue.create` and `issue.comment.add` cannot be
  compared before they run, so a repeated input creates again, and
  `issue.link.add` is not read back. A delete's approval binds the input it was
  issued for, the key and `updated_stamp` among it, like every catalog write.
- The engine parses and re-serialises the body, so it is returned as equal JSON,
  not as Jira's exact bytes.
- `fields`, `properties` and `reconcileIssues` on `issues.search` are declared
  as arrays by the pinned document. Each takes a JSON array, sent as one
  `name=value` pair per element in the order given (`reconcileIssues` elements
  are integers), or one comma-separated string, sent as one pair as before;
  for `reconcileIssues` that string must list integers, such as `"10001,10002"`,
  or it is refused as `invalid_input` before any request. See
  [array and required query parameters](local-catalog-provider.md#array-and-required-query-parameters).
- The provider does not walk pages itself and does not bound `maxResults`. It
  sends a read answered `429` once more after
  the delay its `Retry-After` names, when that wait leaves the second request
  time of its own before the invocation deadline, and otherwise returns it as a `rate_limited` refusal carrying that
  delay as `retry_after_seconds`
  ([Limits](local-catalog-provider.md#limits)).
