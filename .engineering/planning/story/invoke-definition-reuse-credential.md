---
format: aep.planning-md/3
id: story:invoke-definition-reuse-credential
kind: story
status: active
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
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-10-18T13:57:18Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-10-18T13:57:18Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":3}}}
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
the definition's instance has no connection, or its authority or profile authentication changed,
and at least one non-revoked connection of the same connector id exists, the command asks once:

- reuse: lists the candidate connections (instance id, authority, validated identity; never
  material); the chosen connection's current custody version is copied inside the owner to a new
  connection of the definition's instance, the new profile's validation runs against it, and the
  connection is published only when validation succeeds and the validated identity equals the
  source connection's; otherwise nothing is published and the operator is offered connect fresh;
- connect fresh: the acquisition of `story:invoke-definition-file`.

A source connection under another instance is unchanged either way; when the source is the
definition instance's own old connection, it is copied first and then revoked (see One connection
per instance below). The question is never asked when a connection bound
to the instance's authority and profile exists, and never without a terminal (refusal with a
`next_action` as in the previous story). A changed authority sends the stored credential to the new
host only after the operator chose reuse for that connection and host, and the prompt names the new
host.

Candidates (drafter's decision, 2026-10-18): D12 says "a credential already stored in connectors";
this story narrows that to non-revoked connections of the same connector id, because only those
hold material the definition's profile can validate (a credential of another connector has no
declared meaning there). Credentials of other connectors are listed under Not in scope; widening
the set is a later decision for the operator.

One connection per instance: when the definition's instance still holds a connection under the old
authority or profile, the question says that connection will be revoked, and either answer revokes
it before publishing the new one. This is the one place where "only a new instance id helps"
(`docs/local-connection-registry.md`, "Configuration upgrades") gives way, and only for the
definition's own instance after the owner's explicit answer; configured instances keep the rule.

## Acceptance

With a token connection under a configured instance of connector C at host H and a hand-written
definition of C at H, `operations invoke --definition` asks once, the answer "reuse" yields the
operation's result without a token prompt, a new connection under the definition's instance with the
same validated identity, and the configured instance's connection unchanged.

## Checks beside the acceptance

Separate tests in the same file; the story is not implemented while any fails:

- the same with a provider that answers a different identity publishes nothing and offers connect
  fresh;
- with the definition's own instance holding a connection under host H and the file changed to
  host H2, the question names H2 and the revocation; "reuse" copies that connection's credential,
  revokes the old connection and publishes the new one under H2 only when the identity matches.

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

Implemented on unit/invoke-definition-reuse-credential (2026-10-18): the acceptance and the first
check beside it, both as namespace journeys. Not implemented, pending an operator decision: the
second check (the definition's own instance holding a connection under host H, the file changed to
H2, "reuse" copying it to H2 and revoking the old one). It sends a stored credential to a host its
connection was never bound to, which the dispatcher's security floor for this unit ("the same
provider authority/profile declaration") excludes. Until decided, an authority or profile change
of the definition's own instance stays `lifecycle_conflict` as in story:invoke-definition-file, and
the story stays active.

Decisions taken inside the floor: a copy, never a shared custody entry (revoking either leaves the
other); only static-entry profiles whose lowered definition names no URL (no token endpoint)
receive a copy; only an invoked definition's instance is a target; a legacy `[adapters]` entry is
no connector's instance; the copy is published only while the source is still what was read.
