---
format: aep.planning-md/3
id: approval-record:gates-baseline-20261023
kind: approval-record
status: draft
title: Gates baseline of beyond10x/connectors moved to d6f9d3ea6
summary: The directing session approved moving the connectors Gates baseline to d6f9d3ea6 after the scan limit refused the F1 push; gates-policy 37dbad0.
revision: 1
---
## Decision

On 2026-10-23 the directing session `b10x-chef-orchestrator` approved, under its class C grant (Gates baseline move), moving the Gates adoption baseline of `beyond10x/connectors` from `2da5f72c9` (2026-10-05, the release 0.27.0 close) to `d6f9d3ea6`.

## Why

- `b10x-gates` 0.1.23 refused the bot push of `unit/catalog-retry-after` (`d93457b06`) with "candidate exceeds scan limit": the candidate is every commit from the baseline to the head, 1005 commits, above the 512 MiB total of `gates/src/git.rs:12`.
- `d6f9d3ea6` is the newest `main` commit whose full Shared source gates scan passed (run of 2026-10-23T11:02:24Z). The merge after it, `abc7aadda`, failed that scan with the same refusal (run 39949606223), so the baseline stops before it and trusts nothing unscanned.

## Delivery

- gates-policy commit `37dbad0e9bce8450ff6160d68706f90752af121e` on `main`, author and committer `b10x-bot[bot]`.
- The directing session refreshes the `B10X_GATES_POLICY` secret copies (organization and `beyond10x/ess`).
