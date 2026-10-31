---
format: aep.planning-md/3
id: story:served-keyed-writes-under-a-standing-approval
kind: story
status: implemented
title: Served keyed writes under a standing approval
summary: idempotency_key on /v1alpha2/invoke and a standing grant for the served placement
relations:
- serves: vision:independent-contract-adapters
- implements: story:standing-write-approval-for-unattended-agents
- depends_on: story:served-instance-admits-a-service-account-token
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T03:14:35Z", actor: "human:timo", revision: 2, executor: "agent:conn-d243-c1", correlation: "wave-d243-c1"}
- {from: "proposed", to: "active", at: "2026-10-30T03:14:35Z", actor: "human:timo", revision: 3, executor: "agent:conn-d243-c1", correlation: "wave-d243-c1"}
- {from: "active", to: "implemented", at: "2026-10-30T10:11:28Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}, executor: "agent:conn-d265-c4", correlation: "wave-d265-c4"}
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

## Implemented (wave D243-C1, unit C3)

- Opt-in, no breaking change. A host whose service configuration has no `standing_grants`
  behaves as at the wave base on every route: an unkeyed write on `POST /v1alpha2/invoke` is
  dispatched once and names its attempt, `idempotency_key` is refused `unsupported` with
  `request_id: null` (`v1alpha2-invoke-refuses-idempotency-key` and
  `v1alpha2-invoke-keyed-or-approval-operation-unsupported` keep their base meaning), and
  `Client::invoke_v1alpha2` still carries writes. The acceptance line "the scenario
  `v1alpha2-invoke-refuses-idempotency-key` changes ... listed as changed" was replaced by this
  (coordinator rework: the change broke loom's unkeyed writes).
- With `standing_grants` set (requires `state`; start refuses it without), every operation
  with the `mutation` profile is keyed on `POST /v1alpha2/invoke`: the legacy descriptor
  carries no idempotency or approval metadata, so "keyed" and "approval-gated" are both
  realized as the `mutation` profile. `approval` and `executor` stay refused `unsupported`.
  `POST /v1/invoke` refuses writes there as `unsupported` (HTTP 501), because it records no
  attempt and reads no grant.
- Refusals before the anchor on such a host, in order, each with no attempt and no provider
  request: `unsupported` (key on a read), `invalid_input` (write without key),
  `idempotency_conflict` (key live for another request), `unavailable` (key store, grant file
  or use count unreadable), `approval_required` (no covering grant, or no file at the path),
  `rate_limited` with `retry_after_seconds` (covering grants at `max_per_hour` in the sliding
  3,600 s window). A winner recheck precedes `approval_required` and `rate_limited`.
- Grant file: a plain file or a Kubernetes Secret/ConfigMap volume key. On Linux the volume's
  `<name> -> ..data/<name>` link is read through `..data` by the custody crate's
  `mounted::open_delivered`, the `mounted_secret` binding's own checks; no other symlink is
  followed, and every parent directory must be owned by root or the host's user and not
  writable by others (the file's own directory only on a read-only mount).
- Concurrency: one host process admits requests with one (caller, key) one at a time, from the
  lookup until the key is reserved with its attempt or the request is refused. A concurrent
  duplicate of an admitted request is a replay, never `rate_limited` (also after a
  revocation), and takes no grant use; a concurrent request with other input is
  `idempotency_conflict` before the anchor (`audit_ref: null`). Across processes sharing one
  state directory, the winner recheck and the attempt reservation still decide, after the
  anchor.
- Replay of a pending original: the replay gives back its service slot and waits at most 25 s
  (then `outcome_unknown`, cause timeout/observation); at most 4 replays wait per attempt, a
  fifth answers `outcome_unknown` at once (cause capacity/observation); an original this
  process does not run (restart or crash) answers `outcome_unknown` at once (cause
  unavailable/observation). Chosen: `outcome_unknown` rather than a second write.
- Durability: keys, attempts (metadata.sqlite3) and hourly use counts (standing-grants.sqlite3,
  created 0600, one connection behind a mutex, writes in IMMEDIATE transactions) survive a
  restart. The HTTP host does not recover a pending attempt after a restart and does not
  expire keys: a used key is never admitted for another request.
- A use is released when no attempt is recorded (anchor refused, prepare failed, concurrent
  winner); a use whose attempt was recorded stays counted even if the attempt aborts. A use
  that names no attempt is deleted once its hour has passed, at most 256 per take; a use bound
  to an attempt is kept with the attempt.
- Catalog: `connectors-catalog-provider --config` with `service.standing_grants` describes and
  dispatches the instance's writes (prepare_write, executed once); writes whose answer carries
  a created credential are refused at start (no custody target on the served host). Without a
  grant file it serves reads only, as before.
- Applied keyed results up to 64 KiB are retained in the attempt settlement for replay; a
  larger one replays as `capacity` with `applied`. A host without grants settles as at base.

Evidence: crates/connectors-host/tests/http_standing_grant.rs (20 keyed-host tests plus 3 for
a host without grants and the legacy route); http_standing_grant_adversary.rs (7) and
http_standing_grant_review.rs (5), the adversary's and security reviewer's tests;
crates/connectors-host/src/local/standing.rs unit tests (grant file layouts, use pruning, file
mode); crates/connectors-custody/src/mounted.rs `--lib mounted`;
crates/connectors-conformance/tests/v1alpha2_keyed_write_scenario.rs (the three scenarios);
crates/connectors-core/tests/wire_v1alpha2_keyed.rs (the keyed codec);
adapters/catalog/tests/served.rs `a_served_catalog_write_under_a_standing_grant_is_sent_once`.
Base-era tests (`http_mutation_ledger_adversary`, `service`, `http_audit_anchor*`,
`http_service_account_token*`, `client_v1alpha2`, `v1alpha2_write_scenario`,
`wire_v1alpha2_adversary`) are byte-identical to the wave base and pass.

Not done (follow-up candidates):
- Key retention expiry (operations §5.1, 86,400 s) on the HTTP host: keys never expire here.
  Blocked on: §5.1 forbids expiry proof from a clock uncertain across restart (the served host
  has only its wall clock), an attempt row is referenced by its execution audit, which audit.md
  §4 forbids deleting or rewriting, and clearing a retained result needs a new ESS command.
  Options: give the served host a trusted time quorum plus an ESS command that retires a
  retained result, or amend §5.1 for the HTTP host.
- Restart recovery of a pending keyed attempt on the HTTP host (today: `outcome_unknown` at
  once).
- loom's service transport sends unkeyed writes through `Client::invoke_v1alpha2`; it needs
  `invoke_v1alpha2_keyed` before it can use a host with standing grants.
- Local CLI placement stays in story:standing-write-approval-for-unattended-agents.
