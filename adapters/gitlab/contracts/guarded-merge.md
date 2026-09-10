# Proposed guarded merge

This selects the first native write in the current GitLab phase. It is not yet
advertised. Existing MR reads and validation remain observations. Create/update
retain the open C14 atomic-head decision; this document does not weaken it.

`merge_request.merge` is an approval-required, local-only Write. Its input reuses
the authored ValidationInput: allowlisted project, positive project-local IID,
exact lowercase source SHA and positive expected pipeline ID. It uses the saved
PAT identity with GitLab `api` scope. Raw credentials and native response bodies
do not belong in host ledgers. Shared subjects, attempts, approvals and audit stay
with their existing owners.

Within the original invocation budget, perform the same current native checks as
`merge_request.validate`. A failed check refuses before any business write.
Capture the validated project/IID/head and immutable request; return preparation
to the host and await its final guarded commit. A previously returned validation
observation is never permission to skip this preflight.

The native request is one `PUT /projects/{project}/merge_requests/{iid}/merge`,
with JSON `sha` from the input, `auto_merge:false` and
`should_remove_source_branch:false`. Omit squash to follow project configuration;
no custom commit message, deferred merge, train bypass or branch deletion is
offered. GitLab's endpoint compares the supplied SHA with source HEAD. Pipeline
identity/status was checked during preflight; the API does not offer an atomic
expected-pipeline-ID condition. Current provider merge rules still apply.

A successful response must decode within the bound, identify the captured
project/IID/source SHA and report `state:merged` before Applied is claimed.
`merge_commit_sha` may be null. Documented permission, mergeability, source-head
conflict and failed-merge refusals are native Refused outcomes. Ambiguous errors,
timeouts, malformed/wrong-target responses and non-merged success bodies remain
Unknown after commit. No response triggers another PUT. An exact-target later
read is available independently; it does not attribute a quarantined attempt's
effect or grant retry.

The source is the checked-in OpenAPI at GitLab commit
`2ff8d865e5016b14b724d1c2ce745f8300696192`, SHA-256
`f9e830bd3d2b99c49d60a7713fe1a64f5164418aca24b559287daab075beb530`,
request body `RequestBody_ce0a14f220cb`, together with the
[official merge endpoint documentation](https://docs.gitlab.com/api/merge_requests/#merge-a-merge-request).
Those support the request/response rules above; Connectors owns the surrounding
approval, preflight, dispatch and uncertainty policy.

Fixtures must count PUT effects, including a response lost after the merge and
same-key invocation after owner/CLI restart with no second PUT. Also verify
changed head between preflight and commit, failed pipeline checks, policy/key
revocation, approval reuse, idempotency conflicts and legacy refusal. Dedicated
GitLab sandbox evidence and exact cleanup of test-owned resources are additionally
required for this workflow's acceptance; local fixtures cannot supply that fact.
