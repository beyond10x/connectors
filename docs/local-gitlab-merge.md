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
