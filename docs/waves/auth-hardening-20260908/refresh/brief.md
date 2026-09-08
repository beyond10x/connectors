# Unit brief: contracts-refresh-coordination

Story: story:contracts-refresh-coordination
Branch: impl/contracts-refresh-coordination
Base: 1e567571d9ac62070933c7099b93a4e030613e58 from wave/auth-hardening-20260908
Acceptance: After revision, the refresh failure matrix permits no second rotating-token exchange while a previous exchange may have consumed the token.
Worktree: /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-refresh-20260908
Build: /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-refresh-20260908/target
Scratch: /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-refresh-20260908/.local/waves/auth-hardening-20260908/refresh
ESS: /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-refresh-20260908/.local/toolchains/ess/0.20.0/bin/ess

## Owned files
- contracts/auth/acquisition/v1alpha1/semantics.md
- contracts/auth/custody/v1alpha1/semantics.md
- docs/adapters/atlassian.md
- ess/domains/refresh.yaml
- contracts/auth/acquisition/v1alpha1/scenarios/
- contracts/auth/acquisition/v1alpha1/verification.md

## Repository invariants and gate

Read the installed `/home/timo/.codex/plugins/cache/beyond10x/aep-drive/0.8.1/agents/implementor.md` charter and the worktree and ESS skills. The harness is a generic agent obeying this role, not a native subagent_type. Work only in the assigned tree; acquire/renew/release your own `codex-auth-<unit>-impl-20260908` worktree session lease. Coordinator also holds a lease; never release it.

Read root AGENTS.md, design §12 and §18, whole story and source finding before authoring. No planning store writes or AEP mutations; no git add/commit/stash/branch/worktree commands, no integration clients or runtime host implementation. All executable project tooling must be Rust; no bespoke scenario interpreter. Coordinator owns shared credentials.yaml, system.yaml, gate.rs, all planning and integration. Request any common correction via a patch in scratch and report; do not edit those files.

Use own target/ only; no shared CARGO_TARGET_DIR. Set TMPDIR to assigned scratch and CARGO_BUILD_JOBS=2. Compiler cache already configured in ~/.cargo/config.toml. No /tmp, outside artifacts or cleanup. Leave wanted changes unstaged and release only your lease on handback. Do not modify versioning/migration proposals or generated files.

Test-first semantic adaptation: write at least an acceptance-bearing authored ESS scenario before its declarations, run pinned ESS author and preserve missing-declaration refusal. Then model and prose, validate incrementally and compile scenarios. This measures compiler structure, NOT runtime behavior. The fixed header must honestly say runtime executed0→0 and needs-coordinator yes; report separate compiled counts and refusal exits, explicit UNMAPPED/runtime obligations. Do not invent persistent evidence entities merely to get a lifecycle: use value types or a real trusted decision/admission record with its own meaning. Shared immutable generation records use expected identity, never claim verified identity from capture.

Unit gate: `$ESS specify validate --path ess`; `$ESS specify compile --path ess --out $SCRATCH/ir.json`; `$ESS verify conform author --path ess --scenarios <private-directory> --out $SCRATCH/authored.json`; `$ESS verify conform synthesize --path ess --target ir --scenarios <private-directory> --out $SCRATCH/synthesized.json`; additionally compile existing operations scenarios as regression and `git diff --check`. Use local pinned .local/toolchains/ess/0.20.0/bin/ess. No Rust changed in your unit, so no Rust builds/full gate. Paste exact outputs and exit statuses in verification.md and report, separating generated and authored counts. No runtime conformance target exists for Connectors.

First six report lines exactly: unit: <story>; verdict: green|red|blocked; cases: executed 0→0, red 0; origin: n/a; wrote-outside-worktree: none|paths; needs-coordinator: yes. Then evidence, scope checked/inferred errors, owned diff hunk headers, semantic decision matrix, remaining unmodeled guarantees. Write exact final report in assigned scratch/implementor-report.md as well as returning it. Adversary later reads your worktree and may add authored cases. Do not spawn agents yourself.

## Decisions this unit settles

F04: coordinator-owned exclusion before any possible exchange; durable attempt authorization; lease expiry alone must never reopen a consumed/possibly consumed rotating token. Separate pre-authorization recovery (fence old owner, then reclaim) from post-authorization uncertainty (known durable response may recover, otherwise repair/reauthorize; no second exchange). Publication must compare expected source generation, ownership/fence and current revocation state atomically. Document backend-independent required atomic operations; secret store only write_new/read/delete and immutable durability. Source generation references exactly one shared connectors.credentials.CredentialGeneration. New generation is validated under F05 before publication; use that common seam without duplicating evidence policy. Scenarios must include two replicas, owner loss both sides of authorization, stale publication after takeover and publication/revocation race. Keep strict conservative behavior if guarantees cannot be supplied. Public errors/wire changes outside this semantic wave.
