---
format: aep.planning-md/1
id: specification:spec-completion-parallel-20260909
kind: specification
status: draft
title: Parallel completion of remaining semantic specifications
relations:
- informed_by: specification:contract-driven-connectors-design
revision: 1
---
# Specification completion parallel plan, 2026-09-09

The operator explicitly requested faster work with sub-agents after authorizing the full specification-completion goal, no token budget, local commits and managed worktrees for parallel edits. This is the approved dispatch page, not a new approval request. Runtime implementation and external publication remain excluded.

Common local checkpoint: 8e1836cad8ae1b2127ce9ae306c6d8131960db4c. Primary main is clean. Coordinator branch: integrate/spec-completion-20260909. Three workstreams are assigned by exact files; stories sharing files execute sequentially inside their stream. This is an explicit grouping adaptation to the wave's one-story unit pattern. The harness offers generic agents; role charters and disk briefs govern this interactive dispatch, not an aep drive engine run.

## Workstreams and triples

| Owner | Work | Managed id / branch | Tree | Build | Scratch |
|---|---|---|---|---|---|
| datasources | story:contracts-log-continuation then story:contracts-document-admission | specs-datasources-20260909 / specs/datasources-20260909 | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/specs-datasources-20260909 | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/specs-datasources-20260909/target | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/specs-datasources-20260909/.local/spec-completion-20260909 |
| read-retry | story:contracts-read-refresh-retry | specs-read-retry-20260909 / specs/read-retry-20260909 | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/specs-read-retry-20260909 | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/specs-read-retry-20260909/target | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/specs-read-retry-20260909/.local/spec-completion-20260909 |
| contract-cleanup | story:contracts-media-controls then story:contracts-supported-vocabulary; prepare story:contracts-documentation-index patch after media | specs-contract-cleanup-20260909 / specs/contract-cleanup-20260909 | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/specs-contract-cleanup-20260909 | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/specs-contract-cleanup-20260909/target | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/specs-contract-cleanup-20260909/.local/spec-completion-20260909 |
| coordinator | evidence precision; ownership closure; shared integration and planning | specs-integration-20260909 / integrate/spec-completion-20260909 | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/specs-integration-20260909 | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/specs-integration-20260909/target | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/specs-integration-20260909/.local/spec-completion-20260909 |

Each unit has its immutable original brief.md in the assigned scratch. Workers acquire, renew and release their own managed leases. Coordinator session is specs-coordinator-20260909. No shared CARGO_TARGET_DIR and no writes in primary.

## Collision resolution

The captured aep graph and computed-waves.json expose collisions across auth capability, media, indexes and historical native paths. They are not asserted away: retry owns capability; cleanup returns capability/index patches; datasource owns native log/document paths and new native evidence; coordinator owns shared ESS, service compatibility/v1alpha2, root indexes, docs/design and planning. Evidence-precision audits worker-owned documents read-only and applies needed edits only after handoff/integration. Documentation-index depends on completed media and is applied after that semantic work. Native datasource stories are serialized under one owner. Five older draft-story boundaries/scopes will be reconciled through AEP CLI before their lifecycle claims.

## Execution and checks

The principal datasource corrections already exist in the common checkpoint. Workers complete remaining concrete findings, bounded textual scenarios and appropriate pinned ESS 0.20.0 specify/compile checks. Schema checks do not execute provider behavior. Runtime test-first charter instructions are adapted to the explicitly specification-only user scope; no invented runtime tests or helper programs.

Coordinator commits locally through the verified Atlas bot wrapper, verifies exact author/committer, dry-runs merges, integrates green scoped changes and shared patches, then runs one full repository gate with MSRV. Independent semantic review covers immutable combined source snapshot; two independent approvals, reviewed finding dispositions and passing validation are the coding gate. Review correction rounds are bounded; no unrelated feature work expands this wave.

## Recovery and finish

No remote deployment/publication is authorized. Existing retained design/Kubernetes trees are untouched. The previous local-recovery bare repository is available but recovery of each new commit must be verified before managed finish/GC; no local commit is assumed published. Preserve evidence before cleanup, review worktree gc --dry-run and apply only exact reviewed ids if eligible. Otherwise hand off the retained path, branch, commit and reason explicitly.

## Current stage

Dispatched from common checkpoint; seven original remediation stories remain open at dispatch, plus ownership closure and consolidated final consistency review. Root remains sole planning writer. Individual worker completion is not final specification readiness.

