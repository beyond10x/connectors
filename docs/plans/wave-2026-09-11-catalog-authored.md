# Wave — locally authored TOML action, 2026-09-11

Skill version 0.9.1. Coordinator: window `cv2-aep`. Pre-approved: "continue with next 3 waves".
This is the first of those three.

## Unit

| unit | story | objective | scope |
|---|---|---|---|
| 1 | `story:catalog-local-toml-action` | `vision:independent-contract-adapters` | cited for both files it writes, inferred for the crate manifest |

N is one. It is the operator's own requirement from the HTTP/catalog handoff — a lightweight local
TOML file expanding into the same validated template an imported OpenAPI operation produces — and it
depends on the template unit that merged as `9b88cd0`.

## Pre-flight

| check | value |
|---|---|
| working tree | clean, `main` at `2c66010` |
| free disk | re-read at wave start, printed below if it crossed the floor |
| `toml` | already in `Cargo.lock`; the unit adds it as a direct dependency of this crate only |
| model budget | wave 4 cost 592 k tokens across four agent runs; this one is sized at one implementor and one adversary pass |

## Triple

| unit | worktree | build directory | scratch root |
|---|---|---|---|
| 1 | `~/.local/state/worktree/trees/b10x/connectors_v2/wt-35ad348b4293` | that tree's `target/` | `~/.cache/cv2-catalog-tmp` |

The tree is rebased onto `main` before dispatch, so the unit builds on the merged template.

## Commits this wave makes

The opening store commit, one unit commit, the closing store commit, and the merge into `main`.

## Stages

| unit | stage |
|---|---|
| 1 | dispatched |
