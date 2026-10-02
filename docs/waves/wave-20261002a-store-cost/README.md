# Wave 2026-10-02 (a): metadata store cost, catalog parameter exclusion, setup on missing state

Skill version: aep implementing 0.18.0. Status: approved by the operator 2026-10-02 (plan approved in session).

## Units

| story | serves | scope | agents | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|---|---|
| story:metadata-invoke-cost-flat-in-store-size | vision:independent-contract-adapters | inferred | aep:implementor (aep:diagnosing method), aep:adversary | impl/metadata-invoke-cost-flat-in-store-size | `<managed-trees>`/wave1002a-store | `<tree>`/target | `<wave-scratch>`/store | merged |
| story:catalog-selection-excludes-parameters | vision:independent-contract-adapters | cited (two inferred tests) | aep:implementor, aep:adversary | impl/catalog-selection-excludes-parameters | `<managed-trees>`/wave1002a-params | `<tree>`/target | `<wave-scratch>`/params | merged |
| story:setup-initialises-missing-state | vision:independent-contract-adapters | cited | aep:implementor | impl/setup-initialises-missing-state | `<managed-trees>`/wave1002a-setup | `<tree>`/target | `<wave-scratch>`/setup | merged |

Integration branch: `wave/20261002a-store-cost` off `main` 4d308c09a (v0.23.0).

## Selection

`aep plan artifact waves --kind story --status active --format json` (aep 0.68.0) places all three in wave 1 with no
collision, after one scope correction: setup-initialises-missing-state listed `crates/connectors-host/src/local/metadata.rs`
as inferred, and its scoper wrote "reused unchanged"; the entry was removed and the unit's brief forbids editing the
file. Priority: the metadata store cost blocks every paged connector read (measured 2026-10-01: 1.75 s per invoke at
about 50 events, 13.5 s at 577, 10 events per invoke); a consumer has parked its Jira run until it ships.

Not in this wave: service-failure-carries-upstream-reason (needs a contract decision); the CLI stories that collide on
`apps/connectors/src/local.rs`.

## Commits approval authorises

One commit per unit, the merges into `wave/20261002a-store-cost`, the opening and closing store commits, the push and
pull request through `b10x-gates`, the merge into `main`; then release 0.24.0.

## Close

- Store cost: diagnosed, not fixed here. Release build: 578 ms per invoke at 55 events, 12.4 s at 601, 40.2 s at 1,203;
  `execute_batch` 62-87% of it, host-only work 0.4%. The cost is inside Entity Runtime 0.25.1 (a batch's scoped read
  follows the registry clock's batch closure, which is the whole store): filed as beyond10x/entity-runtime#51 and
  recorded as upstream-blocker:entity-runtime-batch-closure-cost. The story stays active. Shipped: the measurement loop,
  a batch-cost probe and a reopen guard test. A host change skipping the reopen verification saved 1.3% and regressed
  recovery after a timed-out batch; it was reverted. Follow-ups: registry-clock-outside-shared-batches,
  bridge-drop-waits-for-dispatched-batch.
- Implemented: catalog-selection-excludes-parameters (adversary pass 1: 8 cases green, two doc findings),
  setup-initialises-missing-state (no adversary: small defect, per the plan).
- Gate: three local runs at the wave head from a managed worktree. Run 1 failed on two citation cases (fixed in
  67fb9da30: the wave moved lines in er.rs and semantics.md). Run 2: 1169 passed, 1 failed (sql fixture flake,
  outside the wave; 5/5 alone; story:sql-fixture-accepts-stray-connections). Run 3 passed every step through the
  descriptor checks, then the link failed on a full shared disk. The CI repository gate on the wave PR is the closing
  gate; the --msrv steps run with the 0.24.0 release gate.
