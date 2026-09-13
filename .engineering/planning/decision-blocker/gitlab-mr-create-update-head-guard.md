---
format: aep.planning-md/1
id: decision-blocker:gitlab-mr-create-update-head-guard
kind: decision-blocker
status: cleared
title: Resolve the pinned-SHA guarantee for GitLab MR create and update
relations:
- blocks: initiative:complete-local-connectors
withholds: test_result
revision: 4
---
## Withheld result

The full C14 GitLab create/update acceptance cannot be claimed until the pinned-source SHA race semantics are selected and proved. This blocks that portion of initiative:complete-local-connectors, not independent ledger, audit, approval or validation implementation.

## Evidence

docs/recent-adapter-usage-20260909.md C14 requires validation and MR create/update/merge for a pinned SHA, changed-head refusal and no false merge claim. The selected immutable upstream is adapters/gitlab/upstream/openapi_v3.yaml at GitLab 2ff8d865e5016b14b724d1c2ce745f8300696192, SHA-256 f9e830bd3d2b99c49d60a7713fe1a64f5164418aca24b559287daab075beb530. Its complete create body RequestBody_b3b0a765ad52 at line 116565 and update body RequestBody_2b4971ab6883 at line 116667 have no SHA precondition. Merge body RequestBody_ce0a14f220cb at line 116791 explicitly provides a source-HEAD sha guard.

The same pinned [GraphQL create implementation](https://gitlab.com/gitlab-org/gitlab/-/raw/2ff8d865e5016b14b724d1c2ce745f8300696192/app/graphql/mutations/merge_requests/create.rb) and [update implementation](https://gitlab.com/gitlab-org/gitlab/-/raw/2ff8d865e5016b14b724d1c2ce745f8300696192/app/graphql/mutations/merge_requests/update.rb), read on 2026-09-10, expose no equivalent argument in those classes. This is evidence about these selected surfaces, not proof that no GitLab extension or future version could support a guard.

A preflight head read followed by an unguarded create/update leaves a race. A postflight read can report it but cannot truthfully say the mutation was refused if the provider already applied it. Local idempotency and approval bind exact intended input; they cannot create an absent provider-side atomic precondition.

## Clearance

The native adapter owner must record and review a concrete C14 interpretation and provider binding that either proves the required guard with admitted provider controls, or obtains the operator's explicit acceptance of a documented preflight/postflight race boundary. Preserve possible-write uncertainty and never implement automatic corrective mutations to conceal a race. A source-version upgrade is a separate reviewed input change, not an implicit vendor refresh. Clear this blocker only with that decision and checkable evidence; local fixtures or the completed specification milestone do not settle it.

## Decision — 2026-09-13

The operator accepted a documented preflight/postflight race boundary rather than
waiting for a provider guard that this pinned upstream does not offer. The accepted
interpretation of C14 for create and update is:

- A pinned-SHA create or update is **a preflight read, then an unguarded mutation,
  then a postflight read**. The window between the preflight and the mutation cannot
  be closed with the controls `adapters/gitlab/upstream/openapi_v3.yaml` at GitLab
  `2ff8d865` exposes. Merge keeps its atomic `sha` guard and is unaffected.
- A postflight read that finds the head changed reports the mutation as **possibly
  applied**. It is never reported as refused, because the provider may already have
  applied it. Possible-write uncertainty is preserved and surfaced, not resolved.
- **No corrective mutation is ever issued** to undo, retry or conceal a raced write.
- Local idempotency and approval bind the exact intended input. They do not
  manufacture a provider-side atomic precondition, and no document may imply they do.

This decision covers `merge_request.create` and `merge_request.update` on the pinned
source only. A source-version upgrade that introduced a precondition would be a
separate reviewed input change, and would narrow this boundary rather than extend it.

## What the sandbox showed, after the decision — 2026-09-13

The decision above says a postflight read that finds the head changed reports the
mutation as possibly applied. A live GitLab showed that the postflight read **may not
find it at all**.

The run: `merge_request.update` was invoked against merge request 9 pinned to head
`b6f6c959`. A commit moving `feature/conflicting` to `d7abb779` was pushed the moment
the preflight GET appeared in GitLab's access log, before the update PUT was sent.

- The PUT response reported `sha: b6f6c959` — the pinned head.
- The postflight comparison therefore passed and the write was classified `applied`.
- A read taken immediately afterwards reported `sha: d7abb779`.

A merge request's recorded head is eventually consistent with its source branch in
GitLab. The postflight read is a **best-effort** detector, not a guarantee: it can
confirm a head that did not move, and it can catch a move it happens to see, but it
cannot prove that no move occurred.

This does not reopen the decision — a guard the provider does not offer cannot be
built here — but it narrows what may be claimed. The operator accepted a race; the
evidence shows the race can also pass unnoticed. No document may describe the
postflight read as detection.
