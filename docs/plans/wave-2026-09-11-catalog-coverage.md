# Wave — bundle coverage report, 2026-09-11

Skill version 0.9.1. Coordinator: window `cv2-aep`. Pre-approved: "continue with next 3 waves";
this is the second.

## Unit

| unit | story | objective | scope |
|---|---|---|---|
| 1 | `story:catalog-coverage-report` | `vision:independent-contract-adapters` | cited for both files it writes, inferred for `src/lib.rs` |

N is one. The two other ready stories remain `kubernetes-helm-release-reads` and
`kubernetes-spec-service`, both in `adapters/kubernetes`, which another session owns; the verb's
collision line between them is unchanged from the last two waves.

## Pre-flight

| check | value |
|---|---|
| working tree | clean, `main` at the wave-5 close |
| tree | reset hard onto `main`, so the unit builds on the merged authored reader |
| model budget | wave 4 cost 592 k, wave 5 cost 375 k; this one is sized the same as wave 5 |

## Triple

| unit | worktree | build directory | scratch root |
|---|---|---|---|
| 1 | `~/.local/state/worktree/trees/b10x/connectors_v2/wt-35ad348b4293` | that tree's `target/` | `~/.cache/cv2-catalog-tmp` |

## Commits this wave makes

The opening store commit, one unit commit, the closing store commit, and the merge into `main`.

## Stages

| unit | stage |
|---|---|
| 1 | dispatched |
