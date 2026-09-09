---
format: aep.planning-md/1
id: epic:local-cli-contracts
kind: epic
status: draft
title: Specify local CLI contracts and the ESS presentation surface
relations:
- informed_by: specification:local-cli-wave-20260909
revision: 1
---
## Outcome
Specify and prove the local CLI presentation, configuration and lifecycle contracts under specification:local-cli-wave-20260909. The user approved OS keyring, protected terminal/file/stdin input, task-grouped commands, per-adapter TOML startup=on-demand|automatic, on-demand default, automatic at local-host startup, no MCP in this stage.
## Acceptance
The selected local CLI has reviewed textual semantics, existing-entity-aligned ESS declarations, complete upstream-supported CLI binding coverage, deterministic generated parser/reference artifacts and passing offline conformance with explicit runtime obligations.
## Boundaries
No production auth/process/provider handlers or external release. Existing identities in ess/domains/auth_bindings.yaml and ess/domains/declarations.yaml remain the owners. CLI configuration and callable metadata are values, not fictitious persistent entities.
