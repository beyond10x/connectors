---
format: aep.planning-md/3
id: release-plan:connectors-v0330-store-cost-memory
kind: release-plan
status: active
title: 'Release 0.33.0: store cost flat in its size, owner memory bounded'
relations:
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-10-08T06:30:15Z", actor: "human:timo", revision: 2}
---
## Outcome and authorization

Minor release 0.33.0 from origin/main `fbeda108ce` (the merge of PR 131), under the operator's
rule "ready means ship" (2026-09-25) and the delegated release decision for this repository.
The tag namespace ends at v0.32.0, an ancestor of that base; recheck both before tagging.

Minor, not patch: a store that `setup init` creates carries Entity Runtime durable open
checkpoints, which 0.32.0 and earlier refuse to open (breaking for a mixed install), and
`setup checkpoints-enable` is a new command. An existing store keeps working with older
releases until its owner runs `setup checkpoints-enable --confirm one-way`; the CHANGELOG states
the migration.

## Released scope (v0.32.0..fbeda108ce)

| PR | Change |
|---|---|
| #131 | Wave 20261007c: per-invoke metadata cost flat in the store size (fixes #101): commands read through a running owner, durable open checkpoints for new stores, `setup checkpoints-enable` for existing ones, due expiries in batches of at most 32; owner memory measured within its target (fixes #103); ESS 0.56.0, Entity Runtime 0.30.1, Eventlog 0.8.1. |

Evidence: PR 131 repository gate https://github.com/beyond10x/connectors/actions/runs/37734860304
(success, head 907668c532); measurements in `specification:wave-20261007c-store-cost-memory`
§ Results (median read invoke 229 ms at 601 events, 255 ms at 6,000; peak RSS 329 MB at 601).

## Not in this release

- The expiry batch bound of 32 stays until Entity Runtime batch cost scales linearly
  (`story:lift-expiry-batch-bound`, blocked by `upstream-blocker:er-batch-cost-superlinear`).
- Peak memory was measured in the store-cost test process, not in a long-running owner.
- The read-only CLI audit findings of 2026-10-07 (completion, help routing, admission
  timeouts, lifecycle conflicts, revalidation `outcome_unknown`) are the next wave.
