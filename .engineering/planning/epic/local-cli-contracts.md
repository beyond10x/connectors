---
format: aep.planning-md/1
id: epic:local-cli-contracts
kind: epic
status: implemented
title: Specify local CLI contracts and the ESS presentation surface
relations:
- informed_by: specification:local-cli-wave-20260909
revision: 8
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

## Completed wave validation

The selected CLI contract fixture and both wave stories are implemented. ESS
implementation source 6f7ef46163e758f3401945d1a946e0fc80ebc003 passes its full
required task check, including all 152 freshly qualified consumer cases and the
six-row metadata guard. The separately required ESS website check also passed.
Historical partial attempts remain evidence of those attempts only.

Connectors implementation tree b6573f9a3c7936f7f7b3dde93e600bc27d17d03a selects
that exact clean source through a verified local build receipt. The final
connectors-full-gate-04.log records exit zero with Rust 1.88 compatibility,
all workspace tests and strict Clippy, shared plus five independent native ESS
roots, deterministic generation, adapter boundaries, the 15-command fixture,
64 structural CLI fixtures and 315 scenarios including 34 authored traces.
GitLab regeneration changes only ESS source provenance; generated API/schema
bytes remain unchanged.

website-build-final-03.log and website-reference-check-final-04.log pass on
the same pin. Earlier typecheck, 15 Rust example tests, browser walkthroughs and
UI checks passed; the final upstream follow-up changes only a test expectation
and planning evidence. No presentation or browser-example implementation changed
after those checks.

The 152 ESS cases comprise 130 new fine-grained CLI consumer checks, 20 existing
change-detection checks and two existing relation projections. They are separate
from Connectors' 315 scenarios. The accepted ESS unknown baseline remains
157,677 unproven cells; metadata bookkeeping is not runtime behavioral coverage.

Durable evidence is in the primary checkout under
.local/cli-contract-wave-20260909/integration-evidence/: connectors contains the
gate, build and website logs; ess contains the upstream gate, qualified summary
and qualified-cases.md. The ESS primary checkout retains the complete compressed
qualification and earlier checkpoints under its corresponding local evidence
directory. Final integration heads and managed retirement are recorded in the
local integration receipt.

This completes the approved contracts/ESS fixture milestone. Production OS keyring
custody, metadata persistence, adapter supervision and provider handlers remain
future implementation; MCP, cloud/federation expansion and external publication
remain deferred. Local integration and retirement follow the completed checks.
