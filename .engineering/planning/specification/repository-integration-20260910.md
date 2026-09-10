---
format: aep.planning-md/1
id: specification:repository-integration-20260910
kind: specification
status: draft
title: Integrate all retained Connectors v2 work into main and retire extra recovery storage
relations:
- informed_by: specification:contract-driven-connectors-design
- informed_by: story:local-cli-ess-surface
revision: 3
---
## Outcome and authority

The operator explicitly requested all retained Connectors v2 work integrated sanely into main, no stale Connectors v2 worktrees, and removal of the additional bare recovery repository. This is an interactive integration and cleanup. It authorizes local commits, history integration and reviewed retirement; it does not launch the Kubernetes implementation, expand runtime support, publish externally or change another product repository.

## Inspected inputs

Primary main starts at e4b4b9817ee65cd859baef960299d56cce7e2063. All retained branch and tag tips are already ancestors except dfc63c334753edae6cbce36254779f040f017b05 and cf24afc86195c834634ae5d630a3e9517696c6eb. The branch/ref inventory and manager records are retained under .local/integration-20260910 in the primary checkout.

The dfc63c3 toolchain checkpoint explicitly states that 93fdc44 contains its final integrated implementation. Comparison of its five changed files confirms unchanged adversarial tests and documentation; current main adds canonical snapshot-path handling, excludes temporary ESS anchor metadata from generated bundles, and selects the later verified ESS source 6f7ef46163e758f3401945d1a946e0fc80ebc003. Preserve those current bytes while integrating the historical checkpoint.

The cf24afc Kubernetes checkpoint contains only a draft story, AEP journal entries and a draft driver task. Recreate its current backlog record through AEP and reconcile the task against current main before recording the old checkpoint as integrated history. Never concatenate or repair the two planning journals manually. Preserve the old draft and journal in the checkpoint's Git history.

The two older directories connectors_v2-model-closure-20260909 and connectors_v2-spec-completion-20260909 under the manager's recovery directory contain retained verification files, not Git repositories or worktrees. Preserve them. The only additional bare Connectors repository found is connectors_v2.git. Its refs and objects must be accounted for before its exact path is removed. Historical stash objects are already present in the primary object database and are not new implementation candidates.

## Integration and verification

Use the managed integration checkout and a clean Atlas checkout verified at remote main solely for the required bot wrapper. All store changes use aep plan artifact. Keep the Kubernetes story draft, with current typed owners and current toolchain resolution; no evidence is manufactured to advance it.

Verify that all original branch/tag tips and removed worktree heads are reachable from final main, the changed files are planning/task documentation only, and the runtime, contracts, ESS models, generated artifacts, toolchain pin and website implementation match e4b4b98 exactly. Run AEP validation and whitespace/link checks. Reuse the existing successful local gate and website evidence for the unchanged implementation; do not rerun expensive gates remotely or dispatch CI.

Publish the integration to the existing temporary recovery destination before managed finish, land it on primary main, review exact-id GC and remove only the task's trees. Then remove redundant merged branches, the local-recovery remote and its exact bare directory after checking all wanted history is reachable locally. Preserve audit/verification receipts under primary .local/integration-20260910. The absence of a remaining remote is intentional under the operator's request.

## Scope

This is one integration outcome, not a decomposition. No critic panel is needed. Source changes are limited to this planning record, the integrated Kubernetes draft, its task YAML and planning journal. Git refs and managed lifecycle state are operational changes. No new typed entity or relation is introduced.

## Verified integration before retirement

Commit 7e6551a recreates the Kubernetes draft through AEP and reconciles its driver task with current main. Merge 2890f0f records the earlier toolchain unit while retaining the three verified current file versions; its resulting tree equals its first parent. Merge 2b9e627 records the original Kubernetes checkpoint after the AEP reconciliation; it keeps the current store rather than merging historical journal text. Both original checkpoints are now ancestors, with their original content and journal preserved in Git history.

Every original primary branch/tag tip, every recovery branch/tag tip and every historical managed-worktree head passes git merge-base --is-ancestor against the integration head. All 2,918 objects in the bare recovery repository are already present in the primary object database, including historical unreachable stash objects. The older amended review checkpoint 9f2c7f4 differs from the integrated 492c20c only in retained log/manifest evidence, not runtime code. No unique implementation is stranded in the extra repository.

The complete diff against e4b4b98 contains only the planning journal, this specification, the Kubernetes story and its task YAML. git diff --quiet e4b4b98 HEAD with those planning/task paths excluded passes, proving the previously checked runtime, contracts, ESS model, generated output, toolchain pin and website bytes remain unchanged. Whitespace and cited-path checks pass. AEP validation reports 150 artifacts, 82 historical review-format warnings, and valid; full output is retained in primary .local/integration-20260910/aep-validate.log.

The remaining operational sequence is local publication, fast-forwarding primary main, managed finish and exact-id GC, pruning only merged branch refs, and deleting the explicitly retired local-recovery remote and bare directory. The final observed results and exact main commit will be written to primary .local/integration-20260910/README.md after those operations, without fabricating their completion in advance.

## Local runtime integration checkpoint — 2026-09-10

The operator requested integration and worktree cleanup before continuing the local runtime initiative. This interactive checkpoint selects this existing integration record; it introduces no decomposition, runtime entity or implementation change. No critic panel is required for this operational checkpoint, and no approval bypass is recorded.

The requested implementation was already integrated directly into local main as 2c41fc49e00549ddb671a68845ec5fcf6ee4277b, "Implement local CLI setup and metadata foundation". Both author and committer are b10x-bot[bot]. No additional merge is needed: git merge-base --is-ancestor 2c41fc49e00549ddb671a68845ec5fcf6ee4277b main exits 0, git branch --contains prints main, and initial git status --short --branch prints only:

```text
## main
```

git worktree list --porcelain reports only the primary Connectors checkout at that commit. worktree repo list reports the unmanaged primary, and worktree inspect --repo /home/timo/beyond10x/connectors_v2 --json returns an empty inspections array. There is no Connectors task tree to finish, reconcile or remove. No recovery repository or remote is created, and no Connectors publication is performed.

The earlier task's build target, its symlink and /dev/shm/connectors-local-runtime-20260910 are confirmed absent. The review binary and small verification receipts are retained. All 17 implementation entries in docs/evidence/local-runtime-20260910/implementation-sha256.txt pass sha256sum --check, and the retained binary still has the digest recorded in docs/evidence/local-runtime-20260910/README.md. The successful gate, Rust 1.88, generation, conformance and documentation evidence in that report applies to these unchanged inputs; an AEP-only checkpoint does not rerun unrelated builds or provider tests.

The prior Atlas authority tree is clean and has no live lease from the implementation task. Its head no longer equals remote main. A temporary manager-owned authority tree, connectors-integration-authority-20260910, is therefore created at verified remote commit caa019cfbe6c2056cf3978456cd983742cf0e216 for the required local bot commit. No Atlas source is changed. Its own lease is released after committing, then manager finish and reviewed exact-ID GC retire only that temporary tree. Final observed commit, lifecycle results and cleanup checks are retained after execution in .local/tmp/local-runtime-integration-20260910; this paragraph records the required sequence, not advance evidence of removal.

initiative:complete-local-connectors and story:persistent-gitlab-journey remain active. Their acceptance has not been met. credential-blocker:gitlab-runtime-sandbox and tooling-blocker:kubernetes-driver-protocol-loading remain open, with their existing distinct scopes. The MCP epic and Kubernetes story remain draft; completed specification stories keep their statuses. Historical review records are preserved. This checkpoint changes only this specification and the CLI-owned planning journal.

Fresh Git, manager and hash observations are retained in .local/tmp/local-runtime-integration-20260910. The post-edit aep plan artifact validate output is retained verbatim there as aep-validate.log and relayed to the operator. The final planning inventory is compared with planning-before.json to verify that no artifact status, relation or scope changed.
