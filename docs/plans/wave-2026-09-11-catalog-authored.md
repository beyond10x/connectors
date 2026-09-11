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
| 1 | **merged** as `f3e4564` — 91 executed, all three gates 0, adversary's 4 cases unmodified |

## Cost

| agent | tokens | tool uses | wall |
|---|---|---|---|
| implementor | 105,662 | 53 | 200 s |
| adversary pass 1 | 106,493 | 26 | 257 s |
| implementor correction | 163,121 | 21 | 161 s |
| **total** | **375,276** | **100** | **10.3 min** |

## Scope correction the implementor returned

The scope named three files and the unit had to touch two more: `src/lib.rs`, one line declaring the
module, and `Cargo.lock`, one line adding `toml` to this package's entry — `cargo --locked` refuses
to run once a package gains a dependency. No new package resolved; `toml 0.8.23` was already locked.
`toml` went into the crate manifest rather than `[workspace.dependencies]`, following
`crates/connectors-host/Cargo.toml`, which declares the same dependency the same way.

## Close

One unit, merged. The adversary found seven, one blocker; the correction answered each as a rule
rather than an instance — a table-driven check that every published operation is one the inventory
could have emitted, and an exhaustive refusal table that will not compile until a new variant is
numbered and given a document. Three of the seven had no failing case and were found by mutating a
copy of the crate in scratch; each now has a case that kills its mutant, measured.

The coordinator applied one patch the implementor was held off: `src/template.rs` kept a third copy
of the location-label table, and it now shares `inventory::Location::label`.

Scope corrected in the store after implementation: three paths the story did not name
(`src/inventory.rs`, `src/template.rs`, `Cargo.lock`) and three refusals it did not list.
