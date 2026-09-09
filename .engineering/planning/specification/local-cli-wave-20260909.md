---
format: aep.planning-md/1
id: specification:local-cli-wave-20260909
kind: specification
status: draft
title: Local CLI contract and ESS binding wave
relations:
- informed_by: specification:recent-agent-adapter-usage-20260909
- informed_by: specification:core-model-closure-20260909
revision: 5
---
## Approved scope

The operator approved implementation of the local CLI specification and ESS projection plan, including multiple agents, local commits, managed worktrees and local recovery. This is an interactive run. MCP, cloud identity/federation prerequisites, production credential/launcher/provider handlers, release and external publication are excluded. The earlier no-budget-limit instruction remains in force; the harness has four total concurrent slots.

Startup is per adapter in TOML: startup = "on-demand" (default) or "automatic". Automatic means local-host startup; ordinary read-only listing never starts a host or adapter. An admitted connect/invoke may start the host and its configured automatic adapters plus its selected on-demand adapter. No discovered artifact is installed or executed. Linux is the first selected OS binding, without a portability claim.

The operator selected compact setup/adapters/connections/operations groups, OS keyring, protected terminal and file/stdin entry, durable credential reuse, same-identity repair and terminal revoke. Existing describe/invoke compatibility and legacy serve remain explicit. Secret custody, host management and provider behavior preserve their current owners.

## Evidence and ESS boundary

Sources: docs/design.md sections 12,17,31,32; docs/recent-adapter-usage-20260909.md C01-C05; contracts/auth/management.md; ess/domains/auth_bindings.yaml. Existing AuthProfile, Connection, Acquisition, CustodyVersion and ServiceConfiguration identities are reused. New CLI presentation/configuration metadata and typed request/result values do not invent persistent business entities.

ESS 0.20.0 supports owner-local CLI commands/views but cannot place forwarded commands without changing ownership. Its Clap view projection ignores modeled parameters. Current upstream source 9845922dbf2f8047d533eedf35fbe13d6d234079 preserves that relevant code. Implement an additive versioned CLI presentation binding, with declared local/forward/dynamic callables, typed input/result references, independent path/argument/source/process rules and deterministic Rust/Clap projection. Preserve existing ess/1 and byte-oriented composition. No generic Read entity, workflow DSL or typed composition rewrite.

## Work and coordination

Two repository lanes: ESS compiler/projector in the sibling repository, and Connectors semantics/model/binding/conformance in this repository. Coordinator alone writes either planning store and shared integration files. Unit source scopes must be frozen before implementation; reviewers use tests only or read-only semantic review.

Role dispatch uses aep-drive:story-scoper, aep-drive:implementor and aep-drive:adversary charters through generic harness agents (no native subagent_type selector). Four aep-plan critic perspectives run independently in batches because only three child slots exist. Record every verdict, including an empty findings block.

Approval authorizes the existing analysis checkpoint, opening plan/model commits, unit commits, integration merges, closing store commits, base integration and local recovery publication only. No external push, tag or release.

## Pre-flight

Primary Connectors main was checkpointed at 51c7fe3a6af2f0c0343a3861810b7b20ba19afbc and is clean. AEP protocol 0.54.0 currently reports waves=[], collisions=[], unassessed=[], cycles=[] before these new units exist. Existing store validates with 73 historical review-format warnings.

Free space measured approximately 20 GB; existing Connectors target 7.0G and cached extraction target 261M. sccache is configured by the user. Use two Cargo jobs, no parallel full builds; reserve 8 GiB free and remeasure before each build. Build/check only one repository at a time; new worktree outputs remain isolated.

The retained dirty connectors-v2-kubernetes-drive-20260908 tree belongs to unrelated prior work. ESS has a live ess-review-boundaries-21 coordinator and retained unit trees. The approved plan explicitly preserves them. Our isolated source branch and planning store do not mutate those trees; recheck upstream main and scope before integration. No stale-lease cleanup or guessed retirement is authorized. This is a documented adaptation of the wave's no-prior-tree preflight, not an abandonment claim.

## Managed records

- coordinator id cli-contracts-20260909; repository connectors_v2; branch integrate/cli-contracts-20260909; base 51c7fe3; path /home/timo/.local/state/worktree/trees/b10x/connectors_v2/cli-contracts-20260909; build /home/timo/.local/state/worktree/trees/b10x/connectors_v2/cli-contracts-20260909/target; scratch /home/timo/.local/state/worktree/trees/b10x/connectors_v2/cli-contracts-20260909/.local/tmp/cli-wave; session cli-coordinator-20260909.
- ESS lane id cli-binding-ess-20260909; repository ess; branch work/cli-binding-20260909 (to create); base 9845922d; path /home/timo/.local/state/worktree/trees/b10x/ess/cli-binding-ess-20260909; build /home/timo/.local/state/worktree/trees/b10x/ess/cli-binding-ess-20260909/target; scratch /home/timo/.local/state/worktree/trees/b10x/ess/cli-binding-ess-20260909/.local/tmp/cli-wave; coordinator lease cli-coordinator-20260909.
- authority id cli-authority-20260909; repository atlas; exact clean remote-main authority 62a8d6bbd54dbfc483eff31b83a738a5cb019ea9; no source changes; coordinator lease cli-coordinator-20260909.

## Exit criteria

Reviewed local semantic contract, validated CLI model, upstream supported binding/compiler/projector, generated parser/reference and deterministic drift tests, process conformance with recording handlers, C01-C05 and startup failure traces with honest runtime-obligation accounting. Run package checks and final owning-repository gates; record each real exit and skipped/not-executed coverage separately. Preserve evidence and local recovery before exact-id managed cleanup. No real keyring/provider conformance is claimed by fixtures.

## Current-source steering and review corrections
The operator subsequently instructed us to inspect latest Atlas, use latest ESS main pinned by exact source rather than requiring the 0.20.0 release, and revalidate/regenerate because upstream has changed. Clean Atlas remote main 62a8d6bbd54dbfc483eff31b83a738a5cb019ea9, policies/latest-upgrades.md and AGENTS.md Foundation composed releases select exact source commits without requiring a tag. They explicitly keep connectors_v2 outside automatic enrollment.
Latest ESS remote main observed 19de6406f97dca339136d7c9075ecc9b8fdb7af7, superseding initial local 9845922d. Preserve preparation commit/branch before moving our isolated lane onto current source; rebuild and record exact commit/binary identity. Regenerate and review every affected shared/native/schema/bundle/docs/example output and run appropriate conformance; do not change pinned vendor inputs to conceal incompatibility.
Scope critic r1 findings are assigned explicitly to story:local-cli-binding-semantics: Linux binding/portability boundary and restart credential reuse plus same-identity repair. Parent acceptance carries both and no other story silently owns them.
The ESS active lane branch becomes work/cli-binding-main-20260909 at the same managed path. work/cli-binding-20260909 preserves initial planning preparation only. Current source is an explicit operator correction, not a discarded review. Stage: plan review complete except final corrected-scope recheck; no production code dispatched yet.

## Resumed implementation

Implementation resumed in the same session after an interruption. Both initial implementor processes had stopped before any source edits; replacement agents own the same bounded unit surfaces. All four final plan critics approved in round 2. No further plan approval is pending.

Current ESS source base is 19de6406f97dca339136d7c9075ecc9b8fdb7af7 on work/cli-binding-main-20260909, planning checkpoint a7e2316. The old work/cli-binding-20260909 branch preserves preparation only. Connectors semantic unit is cli-semantics-20260909, branch work/cli-semantics-20260909, base 88c0365. Coordinator branch integrate/cli-contracts-20260909 remains the sole planning writer.

One additional isolated unit implements the already-assigned exact-source toolchain portion of story:local-cli-ess-surface: id cli-toolchain-20260909, branch work/cli-toolchain-20260909, base 88c0365, path /home/timo/.local/state/worktree/trees/b10x/connectors_v2/cli-toolchain-20260909. It owns crates/connectors-spec/toolchain.json, crates/connectors-spec/src/toolchain.rs, the toolchain provenance portion of crates/connectors-spec/src/v2.rs and docs/gitlab-generation.md. Coordinator owns all other integration surfaces; it remains a subtask of the surface story, whose final CLI binding depends on the semantic and upstream units. No dependency is declared complete prematurely.

Coordinator lease cli-coordinator-20260909 covers all five managed trees; each implementor acquires its own lease. Builds are serialized across units and bounded to two Cargo jobs, separate task targets, no incremental/debug data, at least 8 GiB free reserve. Free disk before builds: 23 GB.

## Current-main refresh and integration checkpoint

Remote ESS main advanced again to 113f5925ebb6a0a57a73e661687d9fa5deddb0b8 while this task was active. The current branch is work/cli-binding-latest-20260909 at the existing ESS managed path. Bot checkpoint 76fd70cac3911413b0cd1abe705963deaf67d8a1 preserves the pre-refresh implementation on work/cli-binding-main-20260909. The story was recreated through AEP on the latest base; source was applied separately, preserving journal history. Pre-refresh checks remain historical.

An OS process-resource failure interrupted agents and tool hosts. The operator resumed this session. Replacement turns finished the preserved units, without starting a replacement wave. Available disk recovered to 51 GB at resume and measured 46 GB at this checkpoint; retain the two-job serialized build policy and 8 GiB reserve.

Semantic source commit 30426f6342f4585656509b4e428ecdc2fb696af9 was integrated into the coordinator branch after two attacks. Round 1 found public acquisition-state drift, unlabeled cached connection pages, and cached operation fixtures with stale:false. All were corrected; round 2 executed 79 fixture checks plus five unchanged isolated regressions with zero failures. Both reports are immutable review-result records. These checks used the explicitly provisional older authoring binary; final exact-source regeneration remains pending and the semantic story is not terminal.

The toolchain reader/build unit passed four added adversarial cases with no finding. Its actual clean-source build and final source pin await reviewed ESS integration. The refreshed ESS author reported 27 focused checks and strict Clippy/formatting green. A separate adversary and finite consumer/support gate integration are underway. Full upstream gates, the final source commit/build receipt, Connectors generation/conformance, public reference/example regeneration, final gates, local recovery and exact-id cleanup remain outstanding. No production custody, launch or provider implementation is claimed.