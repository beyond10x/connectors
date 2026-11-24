---
format: aep.planning-md/3
id: story:standing-write-approval-for-unattended-agents
kind: story
status: draft
title: A standing, scoped write approval for an agent running unattended
summary: 'The container part: a grant signed on the operator''s host is mounted under mounted_secret custody, verified in the container and never signed there'
relations:
- decomposes: epic:connectors-with-claude
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-stdio-runtime-branch
- depends_on: story:claude-connects-to-the-mcp-server
- depends_on: story:local-invoke-admits-a-keyed-write-under-a-standing-grant
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: apps/connectors/src/local/approvals.rs
- confidence: cited
  path: apps/connectors/tests/standing_approval.rs
- confidence: cited
  path: crates/connectors-host/src/local/approvals
- confidence: cited
  path: docs/local-container.md
revision: 32
---
## Outcome

An agent running unattended, with Connectors in its container, sends a Slack message under a
standing grant its operator signed on the operator's host and mounted into the container. The
container verifies the grant and cannot sign one. A call outside the grant is refused and makes
no provider request.

Asked by the Loom Kubernetes agent design (question Q2): its `reply.send` effect in `auto` mode
needs this. Until it lands, that agent runs in per-send approval mode.

## Narrowed, 2026-11-23

This story held the whole local placement. On D8 = B (the operator, 2026-11-23) the owner path
on a plain state directory became story:local-invoke-admits-a-keyed-write-under-a-standing-grant,
and the closed input became story:standing-grant-closes-the-input-to-listed-members. Moved to
them and no longer claimed here: the two commands and `list-standing`, the signed file and its
verification, the scope, expiry, revocation and rate checks, the answer member, the ESS model
and the contract text of the mode, the adversary pass on the scope check, and the test that
per-input approvals are unchanged. What stays here is the mount.

## Decided

- decision-blocker:standing-write-approval-mode, option A (the operator, 2026-10-22), and
  `architecture-decision-record:standing-scoped-approval` (accepted), decision 5: the grant is
  issued on the operator's host, mounted into the container under `mounted_secret` custody,
  verified there and never signed there; a changed or removed grant file is read on the next
  call as a revocation, not as a credential rotation waiting for `revalidate_connection`.
- The grant id is named in the answer and bound to the attempt in the uses store, as the local
  story decides (its D10). The first draft of this story said "in the attempt ledger".
- The case "a `thread_ts` from another channel" of the first draft is dropped: the local story
  admits `/body/thread_ts` with any value (its O4).

## Open design points, settled in this story before code

- Under `mounted_secret` custody the container holds no approval key
  (`apps/connectors/tests/mounted_custody.rs:266`, `mounted_refuses_refresh_and_approval_keys`).
  The grant's signature needs the public key in the container. Where it comes from (a mounted
  public key document, the published key set of the state directory) is not decided.
- The hourly count is kept in the state directory. In a container with a read-only root that
  directory is the one story:state-directory-on-mounted-volume (draft) provides; until it
  lands, the count lives in the state directory the operator mounts writable today
  (`docs/local-container.md:43-60`).
- That a Kubernetes Secret update reaches a mounted volume is inferred, not observed;
  story:mounted-custody-kind-secret-volume (draft) observes it.

## Shape

- `standing_grants` in the container's configuration names the mounted grant file. The file is
  read under the rules of `mounted_secret` custody
  (`contracts/auth/custody/v1alpha1/semantics.md` §5.2).
- `approvals grant-standing` and `approvals revoke-standing` are refused in the container, as
  the approval key commands are.
- Revocation reaches the container as a replaced or removed file.

## Acceptance

`apps/connectors/tests/standing_approval.rs` (new), in the test step, the CLI under the
`mounted_secret` jail of `apps/connectors/tests/support/mounted.rs` against a loopback Slack
listener; "no request" is the listener's request count not moving. The grant files are signed
by the test on the host side of the jail. Every test runs 60 s at most; the state of the jail
is built once per run.

- `a_send_under_a_mounted_grant_is_admitted_and_names_the_grant_id`
- `a_mounted_grant_with_a_changed_byte_is_refused_with_no_request`
- `a_send_with_no_mounted_grant_file_is_refused_approval_required_with_no_request`
- `a_replaced_grant_file_without_the_grant_is_a_revocation_on_the_next_call`: no
  `revalidate_connection` is needed, and the connection stays `ready`.
- `a_removed_grant_file_is_a_revocation_on_the_next_call`
- `grant_and_revoke_commands_are_refused_in_the_container`
- `docs/local-container.md` has a standing-grant section that names the Kubernetes observation
  as inferred until story:mounted-custody-kind-secret-volume records it.
- `CHANGELOG.md` has an `### Added` line for the mounted grant.

## Order

- Depends on story:local-invoke-admits-a-keyed-write-under-a-standing-grant: this story mounts
  the file that story signs and verifies.
- story:mcp-stdio-runtime-branch and story:claude-connects-to-the-mcp-server are implemented;
  the edges stay as the orchestrator placed them (2026-10-22).
- No dependency on story:mounted-custody-kind-secret-volume: the tests replace the file in the
  jail directly.
