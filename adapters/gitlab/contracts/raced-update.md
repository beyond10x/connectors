# Raced merge-request update

`merge_request.update` is the second native write in the GitLab phase, and the first
one the provider does not let us guard. It is exposed only through explicitly
selected private protocol two, alongside [guarded merge](guarded-merge.md).

`merge_request.create` is **not** bound here. It runs through the
[catalog provider](../../../docs/local-catalog-provider.md) from the pinned OpenAPI
source, under the same race boundary expressed as a declarative guard rather than a
handler; the same provider also runs update and merge that way from the shipped
selection set. This native binding is the endpoint-by-endpoint form that
`architecture-decision-record:declarative-http-provider-runtime` says not to extend,
and `epic:retire-native-gitlab-adapter` records its removal.

## Why this is not guarded merge

The pinned upstream `adapters/gitlab/upstream/openapi_v3.yaml` at GitLab `2ff8d865`
gives the merge body an explicit source-HEAD `sha` precondition. The update body has
none, and neither does the pinned GraphQL `update.rb`. A pinned-SHA update is
therefore a **preflight read, an unguarded mutation, and a postflight comparison**.

The operator accepted that boundary on 2026-09-13 rather than wait for a precondition
GitLab does not offer. `decision-blocker:gitlab-mr-create-update-head-guard` carries
the decision and the evidence behind it.

## What the implementation promises

Before dispatch, the native preflight reads the merge request through the same path
`merge_request.get` uses. A merge request that is not open, or whose head already
differs from the pinned SHA, is **refused before any write**. That is the only point
at which a head difference is definite.

After dispatch, the response is compared against the preflight identity and the
pinned SHA. A mismatch returns an uncertain outcome — possibly applied — and never a
refusal, because the provider may already have applied the write.

No corrective mutation is ever issued. There is no retry, no undo, and no second PUT
to tidy up a raced write.

## What it does not promise

**The postflight comparison is best effort, not detection.** A merge request's
recorded head is eventually consistent with its source branch. On 2026-09-13 a live
GitLab answered this PUT with the pinned head while the branch had already moved to a
new commit, and the move was visible on the very next read. The comparison passing
means no move was observed. It does not mean none happened.

Local idempotency and approval bind the exact intended input. They do not manufacture
a provider-side atomic precondition, and nothing here should be read as if they did.
