---
format: aep.planning-md/1
id: specification:local-cli-wave-20260909
kind: specification
status: draft
title: Local CLI contract and ESS binding wave
relations:
- informed_by: specification:recent-agent-adapter-usage-20260909
- informed_by: specification:core-model-closure-20260909
revision: 2
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
