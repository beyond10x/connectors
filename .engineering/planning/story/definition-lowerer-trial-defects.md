---
format: aep.planning-md/3
id: story:definition-lowerer-trial-defects
kind: story
status: implemented
title: A new definition lowers to files that agree, and states its evidence lifetime
summary: 'Two defects a hand-written trial definition found in the lowerer: a declaration naming a selection file not written beside it, and no field for evidence_lifetime_ms'
relations:
- implements: architecture-decision-record:provider-definitions-and-runtimes
- depends_on: story:definition-lowerer-declarations-onto-vocabulary
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-18T01:28:43Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1}}, executor: "agent:connectors-fix-20261018"}
- {from: "proposed", to: "active", at: "2026-10-18T01:28:43Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:connectors-fix-20261018"}
- {from: "active", to: "implemented", at: "2026-10-18T01:28:44Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}, executor: "agent:connectors-fix-20261018"}
---
## Outcome

A definition none of the shipped selections came from lowers to a set of files that agree with
each other, and a definition can state the evidence lifetime of an auth profile instead of an
operator adding it to a generated configuration by hand. Found by a trial on 2026-10-18 that wrote
a GitLab definition by hand (`gitlab-trial`, `catalog: gitlab`, three reads), lowered it with
#179's lowerer and ran it against a self-hosted GitLab.

## Defects

1. The lowered `connector.json` named `"selection": "trial-operations.json"`, but the lowerer wrote
   `gitlab-trial.operations.json` beside `gitlab-trial.connector.json`
   (`crates/connectors-catalog/src/definition.rs`, the declaration's `selection` against the file
   `lower` wrote).
2. `connectors-definition/1` had no field for `evidence_lifetime_ms`; the trial added 300000 to its
   generated provider configuration by hand.

## Findings

- `evidence_lifetime_ms` belongs to the auth profile declaration, not to the operator's instance
  configuration. The catalog provider's `Declaration.profiles` are its `AuthConfig`, which carries
  `evidence_lifetime_ms` with a default of 60000 (`adapters/catalog/src/local.rs`, `AuthConfig` and
  `Declaration`); an instance of a shipped connector names only connector, name, endpoint and
  profile (`Instance`) and takes its `auth` from the declaration (`derived`). The host treats the
  lifetime as part of the profile declaration's revision (`docs/local-connection-registry.md`,
  `ess/domains/auth_bindings.yaml` `LocalStaticProfile`). The docs already say so; none changed.

## Change

- The lowerer writes the bundle and index to `--out`, and the declaration and the selection it
  names to `--out/<catalog>/`, the layout of `adapters/catalog/providers/<catalog>/`. Shipped
  declarations are unchanged byte for byte.
- `Auth.evidence_lifetime_ms` (optional): in the definitions repository's `spec/` and `generated/`
  (connectors-definitions 79f27e5 on `adopt/declaration-ids`), and in the lowerer, which writes it
  after `minimum_scopes`, merges it from a shared auth file and refuses a value outside 1 to
  300000 as `auth_lifetime`. The importer carries it back.

## Acceptance

1. `crates/connectors-catalog/tests/definition_layout.rs`: a plain and a `catalog:` definition over
   a test-only API lower to a declaration whose `selection` is the file beside it; the output holds
   exactly the bundle, the index, the declaration and that selection.
2. `crates/connectors-catalog/tests/definition_auth.rs` and
   `adapters/catalog/tests/definition_auth.rs`: a stated lifetime reaches the declaration, a shared
   one is inherited and replaceable, 0 and 300001 are refused, and the provider's bootstrap
   announces 300000 for a lowered GitLab profile stating it.
3. `definition_import`: every shipped declaration still lowers byte-identical; the fixtures equal
   connectors-definitions `adopt/declaration-ids` (33 of 33 files).
4. connectors-definitions `task check` valid.
