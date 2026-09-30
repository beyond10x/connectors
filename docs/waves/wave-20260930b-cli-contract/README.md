# Waves 2026-09-30 (b): CLI contract defects from the 0.18.0 surface test

Skill version: aep implementing 0.18.0. Operator: "use /planning and multiple waves to implement this efficiently
/wave - use /ess:specifying and /ess:hardening to fix and improve specs … use multiple agents where applicable and
conflicts can be prevented".

## Plan (from `aep plan artifact waves` after scoping; each story tests in its own file)

| wave | stories | conflict handling |
|---|---|---|
| 1 | absent-operation-reports-not-found, catalog-parameters-declare-their-type, legacy-commands-parse-like-the-contract | disjoint |
| 2 | failed-connect-reports-its-cause, list-commands-page-with-a-cursor, cli-surface-minor-findings-0-18-0 | `apps/connectors/src/local.rs` split by function; `git merge-tree` dry run before the first merge |
| 3 | configuration-change-error, revision-conflict-wire-code, setup-initialises-missing-state | after the ESS design review of the CLI domain |

Blocked: malformed-input-json-is-invalid-input on upstream-blocker:ess-cli-dynamic-input-code (beyond10x/ess#274).

## Wave 1 units

| story | agents | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|
| story:absent-operation-reports-not-found | aep:implementor, aep:adversary | impl/absent-operation-reports-not-found | `<managed-trees>`/wave0930b-absent | `<tree>`/target | `<wave-scratch>`/absent | dispatched |
| story:catalog-parameters-declare-their-type | aep:implementor, aep:adversary | impl/catalog-parameters-declare-their-type | `<managed-trees>`/wave0930b-params | `<tree>`/target | `<wave-scratch>`/params | dispatched |
| story:legacy-commands-parse-like-the-contract | aep:implementor, aep:adversary | impl/legacy-commands-parse-like-the-contract | `<managed-trees>`/wave0930b-legacy | `<tree>`/target | `<wave-scratch>`/legacy | dispatched |

Integration branch: `wave/20260930b-cli-contract-1` off `main` b50ffba79 (release v0.18.0).

## Commits approval authorises

Per wave: one commit per unit, the merges, the closing store commit, the merge into `main` through a pull request
once that wave's gate is green; a release after the last wave.

## Wave 2 units (same integration branch, base e9af199d8; one gate for waves 1 and 2)

`apps/connectors/src/local.rs` is split by function: failed-connect owns `owner_failure`; list paging owns
`adapters-list`; minor owns the setup-check prerequisite line and the unknown-alias stage. `git merge-tree` dry run
before the first wave 2 merge.

| story | agents | branch | worktree | stage |
|---|---|---|---|---|
| story:failed-connect-reports-its-cause | aep:implementor, aep:adversary | impl/failed-connect-reports-its-cause | `<managed-trees>`/wave0930b-connect | dispatched |
| story:list-commands-page-with-a-cursor | aep:implementor, aep:adversary | impl/list-commands-page-with-a-cursor | `<managed-trees>`/wave0930b-paging | dispatched |
| story:cli-surface-minor-findings-0-18-0 | aep:implementor, aep:adversary | impl/cli-surface-minor-findings-0-18-0 | `<managed-trees>`/wave0930b-minor | dispatched |
| story:catalog-api-base-gateway-prefix (joined wave 1) | aep:implementor, aep:adversary | impl/catalog-api-base-gateway-prefix | `<managed-trees>`/wave0930b-gateway | dispatched |
