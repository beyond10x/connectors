# Guarded GitLab merge through the local CLI

`merge_request.merge` and `merge_request.update` run through the
[catalog provider](local-catalog-provider.md) from the pinned GitLab OpenAPI
source, selected by the shipped selection set. Merge is guarded: the provider
reads the merge request first and refuses before any request unless it is open,
mergeable, at the pinned `body.sha`, with the pinned `pipeline_id` as its head
pipeline in status `success`; GitLab then checks `sha` atomically on the PUT.
Update carries no SHA precondition at GitLab, so it runs under the accepted race
boundary: a pinned head that already differs is refused in preflight, and a head
that moves during dispatch leaves the outcome uncertain, never refused. The
postflight comparison is best effort: a merge request's recorded head is
eventually consistent with its source branch, and a live GitLab has answered the
update PUT with the pinned head while the branch had already moved.

The local CLI calls both through the approval, audit and mutation coordinator.
Dedicated sandbox acceptance is recorded in
[docs/evidence/gitlab-sandbox-20260913](evidence/gitlab-sandbox-20260913/README.md);
recovery across every storage acknowledgement remains open.

The merging identity needs more than `api` scope. GitLab answers a merge request
**401**, not 403, when it has identified the user and that user may not merge into a
protected branch, so a permission problem reads as an authentication problem. Check
the branch's `merge_access_levels` before concluding the credential is wrong.

Start with the [catalog provider guide](local-catalog-provider.md). Select
`format = "connectors-local/2"` in the host configuration and
`private_protocol = "connectors-private/2"` on the adapter. Select the exact
executable hash, permit `merge_request.merge` and `merge_request.update` in the
adapter's operation permissions and retain `gitlab.pat` in its profile
permissions. The saved PAT must grant GitLab `api` scope. Use a fresh explicit
local selection and an admitted connect to collect its descriptor before
discovering the new metadata. There is no automatic configuration rewrite,
credential migration or fallback write path.

Configure a [bounded authenticated clock](local-clock.md), initialize
[approval-signing keys](local-approval-keys.md) and use the
[approval commands](local-approvals.md) to publish a policy file containing:

```json
{"operations":["merge_request.merge","merge_request.update"]}
```

Discover the exact `schema` and descriptor `revision` using
`connectors operations describe --adapter forge --operation merge_request.merge`.
Put the intended business input in `merge.json`: the project path or numeric id,
the project-local MR IID, the expected successful head pipeline id, and the PUT
body with the exact lowercase source SHA:

```json
{"id":"group/project","merge_request_iid":17,"pipeline_id":123,"body":{"sha":"0123456789abcdef0123456789abcdef01234567"}}
```

Use `approvals prepare` with this input and those exact selectors. Inspect its
subject and pass that subject's digest to `approvals issue --approve-subject`.
Issue to a new absolute `--proof-output` path in a private directory. Ordinary
output contains metadata; the proof stays in the owner-private file.

Invoke the same target and input:

```sh
connectors operations invoke --adapter forge \
  --connection "$connection" --operation merge_request.merge \
  --schema "$schema" --revision "$revision" --input-file merge.json \
  --approval-file "$proof_file" --idempotency-key "$business_key" --output json
```

The optional key is an opaque string of 1–256 bytes. Reuse the exact key for
observation of this invocation. A conflicting input refuses without disclosing
the original result. The optional proof-file argument supports exact replay
without another proof; a new attempt always requires a valid proof. Reads refuse
both write-only options. The original 20-second write budget includes argument
processing, document/proof acquisition, startup, preflight and commit.

Fresh preflight is the guard's five checks against one GET of the merge request.
Commit sends one PUT with the `body` as supplied; add `auto_merge` or
`should_remove_source_branch` there only when wanted, and GitLab applies the
project's squash configuration otherwise. GitLab checks the source SHA
atomically; the pipeline id and status are preflight observations, not an atomic
merge condition.

Success retains the usual JSON-text `result` and adds `request_id`, `mutation`
and `source_audit`. Failure carries those observations inside `error.data` when
available. Inspect `mutation.classification`: `not_attempted`, `refused`,
`applied` or `unknown`. Applied can accompany a result-delivery error; incomplete
audit cannot undo the known effect. A lost response is unknown and never triggers
another PUT. Repeating an exact retained key for a terminal attempt under current
result-access policy observes it without provider, credential, proof or clock access.
Every delivery has fresh request/audit correlation. A pending original never
authorizes resend. Repeating its exact key may start the owner for metadata
recovery on the serialized instance worker, without loading a proof or credential.
Recovery does not launch a native child; owner startup still honors configured
automatic startup. The on-demand fixture starts no native child. A live original
remains pending. An abandoned dispatch becomes quarantined/unknown; an abandoned preparation becomes
not_attempted when trusted time is available for its replay retention. Unavailable
time leaves the preparation pending. Terminal replay remains passive.
While running, the owner also scans pending attempts periodically, including
unkeyed writes and targets later revoked or removed from configuration. It fences
abandoned records without another invocation of the original write. Live worker
exchanges finish before their queued recovery batch runs; suppression remains set.
Background recovery does not expose an old result or grant a new effect.

The production CLI journeys that covered applied, refused and lost-response
settlement, same-key observation through a newly started owner, an owner killed
after the provider received the PUT, background recovery and post-effect
revocation ran with the native adapter as the child; their records stay under
`docs/evidence/gitlab-*-20260911/`, and the host mechanics they exercised are
unchanged. They have not been re-run with the catalog provider as the child;
`story:catalog-cli-journeys` owns that.

## Notes and discussions

Three more merge-request writes and one read run the same way, from the same pinned source and
selection set. Each write is a required-approval mutation: name it in the approval policy, prepare
and issue an approval for the exact input, then invoke it with `--approval-file`, as above.

```json
{"operations":["merge_request.note.create","merge_request.discussion.reply","merge_request.discussion.resolve"]}
```

In every input, `id` is the project path or numeric id and `noteable_id` is the merge request's
project-local IID (GitLab's name for it on these routes). A discussion id is the string GitLab
gives each discussion; `merge_request.discussion.get` reads one by that id.

| id | GitLab operation | request | guard |
|---|---|---|---|
| `merge_request.note.create` | `postApiV4ProjectsIdMergeRequestsNoteableIdNotes` | `POST /projects/{id}/merge_requests/{noteable_id}/notes`, body `body` (a string, required) and optionally `internal` (a boolean) | none |
| `merge_request.discussion.reply` | `postApiV4ProjectsIdMergeRequestsNoteableIdDiscussionsDiscussionIdNotes` | `POST …/discussions/{discussion_id}/notes`, body `body` (a string, required) | none |
| `merge_request.discussion.get` | `getApiV4ProjectsIdMergeRequestsNoteableIdDiscussionsDiscussionId` | `GET …/discussions/{discussion_id}` (a read) | — |
| `merge_request.discussion.resolve` | `putApiV4ProjectsIdMergeRequestsNoteableIdDiscussionsDiscussionId` | `PUT …/discussions/{discussion_id}`, body exactly `resolved`, a JSON boolean | preflight `id` equals `discussion_id` and `resolvable` is `true`; postflight `id` equals `discussion_id` and `resolved` equals `body.resolved` |

A note on merge request 17:

```json
{"id":"group/project","noteable_id":17,"body":{"body":"Looks good; one question on the retry bound."}}
```

Resolving one discussion of it (`false` unresolves):

```json
{"id":"group/project","noteable_id":17,"discussion_id":"6a9c1750b37d513a43987b574953fceb50b03ce7","body":{"resolved":true}}
```

**The two note writes are unguarded.** A note or a reply creates something new, so there is
nothing to compare before it, as with `issue.create`: the same input approved and sent again
adds a second note. The approval binds the whole input by digest, so the approver reads the exact
text that will be posted. That matters because GitLab runs quick actions written in a note, such
as `/close`, `/approve`, `/label` or `/assign_reviewer`, so a note can change the merge request as
well as comment on it. Which quick actions GitLab runs from a note has not been observed against
a running GitLab. The bodies are closed: the merge-request note takes `body` and `internal`, the
reply takes `body`, and a body carrying `created_at` ("The creation date of the note"),
`confidential` (deprecated in 15.5, renamed to `internal`), `merge_request_diff_head_sha` ("The
SHA of the head commit") or any other key is refused before any request. As the pinned request
bodies require (`RequestBody_ac6f9367f3de` for the note, `RequestBody_a45089edc8dd` for the
reply), `body` must be present and a JSON string, and the note's `internal`, when given, a JSON
boolean; a body without its text, with a non-string text or with `internal` as `"true"` or `1` is
refused as `invalid_input` before any request (the selections' `body_required` and `body_types`,
[selection format](local-catalog-provider.md)).

**Resolve is guarded.** `resolved` must be a JSON `true` or `false`, as the pinned
`RequestBody_b5c6ef66b3c0` types it: `"true"`, `1`, `"yes"` or `null` is refused as
`invalid_input` before any request. Before the one `PUT` the provider reads the discussion and
refuses, with nothing written, unless GitLab's answer is the discussion `discussion_id` (its `id`)
and reports it `resolvable`: an individual note cannot be resolved, and a discussion GitLab does
not find, or a read answered with another discussion, is refused the same way. GitLab answers the
`PUT` with the discussion, and that answer must again carry `discussion_id` as its `id` and the
requested `resolved`; otherwise the outcome is `unknown`, never refused. Resolving an already resolved discussion is allowed and is applied when
GitLab's answer shows it resolved. Like the update guard, this one is not atomic: GitLab offers
no precondition on the `PUT`, so a discussion that changes between the read and the write is seen
only in the answer.

A provider refusal of any of these writes keeps GitLab's status as its name: `403` and `405` are
the provider's forbidden, `404` its not-found, `409` invalid input. These operations are checked
against a fixture in the pinned document's shapes
(`adapters/catalog/tests/gitlab_mr_writes.rs`), not against a running GitLab.

## Auto-merge and reopen

Two guarded variants of the writes above run the same way, from the same pinned source and
selection set, since 2026-10-10. Each selection fixes one body member to one value
(`body_fixed`, [selection format](local-catalog-provider.md#configure-the-provider)): the
provider sends exactly that value on every write, and a caller's body naming the member at all,
with that value or any other, is refused as `invalid_input` before any request. Name them in
the approval policy as above:

```json
{"operations":["merge_request.auto_merge","merge_request.reopen"]}
```

| id | GitLab operation | request | guard |
|---|---|---|---|
| `merge_request.auto_merge` | `putApiV4ProjectsIdMergeRequestsMergeRequestIidMerge` | `PUT /projects/{id}/merge_requests/{merge_request_iid}/merge`, body exactly the caller's `sha` and the fixed `auto_merge: true` | preflight `sha` equals `body.sha` and `state` is `opened`; postflight `sha` equals `body.sha`, and either `merge_when_pipeline_succeeds` is `true` or `state` is `merged` |
| `merge_request.reopen` | `putApiV4ProjectsIdMergeRequestsMergeRequestIid` | `PUT /projects/{id}/merge_requests/{merge_request_iid}`, body exactly the fixed `state_event: reopen` | preflight `sha` equals the input `sha` and `state` is `closed`; postflight `state` is `opened` |

**Merge when the pipeline succeeds.** GitLab waits for the pipeline only when the body carries
`auto_merge: true` (`merge_when_pipeline_succeeds` in the pinned document is deprecated in its
favour); with `false` or without it, the same request merges at once. The selection fixes it,
and closes the body to `sha` and `auto_merge`, so the caller sends only the pinned head:

```json
{"id":"group/project","merge_request_iid":17,"body":{"sha":"0123456789abcdef0123456789abcdef01234567"}}
```

The guard pins the head and the open state, and deliberately not a mergeable status or a
succeeded pipeline: waiting for the pipeline is what this variant is for. GitLab still checks
`sha` atomically on the PUT. Its answer has one of two shapes, and the postflight accepts either
(`any_of`): auto-merge set (`merge_when_pipeline_succeeds` `true`, still `opened`), or `merged`
when the pipeline had already succeeded. An answer with neither, or another head, is `unknown`,
never refused. The deprecated `merge_when_pipeline_succeeds`, `should_remove_source_branch`,
`squash` and every other body key are refused before any request; use `merge_request.merge`
for a merge with those options. `merge_request.merge` keeps its five-check guard.

**Reopen.** The selection fixes `state_event` to `reopen` and admits no other body key, so the
caller's `body` may be omitted (or sent as `{}`); `sha` is the pinned head, as for
`merge_request.update`:

```json
{"id":"group/project","merge_request_iid":17,"sha":"0123456789abcdef0123456789abcdef01234567"}
```

Before the one `PUT` the provider reads the merge request and refuses, with nothing written,
unless it is `closed` at the pinned head; an open or merged one is refused the same way. GitLab's
answer must report it `opened`; otherwise the outcome is `unknown`. Like the update guard, this
one is not atomic: GitLab offers no precondition on the `PUT`. `merge_request.update` keeps its
guard, so it still refuses a closed merge request.

Both variants are checked against a fixture in the pinned document's shapes
(`adapters/catalog/tests/gitlab_mr_variants.rs`), not against a running GitLab. The pinned
document declares `merge_when_pipeline_succeeds` on the merge request GitLab answers; that GitLab
answers an accepted auto-merge with it `true` has not been observed against a running GitLab.
