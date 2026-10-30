---
format: aep.planning-md/3
id: story:served-keyed-writes-under-a-standing-approval
kind: story
status: active
title: Served keyed writes under a standing approval
summary: idempotency_key on /v1alpha2/invoke and a standing grant for the served placement
relations:
- serves: vision:independent-contract-adapters
- implements: story:standing-write-approval-for-unattended-agents
- depends_on: story:served-instance-admits-a-service-account-token
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T03:14:35Z", actor: "human:timo", revision: 2, executor: "agent:conn-d243-c1", correlation: "wave-d243-c1"}
- {from: "proposed", to: "active", at: "2026-10-30T03:14:35Z", actor: "human:timo", revision: 3, executor: "agent:conn-d243-c1", correlation: "wave-d243-c1"}
---
## Outcome

A served instance performs a keyed write on `POST /v1alpha2/invoke` for an agent running
unattended, under a standing approval the operator configured once. The request carries
`idempotency_key`; the host keeps a keyed reservation, so a repeat of the same key and input
replays the original outcome without a second provider request, and the same key with other input
is refused. A write the grant does not cover is refused by name and makes no provider request.

This implements `story:standing-write-approval-for-unattended-agents` for the served placement.
The local CLI placement of that story (a grant signed on the operator's host and mounted into a
container) stays in that story.

Wave D243-C1, unit C3. Depends on C2 (the admitted principal).

## Decided

- `decision-blocker:standing-write-approval-mode`, option A (Timo, 2026-10-22): explicit grant,
  exact scope (instance, operation, target), expiry, revocation, rate cap, grant id recorded with
  every write.
- Served placement (wave D243-C1): the grant is part of the instance's mounted configuration, a
  file the operator writes and the host reads on every call. A removed grant, or one listed as
  revoked, is a revocation from the next call on, with no restart. The served host signs nothing
  and is issued nothing: the file's authority is the operator's deployment, the same authority
  that configures the instance and its credential.
- The target is a set of exact input members (JSON pointers to values, such as `/owner` and
  `/repo`); an input whose named member differs, or is missing, is outside the grant.

## Scope

Design first: `architecture-decision-record:standing-scoped-approval` (the served placement),
`ess/domains/standing_approvals.yaml`, the contract text in
`contracts/operations/v1alpha1/semantics.md` and `contracts/service/compatibility.md` section 2.1
(the binding now admits `idempotency_key` for a keyed operation and a standing grant for an
approval-gated one; `approval` and `executor` stay refused). Then
`crates/connectors-host/src/server/v1alpha2.rs`, `crates/connectors-host/src/local/service.rs`
(keyed reservation, rate count), `CHANGELOG.md`.

## Acceptance (named tests)

Against a fixture adapter that counts provider requests:
- `a_keyed_write_under_a_matching_grant_is_dispatched_once_and_records_the_grant_id`
- `the_same_key_and_input_replays_the_original_outcome_with_no_second_request`
- `the_same_key_with_other_input_is_refused_conflict_with_no_request`
- `a_keyed_write_without_a_key_is_refused_with_no_request`
- `a_write_of_another_operation_is_refused_approval_required_with_no_request`
- `a_write_to_another_target_is_refused_with_no_request`
- `a_write_missing_a_target_member_is_refused_with_no_request`
- `a_write_after_not_after_is_refused_with_no_request`
- `a_write_after_revocation_is_refused_with_no_request`
- `a_write_over_max_per_hour_is_refused_with_no_request`
- `a_write_with_no_grant_is_refused_approval_required_with_no_request`
- `an_approval_member_is_still_refused_unsupported`
- `reads_are_unchanged_by_a_standing_grant`

Conformance scenarios in `contracts/service/compatibility.md`:
`v1alpha2-invoke-keyed-write-under-standing-grant`, `v1alpha2-invoke-keyed-replay`,
`v1alpha2-invoke-write-outside-grant-refused`; `v1alpha2-invoke-refuses-idempotency-key` changes
to a non-keyed operation and is listed as changed in `CHANGELOG.md`.

## Review

An adversary pass on the scope check and a security-reviewer pass before the wave's pull request.
