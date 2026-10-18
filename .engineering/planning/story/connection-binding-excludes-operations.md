---
format: aep.planning-md/3
id: story:connection-binding-excludes-operations
kind: story
status: draft
title: A bundle- or selection-only change leaves a connection usable; its credential stays bound to authority, profile, identity and executable
relations:
- implements: architecture-decision-record:provider-definitions-and-runtimes
- serves: vision:independent-contract-adapters
- informed_by: story:connection-follows-configuration-upgrade
scope:
- confidence: cited
  path: adapters/catalog/src/local.rs
- confidence: cited
  path: adapters/catalog/tests/adversary_credential_pass1.rs
- confidence: cited
  path: adapters/catalog/tests/engine_adversary_pass2.rs
- confidence: cited
  path: adapters/catalog/tests/gateway_prefix.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime/revision_recovery.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime/revision_recovery_adversary.rs
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: cited
  path: apps/connectors/src/local/connections.rs
- confidence: cited
  path: contracts/cli/v1alpha1/scenarios.md
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-build/src/metadata_conformance.rs
- confidence: cited
  path: crates/connectors-host/src/local/config.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/er.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/registry.sql
- confidence: cited
  path: crates/connectors-host/src/local/owner.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/cardinality_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/lifecycle.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/observation.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/revalidation.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/stale_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/upgrade_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/use_and_retirement.rs
- confidence: cited
  path: crates/connectors-host/src/local/runtime.rs
- confidence: cited
  path: docs/catalog-slack.md
- confidence: cited
  path: docs/local-catalog-provider.md
- confidence: cited
  path: docs/local-connection-registry.md
- confidence: cited
  path: ess/domains/auth_bindings.yaml
- confidence: cited
  path: ess/domains/cli.yaml
- confidence: cited
  path: website/docs/concepts/local-runtime.md
revision: 5
---
## Problem

A connection is bound to the whole configuration revision of its instance. The catalog provider
folds bundle, vendored source, selection (operations and feed), endpoint, CA, token CA, auth block,
allowed hosts and request prefix into one digest (`adapters/catalog/src/local.rs` `Local::configure`,
the digest at about lines 1157-1186), and `Binding` (`crates/connectors-host/src/local/registry.rs`)
stores only that digest beside authority and profile. Any move of it makes the connection `pending`
with `stale = true` (`registry/observation.rs` `observe`) and refuses reads, reconnect and
approvals until `connections revalidate` (`docs/local-connection-registry.md` "Configuration
upgrades"). A selection gaining one operation therefore costs the operator an explicit step,
although the credential, the host and the identity are unchanged. The provider executable digest is
in neither the revision nor the binding (`config.rs` `Adapter::selection` uses it for descriptions
and approvals only), although the documentation says a new executable stales the connection.

## Outcome

`architecture-decision-record:provider-definitions-and-runtimes` D12 points 1 and 2. The binding
records a credential-side digest (endpoint authority, instance id, profile declaration revision,
CA, token CA, allowed hosts, request prefix) and the provider executable digest beside the existing
`configuration_revision`. Then:

| Change | Result |
|---|---|
| bundle, source or selection only | connection not stale; its next use or revalidation follows the new revision with no credential source and no question |
| executable digest only | connection not stale; follows after the new executable's validation returns the recorded identity; a different identity refuses as today's `identity_mismatch` and changes nothing |
| CA, token CA, allowed hosts or request prefix | today's rule: `pending`, `stale = true`, explicit `connections revalidate` |
| authority or profile authentication | today's rule: `lifecycle_conflict` with today's `next_action` |

A connection saved before this change (no credential-side digest stored) keeps today's rule until
its first revalidation records one; nothing is migrated by guess. Approvals keep their binding to
the full configuration revision and executable selection. `configuration_revision` keeps its inputs
and its values for every existing configuration; the credential-side digest is a new value beside
it, not a replacement.

## Acceptance

A catalog connection saved under selection S1 lists as `ready`, not `stale`, after only its
selection changes to S2 (one added read), and that added read succeeds with no `connections
revalidate`, no credential source and no prompt
(`adapters/catalog/tests/local_runtime/revision_recovery.rs`, a new case beside the existing ones).

## Checks beside the acceptance

Each row of the Outcome table other than the first has its own test in the same file, and the
story is not implemented while any of them fails:

- a changed CA file: the connection lists `pending`/`stale = true` and the read is refused until
  `connections revalidate`;
- only the provider executable replaced (same bundle and selection): `ready`, and the read is
  served after one automatic validation; with a provider that answers a different identity,
  `identity_mismatch` and nothing changes;
- a connection saved before this change: today's rule until its first revalidation.

## Scope

Cited (from the survey of 2026-10-18):

- `adapters/catalog/src/local.rs`: `Local::configure` (split the digest inputs; print the
  credential-side digest in `--print-local-bootstrap`), `derived`.
- `crates/connectors-host/src/local/config.rs`: `Installation::derive`, `Installation::entry`,
  `InstanceTable::pinned`, `Adapter`.
- `crates/connectors-host/src/local/registry.rs` (`Binding`, `encode_stored`/`decode_stored`),
  `registry/observation.rs` (`observe`, `approval_target`), `registry/use_and_retirement.rs`
  (`admitted_read`), `registry/lifecycle.rs` (`begin_repair`, `admit_allocation`, `register`),
  `registry/revalidation.rs` (`changed`, `upgradable`, `follow_instance`, `finish_revalidation`).
- `crates/connectors-host/src/local/metadata/registry.sql`, `metadata/er.rs` (binding projection),
  `crates/connectors-host/src/local/owner.rs` (`saved_binding`, launch binding), `runtime.rs`
  (`Bootstrap::binding`).
- `apps/connectors/src/local/connections.rs` (`registry_failure`, list/describe).
- Tests that pin the old rule, each either kept (credential side) or changed to the new one:
  `registry/upgrade_tests.rs`, `registry/stale_tests.rs`, `registry/cardinality_tests.rs`,
  `adapters/catalog/tests/engine_adversary_pass2.rs`, `adversary_credential_pass1.rs`,
  `gateway_prefix.rs`, `adapters/catalog/tests/local_runtime/revision_recovery.rs`,
  `revision_recovery_adversary.rs`, `apps/connectors/src/local.rs` (test near line 957).
- Contract and model: `contracts/cli/v1alpha1/semantics.md` (the connection-revision paragraphs
  near lines 1077-1131 and the next-action table near 463), `contracts/cli/v1alpha1/scenarios.md`
  (C05), `ess/domains/auth_bindings.yaml` (binding fields), `ess/domains/cli.yaml` (connection
  state), with `ess specify validate` and regenerated outputs.
- Documentation: `docs/local-connection-registry.md` "Configuration upgrades",
  `docs/local-catalog-provider.md` (bundle rebuild then revalidate), `docs/catalog-slack.md`
  (selection change then stale), `website/docs/concepts/local-runtime.md`; generated references
  through `connectors-docs generate`.

Inferred: `crates/connectors-build/src/metadata_conformance.rs` if the stored binding shape is
pinned there.

## Not in scope

Approval binding; invoking a definition file (`story:invoke-definition-file`); reusing a credential
across instances (`story:invoke-definition-reuse-credential`); changing the refusal for an
authority or profile change.

## Why one story

The digest split, the stored field, the classification and the documentation change one rule; a
split would leave a stored field nothing reads or a rule nothing can observe.

## Shared surfaces

Recorded scope collides with these open stories; `aep plan artifact waves` keeps each pair apart,
and the order is:

| Story | Shared | Order and reason |
|---|---|---|
| `story:configuration-change-error` (draft) | `registry/lifecycle.rs`, `docs/local-catalog-provider.md`, the `lifecycle_conflict` refusal | that story lands after this one (`depends_on` edge on it): this story removes the bundle- and selection-only case its refusal is about, and its "use a new instance id" advice must then name only credential-side changes |
| `story:catalog-client-credentials-profile` (proposed) | `adapters/catalog/src/local.rs`, `docs/local-catalog-provider.md` | either order; this story keeps `configuration_revision` values unchanged, so that story's `oauth_existing_configuration_revisions_unchanged` stays green; the second to land rebases |
| `story:catalog-zendesk-reads`, `story:catalog-validates-enum-and-date-time` (draft) | `docs/local-catalog-provider.md` | either order; this story changes only the configuration-upgrade section of that page |
| `story:spec-models-operation-admission` (draft) | `metadata/er.rs`, `crates/connectors-build/src/metadata_conformance.rs` (inferred there) | either order; the second to land regenerates the metadata model and rebases the binding projection |
| `story:mcp-cli-journey-discovery-contract` (draft) | `contracts/cli/v1alpha1/semantics.md` (inferred there) | either order; that story appends an MCP section, this one edits the connection-revision paragraphs |

## Relations

Builds on the upgrade-in-place path of `story:connection-follows-configuration-upgrade`, whose
revalidation is in the tree (`registry/revalidation.rs`) although that story is still `proposed`.
