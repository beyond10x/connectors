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
| 1 | **merged** as `c87066b` — 110 executed, all three gates 0, adversary's 4 cases unmodified |

## Cost

| agent | tokens | tool uses | wall |
|---|---|---|---|
| implementor | 65,413 | 25 | 111 s |
| adversary pass 1 | 67,998 | 19 | 144 s |
| implementor correction | 105,520 | 13 | 119 s |
| **total** | **238,931** | **57** | **6.2 min** |

## What the implementor flagged for the adversary

It added a `source_file_name` field the story's Scope does not name, and it reports unknown methods
after the known seven rather than dropping them, with no case beyond compilation exercising that.
It verified its determinism case can fail: deleting one `sort_unstable()` turns the lane red, and it
measured that rather than asserting it.

## Close

One unit, merged. The adversary broke the unit's own claim with its own input: a parameter name
holding a newline let a document write the report's lines, including `coverage: 100% of operations`
— the sentence the Outcome exists to prevent, printed rather than computed. Both of the unit's
guards had asserted the property over two curated fixtures, which is why they passed.

The correction escapes and quotes at the render boundary rather than refusing at ingest, on the
argument that a newline is legal in JSON, in a parameter name and in a Linux file name, so refusing
would make a lawful document un-ingestible to buy a property belonging to the rendering. It was
measured rather than asserted: disabling the escape turns eight cases red across both lanes.

The guard is now a function of the value — the expected line count is derived from the report, and
the poisoned-field matrix is an exhaustive struct literal that will not compile if a field added
later is left out.

Coordinator decision taken: `source_file_name` stays and the story's Scope was amended to name it.
Provenance a reader cannot tie to a file is weaker, and the forgery it invited is answered by
delimiting the name and the digest.
