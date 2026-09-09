---
format: aep.planning-md/1
id: epic:local-cli-contracts
kind: epic
status: active
title: Specify local CLI contracts and the ESS presentation surface
relations:
- informed_by: specification:local-cli-wave-20260909
revision: 6
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

## Integrated downstream validation

Connectors implementation commit 93fdc44ad6b2984b924cdbc0e2b66517a5d2363c
is preserved on local-recovery candidate/local-cli-contracts-20260909. The selected
ESS pin is now 4079db6b736864cf23dbd3cc791fb10355e5296f, adding only a browser
fixture correction to the already validated generator source. Its independent
clean-source build and executable receipt pass (ess-source-build-05.log).

The final Connectors gate --msrv passes on that pin (connectors-full-gate-03.log):
88 workspace tests, strict Clippy, Rust 1.88 all-target checks, 15-command CLI
inventory/conformance, 64 structural CLI fixtures plus cache/acquisition/freshness
assertions, all shared and five native ESS models, adapter/library boundaries,
deterministic GitLab generation and 315 scenarios including 34 authored traces.
The generated API bytes did not change; the bundle manifest records exact source
provenance. Website production build, reference drift, 15 Rust example tests,
typecheck, browser walkthroughs and UI checks also passed. These demonstrate the
selected contract fixture; production keyring, process and provider handlers remain
separate implementation obligations.

The semantics story is implemented. This surface story and epic remain active until
the required upstream ESS full gate and consumer qualification finish, after which
the integration branches can land locally. No external publication is authorized.

Worker trees cli-semantics-20260909 and cli-toolchain-20260909 are finished and
removed through reviewed exact-id managed GC. Commits 30426f6 and dfc63c3 remain
under local-recovery unit/cli-semantics-20260909 and unit/cli-toolchain-20260909.
Their retained evidence is in the primary checkout's ignored
.local/cli-contract-wave-20260909/worker-evidence directory. The coordinator owns
the remaining Connectors integration, ESS integration and Atlas authority trees.
