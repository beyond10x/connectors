---
format: aep.planning-md/3
id: story:invoke-definition-reuse-credential
kind: story
status: implemented
title: operations invoke --definition offers once to reuse a stored credential of the same connector
relations:
- implements: architecture-decision-record:provider-definitions-and-runtimes
- serves: vision:independent-contract-adapters
- depends_on: story:invoke-definition-file
scope:
- confidence: cited
  path: adapters/catalog/tests/local_runtime/invoke_definition.rs
- confidence: cited
  path: apps/connectors/src/local/definition.rs
- confidence: cited
  path: contracts/auth/custody/v1alpha1/semantics.md
- confidence: cited
  path: contracts/cli/v1alpha1/owner.md
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-build/src/ignored.rs
- confidence: cited
  path: crates/connectors-host/src/local/definitions.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
- confidence: cited
  path: crates/connectors-host/src/local/protected.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/lifecycle.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/reuse.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/use_and_retirement.rs
- confidence: cited
  path: docs/local-connection-registry.md
- confidence: cited
  path: ess/domains/auth_bindings.yaml
- confidence: cited
  path: ess/domains/cli.yaml
- confidence: cited
  path: website/docs/guides/invoke-a-definition-file.md
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-10-18T13:57:18Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-10-18T13:57:18Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "active", to: "implemented", at: "2026-10-18T15:42:53Z", actor: "human:timo", revision: 16, decided_on: {"recorded":{"test_result":4,"review_outcome":7}}}
---
## Problem

After `story:invoke-definition-file`, `operations invoke --definition` connects fresh whenever its
instance has no matching connection. An operator who already holds a credential for the same
connector (a configured instance, or an earlier version of the same definition whose authority or
profile has since changed) is asked to enter it again, and an authority or profile change refuses as
`lifecycle_conflict` with "only a new instance id helps" (`docs/local-connection-registry.md`
"Configuration upgrades").

## Outcome

`architecture-decision-record:provider-definitions-and-runtimes` D12 point 3, the question. When
the definition's instance has no connection and at least one usable connection of the same
connector id exists under another instance, at the same provider authority and with the same
profile authentication, the command asks once:

- reuse: lists the candidate connections (instance id, authority, validated identity; never
  material); the chosen connection's current custody version is copied inside the owner to a new
  connection of the definition's instance, the new profile's validation runs against it, and the
  connection is published only when validation succeeds and the validated identity equals the
  source connection's; otherwise nothing is published and the operator is offered connect fresh;
- connect fresh: the acquisition of `story:invoke-definition-file`.

The source connection is unchanged either way. The question is never asked when a connection
bound to the instance's authority and profile exists, and never without a terminal (refusal with a
`next_action` as in the previous story).

Candidates (drafter's decision, 2026-10-18): D12 says "a credential already stored in connectors";
this story narrows that to non-revoked connections of the same connector id, because only those
hold material the definition's profile can validate (a credential of another connector has no
declared meaning there). Credentials of other connectors are listed under Not in scope; widening
the set is a later decision for the operator.

Cross-host reuse rejected (operator decision A, 2026-10-18): a stored credential is never reused
for another provider authority, including the definition's own instance whose file moved to
another host. D12 point 1 binds a connection to the provider authority (endpoint host and instance
id), and the binding table the operator approved keeps that row because it stops a token being sent
to a different host; reusing across hosts would undo it. When the definition's own instance holds
a connection under the old authority, the command offers no reuse, asks nothing, sends nothing and
answers `lifecycle_conflict` with the `next_action` a `connections connect` of the instance names,
as in `story:invoke-definition-file`. The drafted alternative (copy the old connection to the new
host after a prompt naming it, then revoke the old one) was option B and was not taken.

## Acceptance

With a token connection under a configured instance of connector C at host H and a hand-written
definition of C at H, `operations invoke --definition` asks once, the answer "reuse" yields the
operation's result without a token prompt, a new connection under the definition's instance with the
same validated identity, and the configured instance's connection unchanged.

## Checks beside the acceptance

Separate tests in the same file; the story is not implemented while any fails:

- the same with a provider that answers a different identity publishes nothing and offers connect
  fresh;
- with the definition's own instance holding a connection under host H, a configured instance's
  connection at H present, and the file changed to host H2, no reuse is offered and nothing is
  asked or sent: the command answers `lifecycle_conflict` with the `next_action` a `connections
  connect` of the instance names, and both existing connections are unchanged (cross-host reuse
  rejected, see Outcome).

## Scope

Inferred (no custody copy exists today; the scoper should confirm):

- `crates/connectors-host/src/local/registry/lifecycle.rs` (a reuse allocation beside `register`),
  `registry/revalidation.rs` (validation of a captured custody version), `crates/connectors-host/src/local/keyring/`
  (custody copy of one version), `crates/connectors-host/src/local/owner/supervisor.rs` and
  `owner/transport.rs` (a reuse request).
- `apps/connectors/src/local/operations.rs` and `apps/connectors/src/local/session.rs` (the
  question and its answers), `crates/connectors-host/src/local/protected.rs` (`terminal`, the hidden-input prompt helper).
- `contracts/cli/v1alpha1/semantics.md` (the reuse answer and its refusals),
  `contracts/auth/custody/v1alpha1/semantics.md` (custody copy), `ess/domains/auth_bindings.yaml`
  if the copy is recorded on the new connection.
- Tests: `apps/connectors/tests/invoke_definition.rs` (from the previous story),
  `adapters/catalog/tests/local_runtime/cli_journey.rs`.
- Documentation: `docs/local-connection-registry.md` and the guide section of the previous story.

## Not in scope

Changing the refusal for `connections connect`, `reconnect` or reads under configured instances;
moving a credential rather than copying it; reusing a credential of another connector id; OAuth
consent from definition parameters (D6).

## Shared surfaces

The scope is inferred throughout: no custody copy exists to cite, and the implementor or a `story-scoper` run confirms it before a wave selects this story.

| Story | Shared (inferred here) | Order and reason |
|---|---|---|
| `story:invoke-definition-file`, `story:connection-binding-excludes-operations` | `registry/lifecycle.rs`, `registry/revalidation.rs`, `contracts/cli/v1alpha1/semantics.md`, `apps/connectors/src/local/operations.rs`, `session.rs` | after both (`depends_on` chain) |
| `story:configuration-change-error` (draft) | `registry/lifecycle.rs`, `contracts/cli/v1alpha1/semantics.md` | either order after `story:connection-binding-excludes-operations`; this story touches the reuse allocation, that one the connect refusal code |
| `story:mcp-cli-journey-discovery-contract` (draft) | `contracts/cli/v1alpha1/semantics.md` | either order; different sections (MCP journey there, the reuse answer here) |

## Delivery status

Implemented on unit/invoke-definition-reuse-credential (2026-10-18). The acceptance and both
checks beside it are namespace journeys in adapters/catalog/tests/local_runtime/invoke_definition.rs:
`a_stored_credential_of_a_configured_instance_is_reused_without_a_token_prompt`,
`a_reuse_the_provider_answers_with_another_identity_publishes_nothing_and_offers_connect_fresh` and
`a_definition_moved_to_another_host_is_offered_no_reuse_and_answers_lifecycle_conflict` (check 2 as
restated after operator decision A; a pinning test, since the code never offered reuse there).

Decisions taken inside the floor: a copy, never a shared custody entry (revoking either leaves the
other); only static-entry profiles whose lowered definition names no URL (no token endpoint)
receive a copy; only an invoked definition's instance is a target; a legacy `[adapters]` entry is
no connector's instance; the copy is published only while the source is still what was read.

Observed, not changed here: after an authority change, `connections connect` of the definition's
instance is refused too (D12 point 2 keeps "only a new instance id helps", and a definition's
instance id is fixed), so the moved definition has no connect route under that id.
