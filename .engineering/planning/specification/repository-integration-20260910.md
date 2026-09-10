---
format: aep.planning-md/1
id: specification:repository-integration-20260910
kind: specification
status: draft
title: Integrate all retained Connectors v2 work into main and retire extra recovery storage
relations:
- informed_by: specification:contract-driven-connectors-design
- informed_by: story:local-cli-ess-surface
revision: 1
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
