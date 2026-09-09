---
format: aep.planning-md/1
id: epic:local-cli-contracts
kind: epic
status: active
title: Specify local CLI contracts and the ESS presentation surface
relations:
- informed_by: specification:local-cli-wave-20260909
revision: 5
---
## Outcome
Specify and prove the local CLI presentation, configuration and lifecycle contracts under specification:local-cli-wave-20260909. The user approved OS keyring, protected terminal/file/stdin input, task-grouped commands, per-adapter TOML startup=on-demand|automatic, on-demand default, automatic at local-host startup, no MCP in this stage.
## Acceptance
The selected local CLI has reviewed textual semantics, existing-entity-aligned ESS declarations, complete upstream-supported CLI binding coverage, deterministic generated parser/reference artifacts and passing offline conformance with explicit runtime obligations.
## Boundaries
No production auth/process/provider handlers or external release. Existing identities in ess/domains/auth_bindings.yaml and ess/domains/declarations.yaml remain the owners. CLI configuration and callable metadata are values, not fictitious persistent entities.

## Current implementation checkpoint

The implementation remains scoped to reviewed local CLI contracts and an executable generated contract fixture. Production credential storage, adapter execution and MCP remain outside this epic. The two upstream CLI crates pass 159 parent tests including generated standalone-package verification, all-targets strict Clippy and repository formatting. The final adversary's exact-integer negative-zero defect is fixed while its tests remain unchanged.

Current ESS main integration required authored-input-through-terminal consumer profiles. Finite behavioral witnesses account for 5,433 of 5,439 new pairs. An independently approved bounded policy decision now accounts separately for exactly six schema-document metadata relationships; these are not CLI behavior. Its fresh guard, format migration and final qualification are still implementation work. The original inventory,87 pre-existing profiles and frozen unknown baseline remain preserved. Full ESS gates, exact final source pin/build, downstream regeneration, conformance, local commits and managed cleanup remain required.

Read-only downstream preflight found all15 command/callable/argument mappings aligned with current ESS and64 unique fixture IDs. It found one misleading usage-error example that reused custody_unavailable; the example now uses invalid_configuration and checks complete returned error data. Compilation remains pending generated-package installation. The managed Atlas authority checkout is clean at verified remote main3b2e8455ce7a840b23db32e2f3de73f533112f8c; ADR0047 concerns AEP runtime extraction and does not expand this scope. No Atlas enrollment, external publication or production-handler implementation is authorized by this checkpoint.
