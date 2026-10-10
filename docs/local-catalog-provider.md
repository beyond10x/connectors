# GitLab through the catalog provider

The catalog provider runs ordinary HTTP operations straight from a pinned OpenAPI
document. There is no Rust per endpoint: the pinned source is compiled into a
digest-verified bundle, a selection set says which operations to expose and what
each one is allowed to do, and one engine binds, sends and classifies. Adding a
supported endpoint changes the selection; it changes no code.

This is the `provider` realization of
[the catalog design](../adapters/catalog/design.md) and applies
`architecture-decision-record:declarative-http-provider-runtime`. The host keeps
what it already owns: connection admission, approval policy and proofs, audit,
the attempt ledger, dispatch and recovery. The provider is one more adapter
executable behind the same private protocol as the Kubernetes adapter.

## Build the bundle

```sh
cargo run --locked -p connectors-build -- catalog \
  --provider gitlab \
  --source adapters/gitlab/upstream/openapi_v3.yaml \
  --directory adapters/catalog/generated/bundles \
  --auth-profile gitlab.pat
```

The pipeline reads the source (JSON or YAML), records its SHA-256, extracts the
operation inventory and writes `gitlab.bundle.json` plus `index.json`. The
committed GitLab bundle carries every one of the 1,847 operations in the pinned
source. It names 67 gaps, each an array query parameter the source declares
`explode: false` (such as `labels` and `iids` on issues and merge requests);
each of those is sent as the one value a caller gives, such as `"bug,ui"`, as it
was before arrays were read (see [Array and required query
parameters](#array-and-required-query-parameters)). `adapters/catalog/tests/bundle_drift.rs` refuses a
committed bundle that a fresh run would not reproduce byte for byte. Nothing
here reaches the network.

`--amendments <file>` applies a `connectors-source-amendments/1` file to the
extracted inventory: each entry names an `operation_id`, exactly one change, the
https page that documents it and why the pinned document needs it. The change
is one of:

- `add_parameter`: one optional query parameter to add (`name`,
  `location: query`, `required: false`, `type`) that the pinned document lacks;
- `correct_path`: `{"from": "<path>", "to": "<path>"}`, where `from` is the
  operation's path as the source declares it and `to` is that path with each
  parenthesised optional segment either removed or kept without its
  parentheses, such as GitLab's `/api/v4/projects/{id}/(-/)search` to
  `/api/v4/projects/{id}/search`; a segment holding a path parameter can only be
  kept. The method and every path parameter stay as
  declared;
- `correct_media_type`: `{"from": "<media type>", "to": "application/json"}`,
  where `from` is the one request media type the operation declares, for a
  write the vendor documents with a JSON body while its document declares only
  a form media type, such as GitLab's commit create, which the pinned document
  declares `multipart/form-data`. `to` can only be `application/json`, the one
  body the engine sends, so the correction makes the bundle say what the engine
  already does. The method, path and parameters stay as declared.

The file must carry the source's SHA-256. An operation that is missing or
declared twice, an entry with more than one change or none, a citation that is
not https or a blank reason, a parameter the operation already declares or
anything other than an optional, unrepeated query parameter, a path correction
whose `from` is not the operation's current path or whose `to` equals `from` or
is not derived from it that way, and a media type correction for an operation
that does not declare exactly one request media type, whose `from` is not that
one or whose `to` is not `application/json` are each refused before any bundle
is written, and nothing is applied. The bundle's source record then names the file and its
SHA-256 (`crates/connectors-catalog/src/amendment.rs`; the format is modelled as
`connectors_catalog.amendment` in `adapters/catalog/spec/ess`). Zendesk uses it
for the ticket export's `per_page` ([Zendesk guide](catalog-zendesk.md#source-and-bundle)),
GitLab for the project search path ([Project code search](#project-code-search))
and for the JSON bodies of commit create, file update and project create
([Pipeline, commit, branch and project writes](#pipeline-commit-branch-and-project-writes)).
The GitLab build passes it:

```sh
cargo run --locked -p connectors-build -- catalog \
  --provider gitlab \
  --source adapters/gitlab/upstream/openapi_v3.yaml \
  --directory adapters/catalog/generated/bundles \
  --auth-profile gitlab.pat \
  --amendments adapters/gitlab/upstream/openapi_v3.amendments.json \
  --replace
```

## The shipped selection set

The repository reviews and ships one selection set per provider under
`adapters/catalog/providers/<provider>/operations.json`; Jira Cloud is described
in [the Jira guide](catalog-jira.md), Confluence Cloud in
[the Confluence guide](catalog-confluence.md), HubSpot CRM in
[the HubSpot guide](catalog-hubspot.md), Zendesk Support in
[the Zendesk guide](catalog-zendesk.md), Runpod pods in
[the Runpod guide](catalog-runpod.md) and Slack conversations in
[the Slack guide](catalog-slack.md). The GitLab set,
[operations.json](../adapters/catalog/providers/gitlab/operations.json), exposes
every operation the retired native GitLab adapter exposed, so one configuration
serves the provider from the pinned source alone, plus eleven repository reads
and one unguarded write, `issue.create`, the merge-request note and discussion
operations ([the guarded merge guide](local-gitlab-merge.md#notes-and-discussions)), the
auto-merge and reopen variants of merge and update
([Auto-merge and reopen](local-gitlab-merge.md#auto-merge-and-reopen)), two
merge-request list reads, its diffs and its discussions, project code
search, tags and releases ([below](#tags-and-releases)), and the environment list
and the pipeline, commit, file, branch and project writes
([below](#pipeline-commit-branch-and-project-writes)):

| id | source operation | effect |
|---|---|---|
| `project.get`, `issues.list`, `file.get`, `branch.get` | the matching `getApiV4Projects…` | read |
| `merge_requests.list`, `merge_request.get` | `getApiV4ProjectsIdMergeRequests`, `…MergeRequestIid` | read |
| `pipelines.list`, `pipeline.get`, `pipeline.jobs`, `job.get` | the matching `getApiV4Projects…` | read |
| `job.trace` | `getApiV4ProjectsIdJobsJobIdTrace`, `"response": "text"` | read |
| `issue.create` | `postApiV4ProjectsIdIssues`, unguarded | write |
| `merge_request.create` | `postApiV4ProjectsIdMergeRequests`, guarded | write |
| `merge_request.update` | `putApiV4ProjectsIdMergeRequestsMergeRequestIid`, guarded | write |
| `merge_request.merge` | `putApiV4ProjectsIdMergeRequestsMergeRequestIidMerge`, guarded | write |
| `merge_request.auto_merge`, `merge_request.reopen` | `…MergeRequestIidMerge` and `…MergeRequestIid`, guarded, each with one fixed body value ([Auto-merge and reopen](local-gitlab-merge.md#auto-merge-and-reopen)) | write |
| `merge_request.discussion.get` | `getApiV4ProjectsIdMergeRequestsNoteableIdDiscussionsDiscussionId` | read |
| `merge_request.diffs`, `merge_request.discussions` | `getApiV4ProjectsIdMergeRequestsMergeRequestIidDiffs`, `getApiV4ProjectsIdMergeRequestsNoteableIdDiscussions`, each one page per call ([below](#merge-request-diffs-and-discussions)) | read |
| `search.blobs` | `getApiV4ProjectsIdDashSearch`, path corrected by a cited amendment, `scope` held to `blobs` ([below](#project-code-search)) | read |
| `merge_request.note.create`, `merge_request.discussion.reply` | `postApiV4ProjectsIdMergeRequestsNoteableIdNotes`, `…DiscussionsDiscussionIdNotes`, unguarded | write |
| `merge_request.discussion.resolve` | `putApiV4ProjectsIdMergeRequestsNoteableIdDiscussionsDiscussionId`, guarded | write |
| `tag.get`, `release.get`, `release.links` | `getApiV4ProjectsIdRepositoryTagsTagName`, `getApiV4ProjectsIdReleasesTagName`, `getApiV4ProjectsIdReleasesTagNameAssetsLinks` ([below](#tags-and-releases)) | read |
| `tag.create`, `tag.delete` | `postApiV4ProjectsIdRepositoryTags`, `deleteApiV4ProjectsIdRepositoryTagsTagName`, guarded on the pinned commit | write |
| `release.create`, `release.update` | `postApiV4ProjectsIdReleases`, `putApiV4ProjectsIdReleasesTagName`, guarded on the pinned commit | write |
| `environments.list` | `getApiV4ProjectsIdEnvironments`, one page per call ([below](#pipeline-commit-branch-and-project-writes)) | read |
| `pipeline.retry`, `pipeline.cancel`, `pipeline.create` | `postApiV4ProjectsIdPipelinesPipelineIdRetry`, `…Cancel`, `postApiV4ProjectsIdPipeline`, guarded on the pinned commit | write |
| `commit.create`, `file.update` | `postApiV4ProjectsIdRepositoryCommits`, `putApiV4ProjectsIdRepositoryFilesFilePath`, guarded on the branch head, JSON body by a cited amendment | write |
| `branch.create`, `branch.delete` | `postApiV4ProjectsIdRepositoryBranches`, `deleteApiV4ProjectsIdRepositoryBranchesBranch`, guarded on the pinned commit | write |
| `project.create` | `postApiV4Projects`, guarded on the namespace, JSON body by a cited amendment | write |

The GitLab set also declares the merge request feed, a `datasource.feed/v1alpha1`
binding under profile `gitlab-merge-requests/1`: `feed.containers` lists the
projects the token's user is a member of (`getApiV4Projects`, continued after
the last project id with `id_after`), and `feed.items` reads one project's merge
requests changed since a watermark (`getApiV4ProjectsId`, then
`getApiV4ProjectsIdMergeRequests` with `updated_after`). Both are reads. Every
project is listed `private`, every merge request has a null `url`, and deleted
merge requests are not observed; the profile statement is
[gitlab.md](../adapters/catalog/contracts/feed/v1alpha1/gitlab.md). Add both ids
to an adapter's permitted operations to read the feed;
`operations list --family datasource.feed/v1alpha1` then names them.

The repository reads other than `repository.compare` and `commit.get` list one page per call. Each takes `page` and `per_page`;
a caller has walked the list when a page comes back shorter than `per_page`.
GitLab serves at most 100 items per page, so with a larger value every page
would be short and the walk would stop after page one. The pinned source
declares no range, so the shipped selection bounds `per_page` to 1 through 100
on every list read (`issues.list`, `merge_requests.list`, `pipelines.list`,
`pipeline.jobs`, `merge_request.diffs`, `merge_request.discussions`, `release.links`, `environments.list` and these nine): a value of zero or below never ends a walk on a
short page. A value outside that range, or one that is not an integer, is
refused as `invalid_input` before any request. The provider returns `status`, `body` and `provenance`, not GitLab's
`X-Next-Page` header. Every other query parameter the pinned source declares,
except those a selection withholds (`commits.list` and `repository.tree` withhold
`pagination` and `page_token`, `branches.list` withholds `page_token`), is accepted by name, with the type the pinned source gives it: an integer as a
JSON integer or its decimal string (`100` or `"100"`), a boolean as `true` or
`false` (or those two strings), a string as any string or a JSON integer (sent as
its decimal text); any other value, such as `per_page=true` or `page=2.0`, is
refused as `invalid_input` before any request.

| id | source operation | request | time filter | effect |
|---|---|---|---|---|
| `projects.list` | `getApiV4Projects` | `GET /projects`, e.g. `membership`, `simple=false`, `archived`, `order_by=last_activity_at` | `last_activity_after` | read |
| `tags.list` | `getApiV4ProjectsIdRepositoryTags` | `GET /projects/{id}/repository/tags` | none; each tag carries its commit id | read |
| `releases.list` | `getApiV4ProjectsIdReleases` | `GET /projects/{id}/releases` | none; each release carries `released_at` | read |
| `project.events` | `getApiV4ProjectsIdEvents` | `GET /projects/{id}/events` | `after`, `before` (dates) | read |
| `commits.list` | `getApiV4ProjectsIdRepositoryCommits` | `GET /projects/{id}/repository/commits`, e.g. `ref_name`, `first_parent`; each commit carries `parent_ids`; offset paging (`page`, `per_page`) only: the selection withholds `pagination` and its keyset cursor `page_token`, so an input carrying either (`pagination=keyset` included) is refused as `invalid_input` before any request, because the provider does not follow GitLab's `Link` header; `read_api` scope | `since`, `until`: a string that GitLab reads as an ISO 8601 date-time | read |
| `repository.compare` | `getApiV4ProjectsIdRepositoryCompare` | `GET /projects/{id}/repository/compare` with `from` and `to` (both required) and `straight`; not paged; `read_api` scope. The body is GitLab's own: an empty compare answers `200` with `"commits": []` and `"compare_timeout": false`, a compare GitLab cut short also answers `200`, possibly with `"commits": []`, but with `"compare_timeout": true`; so read `body.compare_timeout`, not the length of `commits`, to tell them apart. GitLab always includes `diffs`, and a body over the provider's 4 MiB response limit answers `capacity` with no body at all; to place commits between two tags, walk `commits.list` instead, which carries no diffs | none | read |
| `deployments.list` | `getApiV4ProjectsIdDeployments` | `GET /projects/{id}/deployments`, e.g. `environment`, `status`, `order_by`, `sort`; the body is GitLab's own, so each deployment carries `environment.name` and `deployable`, the job that ran it (`deployable.id`); a project the token cannot read answers GitLab's `403` or `404` with the refusal the other list reads answer; `read_api` scope | `updated_after`, `updated_before`, `finished_after`, `finished_before`: strings that GitLab reads as ISO 8601 date-times | read |

| `repository.tree` | `getApiV4ProjectsIdRepositoryTree` | `GET /projects/{id}/repository/tree`, e.g. `ref`, `path`, `recursive=true`; each entry carries `id`, `name`, `type` (`tree` or `blob`), `path` and `mode`; offset paging only: the selection withholds `pagination` and `page_token`, so `pagination=keyset` is refused as `invalid_input` before any request, as on `commits.list`. Example: `{"id": "org/project", "ref": "main", "path": "src", "recursive": true, "per_page": 100, "page": 1}` sends `GET /projects/org%2Fproject/repository/tree?ref=main&path=src&recursive=true&page=1&per_page=100` | none | read |
| `commit.get` | `getApiV4ProjectsIdRepositoryCommitsSha` | `GET /projects/{id}/repository/commits/{sha}`, `sha` a commit id, branch or tag name, sent as one path segment (`release/v0.3` is encoded, never a second segment); `stats=true` adds line counts; not paged. A sha GitLab does not find answers its `404`, refused as `not_found` | none | read |
| `commit.diff` | `getApiV4ProjectsIdRepositoryCommitsShaDiff` | `GET /projects/{id}/repository/commits/{sha}/diff`, `unidiff`; each entry is GitLab's diff with `new_path`, `old_path`, `diff` and the `new_file`, `renamed_file`, `deleted_file` flags and GitLab's `collapsed` and `too_large` flags, unchanged. A sha GitLab does not find is refused as `not_found` | none | read |
| `branches.list` | `getApiV4ProjectsIdRepositoryBranches` | `GET /projects/{id}/repository/branches`, e.g. `search`, `regex`, `sort` (`name_asc`, `updated_asc`, `updated_desc`); each branch carries its head `commit`, `default`, `protected` and `merged`; offset paging only: the selection withholds `page_token`, so an input carrying it is refused as `invalid_input` before any request | none | read |

`projects.list` returns each project unchanged, including `archived`,
`created_at`, `last_activity_at` and `path_with_namespace`. All eleven need only
the `read_api` token scope.

### Project code search

`search.blobs` selects `getApiV4ProjectsIdDashSearch`, GitLab's project search,
held to code: it needs `read_api` and sends
`GET /projects/{id}/search?scope=blobs&search=…`, one page per call.

| id | source operation | request | effect |
|---|---|---|---|
| `search.blobs` | `getApiV4ProjectsIdDashSearch` | `GET /projects/{id}/search` with `search` (the expression, required), `scope` (required, and only `blobs`), `ref` (a branch or tag), `page` and `per_page` (1 through 100). The body is GitLab's list of blob matches, unchanged, such as `path`, `ref`, `startline` and `data` (the matching lines). Example: `{"id": "org/project", "scope": "blobs", "search": "connect(", "ref": "main", "per_page": 20, "page": 1}` sends `GET` to the segments `projects`, `org/project`, `search` with the query pairs `scope=blobs`, `search=connect(`, `ref=main`, `per_page=20` and `page=1` | read |

The pinned source declares the path as `/api/v4/projects/{id}/(-/)search`,
GitLab's notation for an optional `-/` segment, which the engine would send
literally. The pinned document stays as GitLab published it; the cited
`adapters/gitlab/upstream/openapi_v3.amendments.json` corrects the bundle's
path to `/api/v4/projects/{id}/search`, the route GitLab's search reference
documents (see `--amendments` in [Build the bundle](#build-the-bundle)). The
same notation on group search and project semantic search is not corrected,
and neither is selected.

The selection bounds `scope` to the one value `blobs`, so the search cannot
reach issues, merge requests, commits, notes, wiki text or users: any other
scope, a case or whitespace variant, a comma-joined list or an absent `scope` is
refused as `invalid_input` before any request, and the declared input schema
lists `scope` with `"enum": ["blobs"]`. It withholds `type`, `state`,
`confidential` and `fields`, which apply only to other scopes, so an input
carrying any of them is refused the same way.

### Merge-request diffs and discussions

Two reads list what a merge request holds, both by the merge request's
project-local IID, one page per call, with `page` and `per_page` (1 through
100) as on the list reads above, and only the `read_api` token scope:

| id | source operation | request | effect |
|---|---|---|---|
| `merge_request.diffs` | `getApiV4ProjectsIdMergeRequestsMergeRequestIidDiffs` | `GET /projects/{id}/merge_requests/{merge_request_iid}/diffs`, `unidiff=true` for the unified diff format; each entry is GitLab's file diff, unchanged: `old_path`, `new_path`, `diff`, the `new_file`, `renamed_file` and `deleted_file` flags, and GitLab's `collapsed` and `too_large` flags. Example: `{"id": "org/project", "merge_request_iid": 7, "unidiff": true, "per_page": 100, "page": 1}` sends `GET /projects/org%2Fproject/merge_requests/7/diffs?page=1&per_page=100&unidiff=true` | read |
| `merge_request.discussions` | `getApiV4ProjectsIdMergeRequestsNoteableIdDiscussions` | `GET /projects/{id}/merge_requests/{noteable_id}/discussions`, `noteable_id` the IID, as on the discussion routes of [the guarded merge guide](local-gitlab-merge.md#notes-and-discussions); each discussion carries its `id`, `individual_note`, `resolvable`, `resolved` and its `notes`, unchanged, system notes included. Its `id` is the `discussion_id` that `merge_request.discussion.get`, `.reply` and `.resolve` take | read |

A merge request GitLab does not find answers its `404`, refused as
`not_found`; one the token may not read answers `403`, refused as `forbidden`.
The flat note list (`getApiV4ProjectsIdMergeRequestsNoteableIdNotes`) is not
selected: every note is in the discussion list, inside its discussion.

### Tags and releases

Three reads, each by tag name sent as one path segment (`release/v2` is encoded,
never a second segment), and four writes, each guarded on a commit `sha` the
caller pins as a top-level input. The reads need `read_api`; the writes need
`api` and at least GitLab's Developer role. Each write's body is closed to the
keys of the pinned request schema, its string keys typed `string`; any other
key, or a missing `sha`, is refused as `invalid_input` before any request.

| id | source operation | request | effect |
|---|---|---|---|
| `tag.get` | `getApiV4ProjectsIdRepositoryTagsTagName` | `GET /projects/{id}/repository/tags/{tag_name}`; GitLab's tag, unchanged: `name`, `message`, `target`, `commit`, `release`, `protected` | read |
| `release.get` | `getApiV4ProjectsIdReleasesTagName` | `GET /projects/{id}/releases/{tag_name}`, `include_html_description`; the release with its `commit`, `description` and `assets` | read |
| `release.links` | `getApiV4ProjectsIdReleasesTagNameAssetsLinks` | `GET /projects/{id}/releases/{tag_name}/assets/links`, one page per call, `per_page` 1 through 100; each link's `id`, `name`, `url`, `direct_asset_url` and `link_type` | read |
| `tag.create` | `postApiV4ProjectsIdRepositoryTags` | body `tag_name`, `ref` (a branch, tag or commit) and `message` (annotates). The guard reads `GET …/repository/commits/{body.ref}` and refuses unless its `/id` is `sha`; GitLab's answer must name `/name` `body.tag_name` at `/commit/id` `sha`. Example: `{"id": "org/project", "sha": "<head of main>", "body": {"tag_name": "v1.2.0", "ref": "main"}}` | write |
| `tag.delete` | `deleteApiV4ProjectsIdRepositoryTagsTagName` | The guard reads the tag and refuses unless its `/name` is `tag_name` and its `/commit/id` is `sha`; GitLab answers `204` with no body, so the guard reads the tag again and the delete is applied only when that read answers `404` (`postflight.absent`). GitLab has no conditional delete, so a tag deleted and recreated at another commit between the first read and the delete is deleted and still reported applied | write |
| `release.create` | `postApiV4ProjectsIdReleases` | body `tag_name`, `name`, `description`, `released_at`, `milestones`, `milestone_ids`, `assets`. The guard reads the tag `body.tag_name` and refuses unless its `/commit/id` is `sha`; GitLab's answer must name `/tag_name` and `/commit/id` the same. The body admits neither `ref` nor `tag_message`, so the release is of an existing tag and never creates one: create the tag with `tag.create` first | write |
| `release.update` | `putApiV4ProjectsIdReleasesTagName` | body `name`, `description`, `released_at`, `milestones`, `milestone_ids`, each replacing the stored value. The guard reads the release and refuses unless its `/tag_name` is `tag_name` and its `/commit/id` is `sha`, and the answer must carry both | write |

A guard read that GitLab answers `404` refuses the write before dispatch. A
differing value after the write (a ref moved between the read and the create,
say) leaves the outcome `unknown`, never refused, and no corrective request is
sent. A tag that already exists is GitLab's own `400`, a second release of one
tag its `409`; both are definite refusals. Release deletion and the release-link
writes are not selected.

### Pipeline, commit, branch and project writes

One read and eight writes. `environments.list` needs `read_api` and lists one
page per call with `page` and `per_page` (1 through 100); the writes need `api`.
Each write but `project.create` is guarded on a commit `sha` the caller pins as a
top-level input; `project.create` is guarded on a namespace. Each write's body
is closed to the keys listed below, its scalar keys typed as the selection
declares them (`body_types`); any other key, a missing required one or, where
the write is guarded on it, a missing `sha` is refused as `invalid_input` before
any request.

| id | source operation | request | effect |
|---|---|---|---|
| `environments.list` | `getApiV4ProjectsIdEnvironments` | `GET /projects/{id}/environments`, `name` (one by name), `search` (at least three characters), `states` (`available`, `stopping`, `stopped`); GitLab's environments, unchanged | read |
| `pipeline.retry`, `pipeline.cancel` | `postApiV4ProjectsIdPipelinesPipelineIdRetry`, `postApiV4ProjectsIdPipelinesPipelineIdCancel` | `POST /projects/{id}/pipelines/{pipeline_id}/retry` or `/cancel`, no body. The guard reads the pipeline and refuses unless its `/id` is `pipeline_id` and its `/sha` is `sha`; the answer must name the same. A pipeline with nothing to retry, or one already finished, is answered unchanged: read the answer's `status` | write |
| `pipeline.create` | `postApiV4ProjectsIdPipeline` | body `ref` (a branch or tag), `variables`, `inputs`. The guard reads `GET …/repository/commits/{body.ref}` and refuses unless its `/id` is `sha`; the answer must name `/ref` `body.ref` at `/sha` `sha`. Not idempotent: the same input sent again runs a second pipeline | write |
| `commit.create` | `postApiV4ProjectsIdRepositoryCommits` | body `branch`, `commit_message` and `actions` (both required), `author_email`, `author_name`, `stats`. The guard reads the branch and refuses unless its `/commit/id` is `sha`; the new commit's `/parent_ids/0` must be `sha`. The body admits no `start_branch`, `start_sha`, `start_project` or `force`, so it never creates a branch or rewrites history; the same input sent again is refused by the guard, the head being the new commit | write |
| `file.update` | `putApiV4ProjectsIdRepositoryFilesFilePath` | `file_path` sent as one path segment (`docs/guide.md` as `docs%2Fguide.md`), body `branch`, `content` and `commit_message` (both required), `encoding`, `author_email`, `author_name`, `last_commit_id`. The guard reads the branch and refuses unless its head is `sha`. GitLab answers only `file_path` and `branch`, so the provider then reads the branch again, `GET …/repository/branches/{body.branch}`, and the write is applied only when its `/commit/parent_ids/0` is `sha`; that read answering anything else (`500`, `403`, `410`) leaves the outcome `unknown`. It never reads `…/repository/commits/{body.branch}`, which GitLab resolves as a git revision, a same-named tag before the branch. Content identical to the file's is GitLab's `400` | write |
| `branch.create` | `postApiV4ProjectsIdRepositoryBranches` | body `branch`, `ref` (a branch, tag or commit). The guard reads the commit `body.ref` names and refuses unless its `/id` is `sha`; the answer must name `/name` `body.branch` at `/commit/id` `sha`. A branch that exists is GitLab's `400` | write |
| `branch.delete` | `deleteApiV4ProjectsIdRepositoryBranchesBranch` | The guard reads the branch and refuses unless its `/name` is `branch` and its `/commit/id` is `sha`; GitLab answers `204`, so the guard reads the branch again and the delete is applied only when that read answers `404` (`postflight.absent`). A branch moved between the first read and the delete is deleted and still reported applied, as on `tag.delete`. GitLab refuses the default and protected branches itself | write |
| `project.create` | `postApiV4Projects` | top-level `namespace` (the namespace's full path), body `name`, `path`, `namespace_id`, `description`, `visibility`, `default_branch`, `initialize_with_readme`. The guard reads `GET /namespaces/{namespace}` and refuses unless its `/id` is `body.namespace_id` and its `/full_path` is `namespace`; the answer must name `/path` `body.path` in `/namespace/id` `body.namespace_id`. The body admits no other setting, `import_url` and templates included. A path already taken is GitLab's `400` | write |

The pinned document declares the bodies of `commit.create`, `file.update` and
`project.create` only as `multipart/form-data`; GitLab's commits, repository
files and projects references document them as JSON. The cited
`adapters/gitlab/upstream/openapi_v3.amendments.json` corrects each one's request
media type in the bundle to `application/json` (`correct_media_type` in
[Build the bundle](#build-the-bundle)), and the engine sends it with
`Content-Type: application/json`. As on the tag and release writes, a guard
read GitLab answers `404` refuses the write before dispatch, and a differing
value after the write leaves the outcome `unknown`, with no corrective request.
`adapters/catalog/tests/gitlab_ci_project_writes.rs` holds each of these against
a local HTTPS fixture; none has run against a live GitLab.

`adapters/catalog/tests/shipped.rs` loads this file against the committed bundle
and pins the complete list of shipped ids, so a renamed, dropped or added id
fails the gate. The native
`merge_request.validate` has no entry: it was `merge_request.get` plus
`pipeline.get` and a comparison, which the merge guard now makes itself.

## Configure the provider

Build the executable and write an owner-only native configuration:

```sh
CARGO_BUILD_JOBS=2 cargo build --release --locked -p connectors -p connectors-catalog-provider
sha256sum target/release/connectors-catalog-provider
```

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "gitlab-sandbox",
  "provider": "gitlab",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://gitlab.example/api/v4",
  "ca_file": "/absolute/private/ca.crt",
  "auth": {
    "profile": "gitlab.pat",
    "header": "PRIVATE-TOKEN",
    "bearer": false,
    "label": "GitLab personal access token",
    "identity": {"path": "user", "kind": "gitlab.user", "subject_pointer": "/id"},
    "scopes": {"path": "personal_access_tokens/self", "pointer": "/scopes"},
    "minimum_scopes": ["api"]
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/gitlab/operations.json"
}
```

The complete configuration used against the sandbox is
[gitlab-catalog-2.json](evidence/gitlab-sandbox-20260913/gitlab-catalog-2.json).

- `auth` is the whole authentication profile: the header the token travels in,
  the read that names the credential's subject, an optional read that lists its
  granted scopes, and the scopes the profile requires. The token itself enters
  through the usual protected entry and custody; the file never holds it.
- `auth.scheme` is `token` when omitted: the protected entry is `{"token":"..."}`
  and travels in `header`, prefixed `Bearer ` when `bearer` is true. `basic` is
  HTTP basic for providers whose API tokens are used with an account name: the
  protected entry is `{"account":"...","token":"..."}` and every request carries
  `Authorization: Basic base64(account:token)`. A basic profile must state
  `"header": "Authorization"` and `"bearer": false`, and names the account
  prompt in `account_label`; the account may not contain a colon. The profile is
  offered to the host as `http_basic` with the fields `account` and `token`.

```json
"auth": {
  "profile": "tracker.api-token",
  "scheme": "basic",
  "header": "Authorization",
  "bearer": false,
  "account_label": "Account email",
  "label": "API token",
  "identity": {"path": "myself", "kind": "tracker.user", "subject_pointer": "/accountId"}
}
```
- `oauth2_refresh` is for providers that issue expiring OAuth access tokens
  from a refresh token that does not rotate, such as Google for installed apps
  (`architecture-decision-record:oauth-material-as-static-entry`). The protected
  entry is `{"client_id":"...","client_secret":"...","refresh_token":"..."}`,
  each field under the token rules, and nothing else. The provider posts it
  form-encoded (`grant_type=refresh_token`) to `token_url`, with no credential
  header, and sends the access token it gets back as
  `Authorization: Bearer <access token>`, so the profile must state
  `"header": "Authorization"` and `"bearer": true`. The access token lives in
  the provider process only, keyed by a digest of the entry, until 60 seconds
  before its `expires_in`; a read or write the API refuses as an invalid credential
  evicts it, and `validate` always exchanges afresh. A token answer carrying a
  different `refresh_token` is refused as an invalid credential and nothing is
  kept. `invalid_grant` and `invalid_client` are an invalid credential; on an
  `operations invoke` of a read on an existing connection the CLI reports it
  with `next_action: repair_connection`, and while connecting with
  `retry_explicitly`. A guarded write refused for its credential reports
  `retry_status` until `story:guarded-write-credential-refusal-says-repair`
  lands; a 429 is a provider
  rate limit and a 5xx is `unavailable`. `token_url` and `authorize_url` must
  be `https` URLs without credentials, query or fragment, written in canonical
  form. `authorize_url` and `requested_scopes` are never called or checked by
  the provider: they reach the host as the profile's `acquisition`, next to
  `token_url`, for the CLI to obtain the entry by consent
  ([Google OAuth guide](catalog-google-oauth.md)).
  `token_ca_file` names trust roots for the token host only; omit it to use
  the platform roots. Like `ca_file`, its bytes enter the configuration
  revision and its path does not. It applies to the provider's refresh and
  validation exchanges, not to the code exchange the CLI makes during consent,
  which uses the platform roots only. Each `minimum_scopes` and
  `requested_scopes` entry is one scope: an empty entry or one holding
  whitespace is refused at load. The profile is offered to the host as
  `http_bearer` with the fields `client_id`, `client_secret` and
  `refresh_token`.
- `oauth2_client_credentials` is for providers that issue an access token to a
  confidential client with no user present and no refresh token, such as
  Zendesk ([Zendesk guide](catalog-zendesk.md#authentication)). The protected
  entry is `{"client_id":"...","client_secret":"..."}` and nothing else. The
  provider posts it form-encoded (`grant_type=client_credentials`) to
  `token_url`, with `requested_scopes` joined by one space as `scope` and no
  credential header, and sends the access token as
  `Authorization: Bearer <access token>`. Caching, eviction, `token_ca_file`
  and the refusal codes are those of `oauth2_refresh`; when the cached token
  expires the provider makes the same request again, so nothing is ever
  refreshed or stored. A token answer carrying a `refresh_token` is refused as
  a protocol failure (RFC 6749 4.4.3). An answer without `expires_in`, which
  RFC 6749 5.1 only recommends, is cached for at most 24 hours; an
  `unauthorized_client` refusal is an invalid credential. The profile requires `token_url` and
  `requested_scopes`, refuses `authorize_url` and an `id_token` identity, and is
  offered to the host as `http_bearer` with the fields `client_id` and
  `client_secret`, with no acquisition: connect it with the hidden prompt or a
  protected credential file.
- `access`, optional: one read validation makes after the identity read and the
  scope read, as `{"path": "...", "query": {"name": "value"}}` relative to the
  provider authority, with no `?` in the path and at most 16 query pairs. A `401`
  or `403` refuses the connection as `insufficient_scope`: the credential
  identified its holder but may not read what the selection needs, which a repair
  with the same credential cannot change. Any other non-2xx answer is refused as
  the identity read refuses it. The body is not read. Omitted, nothing changes and
  existing configuration revisions hold. Confluence declares a v2 page read here
  ([Confluence guide](catalog-confluence.md)), because its v1 identity read passes
  for a token whose scopes do not cover the v2 page reads (#102).
- `identity.source` is `api` when omitted, the read shown above. `id_token`,
  for `oauth2_refresh` only, takes the subject from the `sub` of the token
  answer's `id_token`, after checking that its `iss` is
  `https://accounts.google.com` or `accounts.google.com` and its `aud` is the
  entry's `client_id`, that its `exp` is after now and that its `iat`, if
  present, is at most five minutes ahead, and the granted scopes from the answer's
  space-separated `scope`, which `minimum_scopes` is checked against. The
  `id_token` came straight from the token endpoint over verified TLS, so its
  signature is not checked. When the answer has no `id_token`, the provider
  asks `tokeninfo` on the token host for the fresh access token and reads the
  same `sub`, `aud` and `scope` there. A wrong issuer or audience, or an expired
  `id_token`, is refused as a protocol failure. An `id_token` identity names no `path`,
  `subject_pointer` or `scopes` read.
- `identity.source` `configuration` is for a provider whose API has no user or
  account read, such as Runpod ([Runpod guide](catalog-runpod.md#authentication)).
  The read at `path` must answer `200`, which proves the credential, and its body
  is not read; the subject is the configuration's `instance` id, so the identity
  is the configured connection, not a provider account, and repair or revalidate
  accepts any credential the read admits. It names a `path` and no
  `subject_pointer`, and is only for a static credential (`token` or `basic`):
  under `oauth2_refresh` or `oauth2_client_credentials` it is refused at load,
  because repair would then accept another holder's OAuth credential as the same
  identity. Anything else is refused at load too. The rules are modeled as
  `connectors_catalog.identity.Probe` and `connectors_catalog.identity.Profile`
  in `adapters/catalog/spec/ess`.

```json
"auth": {
  "profile": "google.drive",
  "scheme": "oauth2_refresh",
  "header": "Authorization",
  "bearer": true,
  "label": "Google refresh token",
  "identity": {"source": "id_token", "kind": "google.user"},
  "minimum_scopes": ["https://www.googleapis.com/auth/drive.readonly"],
  "token_url": "https://oauth2.googleapis.com/token",
  "authorize_url": "https://accounts.google.com/o/oauth2/auth",
  "requested_scopes": ["openid", "https://www.googleapis.com/auth/drive.readonly"]
}
```
- `request_prefix` is optional, for a provider reached through an API gateway
  that adds path segments in front of the document's paths, such as
  `/ex/jira/<cloud id>`. It names the leading part of the `api_base` path that
  belongs to the gateway; the rest of that path is the document base every
  selected operation must lie under. Requests, the identity and scope reads
  included, still go to the full `api_base`, which is also the provider
  authority. The prefix is plain path text: a leading `/`, no trailing `/`, and
  segments of letters, digits, `-`, `.`, `_` and `~` only, none of them `.` or
  `..` (so no query, fragment or percent-encoding); it must be the leading part
  of the `api_base` path, or the configuration is refused. Without it the
  configuration revision is what it was before the field existed.
- `hosts` is optional: up to eight origins the connection admits besides
  `api_base`, each `{"origin": "https://<host>", "credential": true}`.
  Only a binary read reaches them, and only those its selection's
  `binary.hosts` names. With `credential` true the connection's credential
  header is sent there too, as Slack's file host requires; absent or false,
  requests there carry no credential, as for a redirect to a URL that carries
  its own grant. Each origin must be canonical (as for `binary.hosts`), named
  once and reached by some selected binary read, or the configuration is
  refused. A host a selection reaches but the connection does not admit is not
  a configuration error: the read that needs it is refused as `forbidden` with
  nothing sent there. Without `hosts` the configuration revision is what it was
  before the field existed; adding it changes the revision. Modelled as
  `connectors_catalog.binary.Hosts`.

  ```json
  "hosts": [{"origin": "https://files.slack.com", "credential": true}]
  ```
- `operations_file` names a shipped selection set; its `provider` must match. An
  inline `operations` list is accepted as well and comes first. The selection
  ids, in either place, are what the host permits and the approval policy names.
- A selection set may also carry `feed`, a provider's `datasource.feed/v1alpha1`
  binding declared as data; the service then declares `feed.containers` and
  `feed.items` as reads under the declaration's profile, and `operations` may be
  empty. The shape, its watermark forms and what it cannot express are in
  [the catalog contract, section 3.3](../contracts/catalog/v1alpha1/semantics.md#33-declared-feed-bindings).
  A selection id may not be `feed.containers` or `feed.items`.
- Each selection exposes one `operationId` from the bundle under a local id.
  `effect` is declared, not inferred from the method: `read` is allowed only for
  GET, `write` only for POST, PUT, PATCH and DELETE, and a write is a
  required-approval mutation under private protocol two like any other.
- `response` is a reviewed exception for a read whose answer is not what the
  bundle's media types say. Without it the bundle decides: a 2xx declared only as
  `text/…` is read as text, anything else as JSON. `text` is for a source that
  declares JSON where the provider answers plain text, as GitLab does for a job
  trace. `binary` reads the body as bytes, whatever media type it carries, and
  needs `binary` beside it. A write never carries either.
- `binary` is required with `"response": "binary"` and refused without it:
  `{"max_bytes": <n>, "hosts": ["https://<host>", …]}`.
  - `max_bytes` (1 to 2,097,152) bounds the body. A longer body is refused as
    `capacity`, never truncated. 2 MiB keeps the base64 body and its envelope
    under the provider's result limit.
  - `hosts` is optional: one to four origins, `https://<host>` or
    `https://<host>:<port>` in lower case, with no path, query, fragment or user
    information, that this read may reach besides `api_base`. A redirect (301,
    302, 303, 307 or 308 with a `Location`) is followed at most three times, and
    only to an origin in `hosts` that the connection also admits (`hosts` in the
    configuration, below); a redirect anywhere else is refused as `forbidden`
    by name, with nothing sent there. Without `hosts` no redirect is followed and
    a 3xx is a refusal.

  The result of a binary read is `{status, body, provenance}` as for any read,
  with `body` `{media_type, length, sha256, content_base64}`: the answer's
  `Content-Type` without parameters in lower case (`application/octet-stream`
  when it names none), the length in bytes, the lower-case hex SHA-256 of the
  bytes and the bytes in standard base64 with padding. `status` is that of the
  answer that carried the body, after any redirect. The body is never written to
  a file. A non-2xx answer after a redirect is classified as for any read.
- `download` is optional, only with `"response": "binary"`, `binary.hosts` and no
  `operation_id`: `{"path_prefix": "/<segment>/"}`. It makes the selection a read
  of a URL the provider itself handed out, such as Slack's `url_private`: the
  only input is `url`, an `https` URL on an origin in `binary.hosts` whose path
  starts with `path_prefix`, sent as one GET to that origin. A URL elsewhere is
  refused before any request. A `download` selection names no bundle operation;
  `path_prefix` must start and end with `/`.

  `response`, `binary` and `download` are modelled as
  `connectors_catalog.binary.BinarySelection` in `adapters/catalog/spec/ess`.
- `bounds` is optional and narrows query parameters the source declares, each
  by exactly one of two forms:
  - a range, `{"<parameter>": {"minimum": <n>, "maximum": <n>}}`, such as a
    provider's page-size cap; `minimum` may be omitted. The value, whether sent
    as a number or a string, must be a decimal integer no greater than `maximum`
    and no less than `minimum` (`-0` is zero). The declared input schema
    carries both limits.
  - a set, `{"<parameter>": {"values": ["<value>", …]}}`, for a parameter the
    source types as a string, such as GitLab's search `scope` held to `blobs`.
    The text the value is sent as must be exactly one of the values (a JSON
    integer is compared as its decimal text); case, whitespace and
    comma-joined variants are not. The declared input schema lists the set as
    the parameter's `enum`.

  A value outside its bound is refused as `invalid_input` before any request;
  a repeated parameter's bound holds for each element. When the selection
  loads, a bound is refused on a parameter the operation does not declare as a
  query parameter, with both forms or neither, with a `minimum` above its
  `maximum` or beside `values`, with `values` on a parameter the source does not
  type as a string, or with an empty set or an empty or repeated value. The
  form is modelled as `connectors_catalog.selection.Bound` in
  `adapters/catalog/spec/ess`.
- `required` is optional: `["<query parameter>", …]` marks query parameters the
  provider requires although the pinned source does not. Each is then declared
  required and refused as `invalid_input` when absent, before any request. A
  name that is not a query parameter of the operation is refused when the
  selection loads.
- `withhold` is optional: `["<parameter>", …]` names declared parameters the
  selection does not expose, such as a paging mode the provider cannot follow.
  Each is left out of the declared input schema, so `operations describe` does
  not list it, and an input carrying it is refused as `invalid_input` before
  any request. A name that is not a parameter of the operation, one the source
  or the selection's `required` marks required, one the selection also bounds,
  or one its guard reads as an input is refused when the selection loads.
- `credential` is optional: `["<parameter>", …]` names the parameters through
  which the pinned source passes the credential that the connection already
  sends in its authentication header, such as Slack's `token`. Each is left
  out of the operation even when the source marks it required, so it is never
  declared, never required and never sent, and an input carrying it under any
  spelling is refused as `invalid_input` before any request. The removal comes
  before the check that refuses an operation needing a header parameter, so a
  required credential header no longer refuses the selection. A name that is
  not a query or header parameter of the operation (a path parameter of the
  same name included), one the selection also bounds, marks `required` or
  withholds, or one its guard reads as an input is refused when the selection
  loads. A write's JSON body is caller input too: a top-level body key equal to
  a credential name, compared without regard to ASCII case, is refused as
  `invalid_input` before any request, and the declared input schema says the
  same of an open body; `body_keys` admitting such a name is refused when the
  selection loads. It applies to a guard's preflight read as well: each name
  leaves the probe operation, so the probe's request never carries it from
  caller input and the connection's header carries the credential, and a
  guard whose `preflight.values` names one as a probe parameter, under any
  ASCII case, is refused when the selection loads. `withhold` keeps its
  meaning: it still refuses a parameter the source requires.
- `rate_limit_reasons` is optional: `["<reason>", …]` names the reasons a
  provider gives in a `403` when it means a quota rather than a permission. A
  `403` whose JSON body carries one of them in `error.errors[].reason` or
  `error.status` is `rate_limited`, for a read and for a write's definite
  refusal; every other `403` stays `forbidden`. An empty reason is refused
  when the selection loads.
- `body_keys` is optional, for a write: `["<key>", …]` closes its JSON body to
  exactly those top-level keys. The declared input schema types `body` as an
  object with those properties and no others, and a body carrying any other key
  is refused as `invalid_input` before any request, the preflight read
  included. Each key a guard reads is required and compared by that guard; one
  it reads as `body.<key>` itself is declared as the scalar it compares. A key
  no guard reads takes any JSON value and is compared by nothing: closing the
  body only keeps out keys outside the set. It is for a write whose provider
  would act on a body field the guard does not compare, such as a Gmail draft
  send given a replacement message. A read, an operation
  without a request body, an empty, dotted or repeated name, or a guard reading
  a `body.<key>` the set does not admit is refused when the selection loads.
  The bundle records no body schema, so the names are not checked against the
  provider's.
- `body_types` is optional, beside `body_keys`: `{"<key>": "string" | "integer"
  | "boolean"}` gives a closed body key the JSON type the provider's request
  body schema gives it. A typed key present in a body must be a JSON value of
  exactly that type, with no string spelling of a boolean or an integer and no
  `null`, or the write is refused as `invalid_input` before any request; the
  declared input schema types it the same, also where a guard reads it. One
  difference: an `integer` key admits only an integer literal in the 64-bit
  range, while JSON Schema's `integer` also admits `3.0` or `1e2`, so such a
  value passes the declaration and is still refused.
  `body_required` is optional too: `["<key>", …]` names the closed body keys
  that schema requires, and a body without one is refused as `invalid_input`
  before any request and declared required (keys a guard reads are required
  already). GitLab's note and reply type `body` as a string and require it; its
  discussion resolve types `resolved` as a boolean. A key either names that
  `body_keys` does not admit (so either one without `body_keys`), a key
  `body_required` repeats, or a guard reading a path nested under a typed key
  is refused when the selection loads. Like `body_keys`, they are declared by
  the selection from the pinned document and not checked against it.
- `body_fixed` is optional, beside `body_keys`: `{"<key>": <value>}` fixes a
  closed body key to one JSON string, integer or boolean. The provider sends
  exactly that value under that key on every write, and a body naming the key
  at all, with that value or any other, is refused as `invalid_input` before
  any request. The key is left out of the declared input schema; when every key
  `body_keys` admits is fixed, `body` is not required and may be omitted. It is
  for a guarded variant of an existing write whose meaning rests on one body
  value, such as GitLab's merge with `auto_merge: true` or its update with
  `state_event: reopen` ([Auto-merge and reopen](local-gitlab-merge.md#auto-merge-and-reopen)).
  A fixed key `body_keys` does not admit (so any without `body_keys`), a value
  that is not a string, an integer or a boolean or not of the key's
  `body_types` type, a key `body_required` names and a key a guard reads are
  refused when the selection loads: a guard compares a fixed value as a
  `literal`. `body_keys`, `body_types`, `body_required` and `body_fixed` are
  modelled as `connectors_catalog.selection.ClosedBody` in
  `adapters/catalog/spec/ess`, and every shipped selection is checked against
  it.
- `guard` is optional and declarative. The preflight reads another GET from the
  bundle, binding its parameters from the write's input, and refuses before any
  request unless every check holds. A check compares the scalar at a JSON
  `pointer` in the observed body with `{"input": "<key>"}` (a top-level input or
  `body.<key>`) or `{"literal": "<value>"}`. The postflight applies its checks to
  the write's own response; a difference there leaves the outcome uncertain and
  never refused, because the provider may already have applied the write. No
  corrective request is ever issued. This is the C14 boundary the operator
  accepted for create and update, written as data.
  - `further_preflights` is optional: up to three more reads before the write,
    each `{"operation_id", "values", "checks"}` like `preflight`, read in order
    after it; the first that refuses stops the guard with nothing written. It
    is for preconditions that live in two answers, such as a Jira issue's status
    and whether a transition is open on it.
  - `postflight.read` is optional: `{"operation_id", "values"}`, a GET of the
    bundle bound from the write's input like a preflight. With it, the engine
    issues that read once after the write is answered with a `2xx`, and the
    postflight's checks, at least one, apply to the read's answer instead of the
    write's. It is for a write that answers without a body, such as Jira's
    `doTransition` (`204`). A read that fails, or answers without the pinned
    value, leaves the outcome uncertain as a differing value does. The write's
    own status and body are still what the operation returns.
  - `postflight.any_of` is optional: two or more checks of which at least one
    must hold, beside every check in `postflight.checks`, against the same
    answer. It is one check that accepts one of several observations, for a
    write whose success has more than one shape: GitLab's merge with
    `auto_merge: true` answers either with `merge_when_pipeline_succeeds`
    `true` or, when the pipeline had already succeeded, with `state` `merged`.
    An answer with none of them leaves the outcome uncertain. Its checks count
    toward the sixteen.
  - `postflight.absent` is optional, only `true` and only beside a postflight
    `read` with no `checks` and no `any_of`: that read must answer `404` after
    the write, proving a delete whose provider answers without a body, such as
    GitLab's tag delete (`204`). The target still found, or a read that fails
    otherwise, leaves the outcome uncertain. The write's own status and body
    are what the operation returns.
  - Every read binds every value, and every check resolves its input, before
    the first request; a missing one is refused as `invalid_input` with nothing
    sent. A read that is not a GET under the base path, more than sixteen checks
    in all, a further preflight without checks, more than three of them, a
    postflight read without checks and without `absent`, an `absent` that is
    not `true`, lacks a read or sits beside a check, and an `any_of` of one
    check are refused
    when the selection loads, as is a credential parameter bound by any read's
    `values`. The format is modelled as `connectors_catalog.guard.Guard` in
    `adapters/catalog/spec/ess`, and every shipped guard is checked against it.

The merge guard in the shipped set reads the merge request and requires, before
the one PUT: `/sha` equal to the pinned `body.sha`, `/state` literally `opened`,
`/detailed_merge_status` literally `mergeable`, `/head_pipeline/id` equal to the
input `pipeline_id` and `/head_pipeline/status` literally `success`. After it:
`/state` literally `merged` and `/sha` still the pinned head. Those are the
checks the retired native `merge_request.validate` made in Rust, as five lines of data.

### Array and required query parameters

A query parameter the pinned source declares as an array with `style: form` and
`explode: true` (OpenAPI's default for a query parameter) is repeated: its input
takes a JSON array of scalars, sent as one `name=value` pair per element in the
order given, each encoded as a single value is; an empty array sends nothing.
One scalar is still accepted and sent as one pair, so a caller that already
sends a comma-joined string, such as Jira's `fields` or Confluence's
`space-id`, sends the same request as before. That scalar is typed like the
elements: for integer elements, an integer or a string of comma-separated
integers (`"65538,98305"`); for boolean elements, `true`, `false` or a string
of them separated by commas; for string elements, any string (or a JSON
integer, as for any string parameter); for elements of no declared type, any
scalar. The declared input schema says the same: its `type` is `array` together
with those scalar types, `items` is the element type, a `pattern` constrains
the joined string, and a required repeated parameter declares `minItems: 1`
because an empty array is refused as absent. A `bounds` entry holds for each
element, and a comma-joined string for a bounded parameter is refused. An
array for a parameter that is not repeated, an element that is itself an
array, an object or `null`, or a scalar that fits none of the forms above, is
refused as `invalid_input` before any request.

An array query parameter with any other `style`, or `explode: false`, is
recorded as a gap in the bundle and keeps the one-value reading it had before:
the caller sends the joined value itself. Array path and header parameters are
not read as arrays.

A selection's `required` list adds the provider's requirement where the source
omits it; see the selection fields above.

The bundles were rebuilt to record repeated parameters. The configuration
revision digests the bundle's SHA-256, so the rebuild moves the configuration
revision, and with it the descriptor revision, of every GitLab, Jira and
Confluence instance, including one whose selections have no repeated
parameter. After upgrading:

1. Print the bootstrap again and copy its `configuration_revision` into the
   adapter entry (see [Bind the provider to the local
   CLI](#bind-the-provider-to-the-local-cli)). Until then the adapter does not
   start: the host refuses a bootstrap whose revision differs from the entry's.
2. Run `connections revalidate` on each connection of an instance that was
   already connected. It moves the connection to the new configuration revision
   with the credential already in custody, and asks for no credential, when the
   provider host and the profile are unchanged and the provider still answers
   the same identity. Until then the connection reports `pending` and reads
   refuse with `lifecycle_conflict` and `next_action = revalidate_connection`.
   When the provider host or the profile changed, revalidate refuses with
   `lifecycle_conflict` and `next_action = create_connection`; when the provider
   answers another identity, or refuses the credential, with its own code and
   the same `next_action`. The credential is not invalidated; the refusal is
   recorded, and reads then name `create_connection` too. Connect again: under
   the same `instance` id a new connection is admitted under the configured
   revision and moves the instance to it when it publishes; a changed provider
   host or profile still needs a new `instance` id. See [Configuration
   upgrades](local-connection-registry.md#configuration-upgrades).
3. Issue every write approval policy bound to the instance again.

Separately, the declared input of these shipped reads changed, because each
has a repeated parameter: Jira `issues.search`; Confluence `pages.changed`,
`space.pages`, `page.get` and `page.comments`; and GitLab `issues.list`
(`assignee_username`, `not[labels]`, `not[iids]`, `not[assignee_username]`)
and `merge_requests.list` (`assignee_username`, `not[assignee_username]`,
`not[labels]`).

## Bind the provider to the local CLI

The local CLI requires Linux x86_64 and the
[qualified Secret Service binding](local-secret-service.md). Use the optimized
build for the local owner: startup copies and verifies the executable within a
bounded deadline, and a large debug binary can exceed that budget on a busy host.

```sh
target/release/connectors --output json setup init
target/release/connectors --output json setup check
target/release/connectors-catalog-provider --local-config /absolute/path/gitlab-catalog.json --print-local-bootstrap
sha256sum target/release/connectors-catalog-provider
```

Add an adapter entry to the configuration created by setup, with the bootstrap's
exact `configuration_revision`, the executable's SHA-256 and absolute paths. The
operation ids are the selection ids; omitted permission sets deny access.

```toml
[adapters.forge]
instance_id = "gitlab-sandbox"
adapter_id = "catalog"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/2"
startup = "on-demand"
restart = "never"

[adapters.forge.permissions]
profiles = ["gitlab.pat"]
operations = ["project.get", "issues.list", "file.get", "branch.get", "merge_requests.list", "merge_request.get", "pipelines.list", "pipeline.get", "pipeline.jobs", "job.get", "job.trace", "issue.create", "merge_request.create", "merge_request.update", "merge_request.merge"]

[adapters.forge.executable]
path = "/absolute/path/connectors-catalog-provider"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/gitlab-catalog.json"]
```

Credentials never belong in TOML, native configuration, executable arguments or
environment variables. Connect with the hidden token prompt on your controlling
terminal, or `--credential-file` / `--credential-stdin` carrying the protected
`{"token":"..."}` document (`{"account":"...","token":"..."}` for a basic
profile) from an owner-only source:

```sh
target/release/connectors --output json connections connect --adapter forge --profile gitlab.pat --credential-prompt
target/release/connectors --output json operations describe --adapter forge --operation project.get
```

Connect runs the declared identity and scope reads and refuses a token below
`minimum_scopes`. Validation evidence lasts the `auth` object's `evidence_lifetime_ms`
(60 seconds when omitted, at most 300 000 ms); after expiry the
connection reports `pending`, and `operations invoke` of a read refuses with
`not_granted` at `admission` and `next_action: revalidate_connection` until
`connections revalidate --adapter forge --connection CONNECTION --expected-revision REVISION`
renews it with no credential re-entry. The invoke does not revalidate on its own.
A credential the provider refused or one below an operation's scopes still
answers `next_action: repair_connection`. `connections repair` replaces an invalid
credential and refuses a changed identity; `connections revoke` is terminal local
revocation and does not revoke the token at GitLab. Lists, descriptions and status
start nothing. Writes need `private_protocol = "connectors-private/2"` and an
approval policy naming them; see [the guarded merge guide](local-gitlab-merge.md).

## Invoke

Input is one property per declared path or query parameter, named as the source
names it, a `body` object where the operation takes one, and any extra value a
guard reads. Output is the provider's status, its body unchanged (JSON, a
string for a text read, or `{media_type, length, sha256, content_base64}` for a
binary read), and provenance whose `source_revision` is the pinned source's
SHA-256.

```sh
connectors operations invoke --adapter forge --connection "$connection" \
  --operation merge_request.get --schema "$schema" --revision "$revision" \
  --input-json '{"id":"group/project","merge_request_iid":9}'
```

```json
{"id":"group/project","merge_request_iid":10,"pipeline_id":19,
 "body":{"sha":"<source branch head>"}}
```

Prepare, issue and invoke a write with that file exactly as
[the guarded merge guide](local-gitlab-merge.md) describes. A read that the
provider refuses is an error with a safe code; a write is classified
`not_attempted` when the guard refuses in preflight, `refused` on a documented
definite refusal (400, 401, 403, 404, 405, 409, 410, 412, 415, 422), `applied` on
a 2xx whose postflight holds, and `unknown` for everything else.

## What the sandbox showed

Against the live GitLab, through this provider and the shipped selection set:
all eleven reads answered, the job trace as text; a merge with a stale pinned
`sha` and a merge naming the wrong pipeline were both refused before dispatch
with no request to the merge endpoint; the merge at the pinned head with the
successful pipeline merged request 10 with exactly one PUT; and an update of the
now-merged request was refused by the literal `/state` check with no request
sent. Before that, with the selection typed into the configuration: a create that
opened merge request 10 at its pinned head, the same create with a stale pin
refused with no request sent, a second create for the same branch refused by
GitLab's 409, an update that retitled 10, the same update at a stale pin refused
before dispatch, and a create whose branch was moved the moment the preflight GET
appeared, which opened merge request 11 at the moved head and was classified
`unknown`. The full record is in
[the sandbox evidence](evidence/gitlab-sandbox-20260913/README.md).

## Limits

- The bundle records parameters, media types and response statuses; it does not
  carry request or response schemas. A `body` is passed through as the caller
  supplies it and validated only by the provider.
- Header and cookie parameters are not carried; a selection whose operation
  requires one is refused at load.
- A binary answer is read whole into the result, at most 2 MiB; there is no
  streaming and no binary or multipart request body, so
  uploads are not served. Binary reads are verified against local fixtures
  only (`adapters/catalog/tests/binary_responses.rs`).
- One authentication profile per configuration: a token in one header, a
  basic profile (`"scheme": "basic"`) sending an account and API token as HTTP
  basic, an OAuth refresh profile (`"scheme": "oauth2_refresh"`), or an OAuth
  client-credentials profile (`"scheme": "oauth2_client_credentials"`). Signing
  profiles and rotating refresh tokens are not offered by this provider. The
  basic and both OAuth profiles have run only against the local fixture, not
  a live provider, and the provider does not acquire the OAuth entry itself.
- Pagination and error envelopes are not declared; a paged read returns one page
  as the provider answers it.
- A read answered `429` is sent once more, and never a third time, after the
  delay its `Retry-After` names (delta-seconds, or an HTTP-date rounded up to a
  whole second), when that wait leaves the second request time of its own: the
  wait, then as long again as the first request took, then 500 ms, must all end
  before the invocation deadline. The second request must finish 500 ms before
  the deadline; if it does not, the first answer's refusal and delay stand.
  Otherwise, and after a second `429`, `operations invoke` fails with
  `service_code: rate_limited` and `retry_after_seconds` set to the named delay,
  so the caller can wait; without `retry_after_seconds` the provider named no
  delay the engine could read. Two `Retry-After` lines on one answer that
  disagree name no delay; identical repeats count as one. A guarded write, and
  its preflight read, is sent once and never again, and its failure carries no
  delay.
- A guard compares scalars for equality. It cannot express "any of", ordering or
  a value the preflight must not have.
- Only GitLab has run live. A second provider through the same engine is still
  required by `specification:catalog-http-runtime-handoff`.
