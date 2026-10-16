---
format: aep.planning-md/3
id: story:setup-migrate-adapter-entries
kind: story
status: active
title: connectors setup migrate rewrites adapter entries as connector instances without re-consent
relations:
- decomposes: epic:connector-vocabulary
- depends_on: story:connector-config-dual-read
- depends_on: story:cli-connector-instance-selectors
- informed_by: migration-plan:connector-vocabulary
- depends_on: story:ess-connector-vocabulary-model
- depends_on: story:catalog-connector-declarations
- depends_on: story:connections-reconnect-verb
- depends_on: story:connector-instance-derived-fields
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: apps/connectors-cli-contract
- confidence: inferred
  path: apps/connectors/spec/cli.yaml
- confidence: inferred
  path: apps/connectors/src/local.rs
- confidence: inferred
  path: apps/connectors/tests
- confidence: inferred
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-host/src/local/config.rs
- confidence: inferred
  path: ess/domains/cli.yaml
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T15:49:27Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-10-16T15:49:28Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"review_outcome":3}}}
---
## Context

Step 4 of `migration-plan:connector-vocabulary`, and the answer to open question 2 of
`docs/design/connector-vocabulary.md` that the plan already took: a command rewrites the file.
Existing installations hold `[adapters.<alias>]` entries with live connections, approval policies
and custody items keyed by the alias; none of that may need a new consent, revalidation or policy
publication (plan § Goal).

## Change

- `connectors setup migrate --dry-run` prints the new file and, per entry, before and after: the
  `instance_id`, the `configuration_revision`, the executable selection digest (the approval
  policy's `executable_selection`) and the custody item coordinate.
- `--apply` writes the file owner-only and keeps the old one as
  `config.toml.before-migrate-<timestamp>`.
- Which connector an entry belongs to: for a catalog entry, the connector whose declaration
  (story:catalog-connector-declarations) ships the native configuration's `provider` (for example
  `google-gmail` -> `gmail`). For any other implementation, a connector only where
  story:ess-connector-vocabulary-model maps that `AdapterSpecification` to one; today it leaves this
  UNMAPPED, so credential-binary, MCP, SQL and Kubernetes entries stay `[adapters.<alias>]` and the
  dry run lists them as not migrated. Slack catalog entries stay as well: Slack ships no connector
  declaration (story:catalog-connector-declarations § Change).
- Each migrated entry becomes `[connectors.<c>.instances.<alias>]` carrying its pins
  (story:connector-config-dual-read), so the instance name is the old alias and the alias pin keeps
  mounted-secret item names. All pins are kept; dropping those that derivation
  (story:connector-instance-derived-fields) reproduces byte for byte is a later refinement.
- An entry whose revision or selection would change is not migrated; the command names the field
  that differs.
- Read and write instance pairs are not merged (plan step 8): merging changes scopes and therefore
  the revision.
- `setup migrate` enters `ess/domains/cli.yaml` and `apps/connectors/spec/cli.yaml` as a new
  command; regenerate `apps/connectors-cli-contract`.

## Domain relations

- Connection -> instance (ServiceConfiguration), many-to-one: `ess/domains/auth_bindings.yaml`,
  entity `connectors.auth_bindings.Connection`, relation `instance`; the connection keeps its
  `instance_id` and `configuration_revision`.
- Approval policy -> instance by `instance_id`, `configuration_revision`, `executable_selection`:
  inferred from `crates/connectors-host/src/local/owner/approval_issuance.rs:80-83`.
- Mounted-secret custody item -> alias: inferred from `crates/connectors-custody/src/mounted.rs:164-185`.
- Implementation -> Connector: from story:ess-connector-vocabulary-model
  (`connectors.declarations.Connector`) and, for catalog connectors, the names shipped by
  story:catalog-connector-declarations; cited from the model once it validates.

## Scope

- `apps/connectors/src/local.rs` or a new `apps/connectors/src/local/migrate.rs`
- `crates/connectors-host/src/local/config.rs` (serialising the new form)
- `ess/domains/cli.yaml`, `apps/connectors/spec/cli.yaml`, `apps/connectors-cli-contract/` (generated)
- `apps/connectors/tests/` (fixture configuration with every adapter kind)
- `contracts/cli/v1alpha1/semantics.md`

## Acceptance

On a fixture configuration holding catalog, credential-binary, MCP, SQL and Kubernetes adapter
entries with connections and an approval policy, after `connectors setup migrate --apply` the file
holds each catalog entry (other than Slack, which has no connector declaration) as
`[connectors.<c>.instances.<alias>]` and each other entry still as `[adapters.<alias>]`, while
`connections list` shows the same connection ids and states and `approvals policy-status` the same
policy revision for every entry as before, with no credential entered.

## Notes

- Not checked by the plan and to be checked here: Secret Service attributes of custody entries,
  mutation ledger and execution audit keys, and how `slack-credentials` and `tavily` derive their
  revision (plan § What persists, "Not checked").
- Rollback is restoring the backup and running the previous release (plan § Rollback).
- Ordering on shared files: after story:connections-reconnect-verb (CLI files, generated contract,
  `contracts/cli/v1alpha1/semantics.md`) and after story:connector-instance-derived-fields
  (`crates/connectors-host/src/local/config.rs`).
