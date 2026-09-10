# GitLab pinned-head validation

`merge_request.validate` is the read-only validation portion of C14. It accepts
the configured project, positive project-local `iid`, exact lower-case 40- or
64-digit `sha`, and positive `pipeline_id`. Both IDs fit signed 64-bit integers.
The saved connection, project allowlist and operation permission apply before
the generated GET of the same MR endpoint used by `merge_request.get`.
There is one bounded provider response and no polling, status-recheck query,
fallback to the deprecated `pipeline` field, or mutation.

The result contains `item` and provenance. The item includes the existing bounded
`merge_request` observation, `expected_sha`, `expected_pipeline_id`, nullable
`head_pipeline`, `checks_passed`, `blockers` and `merge_performed:false`.
The head pipeline projection selects positive `id` and `project_id`, exact SHA
and nonempty status of at most 64 bytes. Its project must match the observed
source or target project; this permits fork pipelines without authorizing another
project endpoint. Missing/null head pipeline means unavailable checks. A present
malformed pipeline or inconsistent project fails `upstream_protocol`.

`checks_passed` is true exactly when no blocker applies. Blockers are returned
once each, in this order:

| Blocker | Predicate |
|---|---|
| `head_unavailable` | MR SHA is null. |
| `head_changed` | A non-null MR SHA differs from the requested SHA. |
| `not_open` | State is not exactly `opened`. |
| `draft` | MR is a draft. |
| `merge_checks_pending` | Detailed merge status is not exactly `mergeable`, including unknown statuses. |
| `pipeline_unavailable` | Head pipeline is missing/null. |
| `pipeline_changed` | Head pipeline ID differs from the requested pipeline ID. |
| `pipeline_head_mismatch` | Head pipeline SHA differs from the requested SHA. |
| `pipeline_not_successful` | Head pipeline status is not exactly `success`, including skipped/manual/unknown states. |

A negative validation is a successful read with `checks_passed:false`, not an
invocation or transport failure. It must never satisfy a later mutation's
preconditions. Provider/schema/permission failures retain their normal typed
error results and nonzero CLI status. CI for another SHA, an earlier pipeline,
`has_conflicts:false`, and a legacy `pipeline` object cannot satisfy this profile.
Merged-result or merge-train pipelines whose SHA differs from the requested
source SHA remain blocked; this profile does not infer their equivalence.

This reports selected checks observed in one mutable provider response. GitLab's
asynchronous merge status is not a consistent transaction with the pipeline or
an exhaustive independent audit of every project rule. The provenance revision
remains null: this is not immutable MR metadata. Even a passing result grants no
write authority, reserves no head, issues no proof, and reports no merge. An
eventual merge coordinator must repeat current checks, use the provider's atomic
source-SHA precondition and retain possible-write uncertainty after response loss.
Create/update head-guard semantics remain the existing open decision. Dedicated
sandbox evidence and the remainder of C14 remain required independently.

The unchanged pinned OpenAPI at GitLab
`2ff8d865e5016b14b724d1c2ce745f8300696192` declares MR `head_pipeline` as
`APIEntitiesCiPipeline` (upstream/openapi_v3.yaml:88458). Native ownership and
source digest remain those of [MR reads](semantics.md). The
[GitLab MR API](https://docs.gitlab.com/api/merge_requests/) documents
`head_pipeline` visibility and asynchronous merge status; checked 2026-09-10.
The local CLI has no public-document retrieval operation, so primary documentation
was read directly. No upstream input is refreshed by this change.

The typed values live in `adapters/gitlab/spec/ess/domains/merge_requests.yaml`.
No new persistent entity or entity relation is introduced. Runtime obligations
include every negative predicate, malformed/policy refusals, exact generated GET,
unchanged read behavior, and saved-credential reuse through the production CLI.
