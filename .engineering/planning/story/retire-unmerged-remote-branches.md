---
format: aep.planning-md/3
id: story:retire-unmerged-remote-branches
kind: story
status: implemented
title: Decide and retire 32 unmerged remote branches
relations:
- decomposes: epic:tech-debt-review-20260930
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: refs/heads/* on beyond10x/connectors
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-07T00:09:58Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-07T00:09:59Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-07T00:09:59Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Defect

32 remote branches on `beyond10x/connectors` are not ancestors of `origin/main` (2026-09-30),
committed 2026-09-01..23: v1-lineage work (`feat/repository-workspaces`, `release/0.7.0`,
`release/bounded-source-reads-0.7.1`, `wave/cli-*`, `compat/claude-oauth-*`, …), `archive/wt-*`
(6), `next` (stopped at the v0.11.0 plan close per `AGENTS.md`), `pre-contracts` and
`integrate/ess-evolution-phase-e-20260923`. Merged head branches are deleted by GitHub on merge, so
these are the leftovers.

## Change

For each branch: keep, archive (a bundle or tag outside the branch namespace), or delete. Deleting a
remote branch is irreversible for anything not reachable elsewhere; `decision-blocker:v1-remote-branch-retirement`
holds that choice.

## Scope

- remote refs of `beyond10x/connectors` only; no file in the tree — cited.

## Acceptance

- Every one of the 32 branches is listed with its decision and, if deleted, the commit it pointed at
  and where that commit is still reachable or archived.
- `git ls-remote origin 'refs/heads/*'` shows only `main` and the branches marked keep.
- Every deletion goes through the bot App.

## Outcome

Done 2026-10-07; the per-branch list and the decision are in `decision-blocker:v1-remote-branch-retirement`. 15 deleted through the bot App, 17 kept (the bot route refuses archive tags on history older than the Gates baseline), 0 tagged. `git ls-remote origin 'refs/heads/*'` answers 23 heads: `main`, the 17 kept and five branches of current work created after this story was written.
