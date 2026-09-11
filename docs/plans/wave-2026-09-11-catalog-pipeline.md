# Wave — source to indexed bundle in one run, 2026-09-11

Skill version 0.9.1. Coordinator: window `cv2-aep`. Pre-approved: "continue with next 3 waves";
this is the third and last of them.

## Unit

| unit | story | objective | scope |
|---|---|---|---|
| 1 | `story:catalog-bundle-pipeline` | `vision:independent-contract-adapters` | cited for both files it writes, inferred for `src/lib.rs` |

N is one, for the same reason as the last two waves: the other ready stories are both in
`adapters/kubernetes`, which another session owns.

This unit adds no behaviour of its own — it composes the four modules the earlier waves landed and
records what happened. That is deliberate: it is the first unit whose defects, if any, are about
sequencing and atomicity rather than about a value.

## Pre-flight

| check | value |
|---|---|
| working tree | clean, `main` at the wave-6 close |
| tree | reset hard onto `main` |
| scratch | 502 M before this wave, from earlier adversaries' probe and mutant trees; cleared at close |
| model budget | waves 4, 5 and 6 cost 592 k, 375 k and 239 k |

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
