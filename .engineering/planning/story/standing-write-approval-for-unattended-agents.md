---
format: aep.planning-md/3
id: story:standing-write-approval-for-unattended-agents
kind: story
status: draft
title: A standing, scoped write approval for an agent running unattended
summary: One connection, one operation, input constraints, a rate cap and an expiry, set once by a person
relations:
- decomposes: epic:connectors-with-claude
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-stdio-runtime-branch
- depends_on: story:claude-connects-to-the-mcp-server
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: inferred
  path: apps/connectors/spec/cli.yaml
- confidence: cited
  path: apps/connectors/src/local/approvals.rs
- confidence: cited
  path: apps/connectors/tests/standing_approval.rs
- confidence: cited
  path: contracts/operations/v1alpha1/scenarios
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-host/src/local/approvals
- confidence: cited
  path: crates/connectors-host/src/local/owner/approval_issuance.rs
- confidence: cited
  path: docs/local-container.md
- confidence: cited
  path: ess/domains/approval_issuers.yaml
- confidence: cited
  path: ess/domains/standing_approvals.yaml
- confidence: cited
  path: ess/system.yaml
revision: 23
---
## Outcome

An agent running unattended, with Connectors in its container, sends a Slack message under a
standing approval a person granted once. A call outside that grant is refused by name and makes no
provider request.

Asked by the Loom Kubernetes agent design (question Q2): its `reply.send` effect in `auto` mode
needs this. Until it lands, that agent runs in per-send approval mode.

## Decided

decision-blocker:standing-write-approval-mode, option A (Timo, 2026-10-22). The bounds:
- The operator grants a standing approval explicitly; nothing grants one by default.
- Its scope is exact: connector instance, operation and target (for Slack, a channel id).
- It has an expiry (`not_after`) and can be revoked before it.
- Every send under it is recorded in the attempt ledger with the grant id.
- It never covers an operation, instance or target outside its scope.
- Option A also named a rate cap (`max_per_hour`).

The per-input approvals of `contracts/operations/v1alpha1/semantics.md` steps 5 and 8 stay as they
are. The standing approval is an added approval mode, and `CHANGELOG.md` lists it under features.

## Design first, in this story

1. An ADR, `architecture-decision-record:standing-scoped-approval`, created through the planning CLI: the added mode, why steps 5 and 8 do not apply to it, and what replaces them (scope
   match, expiry, revocation, rate count, grant id in the ledger). It states how a changed grant file
   is read: `mounted_secret` treats changed bytes of credential material as a rotation that waits for
   `revalidate_connection` (`docs/local-mounted-secret-custody.md`); the ADR decides whether a changed
   or removed grant file is instead read on the next call as a revocation, with no revalidate, and
   `a_call_after_revocation_is_refused_with_no_request` holds that answer.
2. The standing approval modelled in ESS beside `ess/domains/approval_issuers.yaml`, through
   `ess:specifying`: grant id, scope, `not_after`, revocation, rate counter. It is a new noun.
3. The contract text in `contracts/operations/v1alpha1/semantics.md`.
4. Then the implementation, in the same pull request only after items 1-3 are reviewed in it.

The hourly count is kept in the state directory. In a container with a read-only root, that
directory is the one story:state-directory-on-mounted-volume provides; until that story lands,
the count lives in the state directory the operator mounts writable today
(`docs/local-container.md:43-60`).

Revocation reaches the container as changed bytes of the mounted grant file. That a Kubernetes
Secret update reaches a mounted volume is inferred, not observed; story:mounted-custody-kind-secret-volume
observes it.

## Shape

- `connectors approvals grant-standing` on the operator's host, signed with the existing
  approval-signing key; `connectors approvals revoke-standing <grant id>`.
- The grant is a file the container mounts and reads under `mounted_secret` custody
  (`contracts/auth/custody/v1alpha1/semantics.md` §5.2), which already refuses signing in the
  container. Revocation reaches the container as a replaced or removed file, which `mounted_secret`
  already detects as changed bytes.
- Verified on each call before any provider request: signature, instance, operation, target,
  `not_after`, not revoked, and the count of calls in the current hour.

## Review

An adversary pass on the scope check before the story merges: any call the grant does not name
(another operation, instance or channel, an input that moves the target, a call after expiry or
revocation) must be refused with no provider request.

## Acceptance (named conformance scenarios)

Design (in the story's pull request, before the implementation commits):
- `architecture-decision-record:standing-scoped-approval` is `accepted`.
- `ess/domains/standing_approvals.yaml` (new, beside `approval_issuers.yaml`) validates in the
  repository gate (`cargo run --locked -p connectors-build -- gate`).
- Four authored traces in `contracts/operations/v1alpha1/scenarios/` (a directory of
  `AUTHORED_DIRECTORIES`, `crates/connectors-build/src/gate.rs:318`): `standing-approval-scope-match.yaml`,
  `standing-approval-expiry.yaml`, `standing-approval-revocation.yaml`, `standing-approval-rate.yaml`.
  The gate's line "executed N authored scenarios, P passed" (`gate.rs:449-452`) shows N and P each
  at least 4 higher at the story's head than at its base, and none of the four is in
  `AUTHORED_KNOWN_FAILING` (`gate.rs:463`).
- Before the story merges, a `review-result` from an `aep:adversary` pass relates `reviews:` this
  story, covering the scope check as § Review says, and each of its findings carries a
  `review_outcome`.

`apps/connectors/tests/standing_approval.rs` (new), in `cargo test`, the CLI under the
`mounted_secret` jail of `apps/connectors/tests/support/mounted.rs` against a loopback Slack
listener; "no request" is the listener's request count staying 0:
- `a_send_to_the_granted_channel_is_admitted_and_recorded_with_the_grant_id`
- `a_send_to_another_channel_is_refused_with_no_request`
- `an_input_that_moves_the_target_is_refused_with_no_request`: the granted channel id given as a
  channel name, as a user id (which opens a DM) or with a `thread_ts` from another channel.
- `a_standing_grant_for_one_operation_admits_no_other`
- `a_standing_grant_for_one_instance_admits_no_other`
- `a_grant_with_a_changed_byte_is_refused_with_no_request`
- `a_grant_signed_by_a_key_the_owner_did_not_issue_is_refused_with_no_request`
- `a_call_after_not_after_is_refused_with_no_request`
- `a_call_after_revocation_is_refused_with_no_request`: the test issues the grant with `connectors approvals grant-standing`, sends once (admitted), runs `connectors approvals revoke-standing <grant id>` before `not_after`, and the next send is refused.
- `a_call_over_max_per_hour_is_refused_with_no_request`
- `a_send_with_no_grant_file_is_refused_approval_required_with_no_request`

`per_input_approvals_are_unchanged`: the story's diff leaves these files unmodified and they pass:
`crates/connectors-host/src/local/approvals/tests.rs`,
`crates/connectors-host/src/local/approval_policy/tests.rs`,
`adapters/catalog/tests/local_runtime/slack_approval.rs`,
`apps/connectors/tests/approval_field_adversary.rs`,
`apps/connectors/tests/approval_field_adversary_2.rs`.

## Order

- Depends on story:mcp-stdio-runtime-branch: both change `crates/connectors-host/src/local` and
  `apps/connectors/src`.
- Depends on story:claude-connects-to-the-mcp-server: the orchestrator placed this story in the
  wave after the waves that carry story:mcp-stdio-runtime-branch and story:claude-connects-to-the-mcp-server
  (2026-10-22). No acceptance line here reads that story's output; the edge is the orchestrator's order.
- No dependency on story:mounted-custody-kind-secret-volume. The scenarios here revoke in the
  jail, where the grant file is replaced directly. Whether a Kubernetes Secret update reaches a
  running pod is that story's observation. This story adds a standing-grant section to
  `docs/local-container.md` that names the observation as inferred until that story records it.
