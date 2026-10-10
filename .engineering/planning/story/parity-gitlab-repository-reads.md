---
format: aep.planning-md/3
id: story:parity-gitlab-repository-reads
kind: story
status: implemented
title: GitLab code search and repository tree
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-09T22:50:37Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-09T22:50:38Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-10T09:10:39Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---

## Wave 20261010a (2026-10-10)

- Delivered: `repository.tree`, `commit.get`, `commit.diff` and `branches.list`; `gitlab.repository.tree` and the commit, diff and branch names covered.
- Not delivered: project code search. Its pinned path carries GitLab's `(-/)` optional segment and `scope` must be held to `blobs`. Waits for `story:catalog-path-correction-and-value-bound`; `gitlab.search.blobs` stays missing.
- The story stays active until search is selected.
