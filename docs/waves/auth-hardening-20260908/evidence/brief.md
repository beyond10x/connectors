# Unit brief: contracts-credential-evidence

Story: story:contracts-credential-evidence
Branch: impl/contracts-credential-evidence
Base: 1e567571d9ac62070933c7099b93a4e030613e58 from wave/auth-hardening-20260908
Acceptance: After revision, every credential-replacement scenario uses evidence valid for the dispatched credential identity.
Worktree: /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908
Build: /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/target
Scratch: /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence
ESS: /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/toolchains/ess/0.20.0/bin/ess

## Owned files
- contracts/auth/evidence/v1alpha1/semantics.md
- contracts/auth/connection/v1alpha1/semantics.md
- contracts/auth/capability/v1alpha1/semantics.md
- docs/adapters/kubernetes.md
- ess/domains/credential_evidence.yaml
- contracts/auth/evidence/v1alpha1/verification.md
- contracts/auth/evidence/v1alpha1/scenarios/

## Repository invariants and gate

Read the installed `/home/timo/.codex/plugins/cache/beyond10x/aep-drive/0.8.1/agents/implementor.md` charter and the worktree and ESS skills. The harness is a generic agent obeying this role, not a native subagent_type. Work only in the assigned tree; acquire/renew/release your own `codex-auth-<unit>-impl-20260908` worktree session lease. Coordinator also holds a lease; never release it.

Read root AGENTS.md, design §12 and §18, whole story and source finding before authoring. No planning store writes or AEP mutations; no git add/commit/stash/branch/worktree commands, no integration clients or runtime host implementation. All executable project tooling must be Rust; no bespoke scenario interpreter. Coordinator owns shared credentials.yaml, system.yaml, gate.rs, all planning and integration. Request any common correction via a patch in scratch and report; do not edit those files.

Use own target/ only; no shared CARGO_TARGET_DIR. Set TMPDIR to assigned scratch and CARGO_BUILD_JOBS=2. Compiler cache already configured in ~/.cargo/config.toml. No /tmp, outside artifacts or cleanup. Leave wanted changes unstaged and release only your lease on handback. Do not modify versioning/migration proposals or generated files.

Test-first semantic adaptation: write at least an acceptance-bearing authored ESS scenario before its declarations, run pinned ESS author and preserve missing-declaration refusal. Then model and prose, validate incrementally and compile scenarios. This measures compiler structure, NOT runtime behavior. The fixed header must honestly say runtime executed0→0 and needs-coordinator yes; report separate compiled counts and refusal exits, explicit UNMAPPED/runtime obligations. Do not invent persistent evidence entities merely to get a lifecycle: use value types or a real trusted decision/admission record with its own meaning. Shared immutable generation records use expected identity, never claim verified identity from capture.

Unit gate: `$ESS specify validate --path ess`; `$ESS specify compile --path ess --out $SCRATCH/ir.json`; `$ESS verify conform author --path ess --scenarios <private-directory> --out $SCRATCH/authored.json`; `$ESS verify conform synthesize --path ess --target ir --scenarios <private-directory> --out $SCRATCH/synthesized.json`; additionally compile existing operations scenarios as regression and `git diff --check`. Use local pinned .local/toolchains/ess/0.20.0/bin/ess. No Rust changed in your unit, so no Rust builds/full gate. Paste exact outputs and exit statuses in verification.md and report, separating generated and authored counts. No runtime conformance target exists for Connectors.

First six report lines exactly: unit: <story>; verdict: green|red|blocked; cases: executed 0→0, red 0; origin: n/a; wrote-outside-worktree: none|paths; needs-coordinator: yes. Then evidence, scope checked/inferred errors, owned diff hunk headers, semantic decision matrix, remaining unmodeled guarantees. Write exact final report in assigned scratch/implementor-report.md as well as returning it. Adversary later reads your worktree and may add authored cases. Do not spawn agents yourself.

## Decisions this unit settles

F05: exact captured generation binds evidence and admission/dispatch; same filename/ref/revision does not imply same bytes. Pin validated immutable material through dispatch or explicitly revalidate using admitted auth-management flow; detect and refuse account reassignment unless separately authorized. Account identity is kind+stable subject within provider authority and binding, not a display name. Accommodate configured bearer/cert/exec snapshots with same rule. Define retained/invalidated checks for same-account refresh versus replacement/reassignment; evidence is value embedded in Connection, do not invent persistence ownership. Known revocation/expiry must invalidate even pinned leases; pinning does not grant authority. Ordinary invocation currently forbids identity probe calls: reconcile replacement validation via declared admitted validation step or refuse, no hidden provider call expansion. Fix public sample ambiguous secret version field so private generation/secret refs never leak. F04 publishes new validated generation under coordinator fence; consume shared generation type and request common changes if needed. Required scenarios named configured-replacement-identity.yaml, rotation-before-dispatch.yaml, same-identity-refresh.yaml; additional precise cases in same directory allowed, report to coordinator for typed scope update.
