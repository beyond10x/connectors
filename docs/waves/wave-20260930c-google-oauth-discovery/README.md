# Wave 2026-09-30c: OAuth refresh profile and Discovery projection

Skill version: aep implementing 0.19.1. Operator approved the plan for `epic:google-workspace-reads`
on 2026-09-30, naming this wave (refresh profile, Discovery projection, repeated query parameters).

## Units

| story | serves | agents | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|---|
| story:catalog-oauth2-refresh-profile | vision:independent-contract-adapters | aep:implementor, aep:adversary | impl/catalog-oauth2-refresh-profile | `<managed-trees>`/wave0930c-refresh | /dev/shm/wave0930c-refresh-target | `<wave-scratch>`/refresh | dispatched |
| story:catalog-discovery-projection | vision:independent-contract-adapters | aep:implementor, aep:adversary | impl/catalog-discovery-projection | `<managed-trees>`/wave0930c-projection | /dev/shm/wave0930c-projection-target | `<wave-scratch>`/projection | dispatched |

Integration branch: `wave/20260930c-google-oauth-discovery` off `main` b50ffba79 (release v0.18.0).
Integration tree `<managed-trees>`/wave0930c-integration.

## Selection

`aep plan artifact waves --kind story --status draft` placed the three planned stories in one wave.
`story:catalog-repeated-query-parameters` left the wave: it edits `adapters/catalog/src/lib.rs`, which
the concurrent wave 20260930b's `story:catalog-parameters-declare-their-type` edits. It runs after
that wave merges.

## Pre-flight

- `/` at 100% (1.1G free) at dispatch time, down from 28G free 10 minutes earlier; the growth is
  not this wave's. Build output, `TMPDIR` and caches for this wave are on `/dev/shm`
  (`CARGO_INCREMENTAL=0`, debuginfo off, no sccache); only git writes land on `/`.

## Commits approval authorises

Two unit commits, their merges into the integration branch, the closing store commit, and the merge
into `main` through a pull request once the gate is green. Not a tag or a release.
