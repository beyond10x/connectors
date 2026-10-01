# Wave 2026-10-02 (a): metadata store cost, catalog parameter exclusion, setup on missing state

Skill version: aep implementing 0.18.0. Status: approved by the operator 2026-10-02 (plan approved in session).

## Units

| story | serves | scope | agents | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|---|---|
| story:metadata-invoke-cost-flat-in-store-size | vision:independent-contract-adapters | inferred | aep:implementor (aep:diagnosing method), aep:adversary | impl/metadata-invoke-cost-flat-in-store-size | `<managed-trees>`/wave1002a-store | `<tree>`/target | `<wave-scratch>`/store | dispatched |
| story:catalog-selection-excludes-parameters | vision:independent-contract-adapters | cited (two inferred tests) | aep:implementor, aep:adversary | impl/catalog-selection-excludes-parameters | `<managed-trees>`/wave1002a-params | `<tree>`/target | `<wave-scratch>`/params | dispatched |
| story:setup-initialises-missing-state | vision:independent-contract-adapters | cited | aep:implementor | impl/setup-initialises-missing-state | `<managed-trees>`/wave1002a-setup | `<tree>`/target | `<wave-scratch>`/setup | dispatched |

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
