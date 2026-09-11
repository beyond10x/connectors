# Wave — catalog operation template, 2026-09-11

Skill version 0.9.1. Coordinator: window `cv2-aep`, session `dce4121b`.
Approval: pre-approved by the operator — "keep implementing, pick your stories yourself".
The page is written anyway; approval removes the stop, never the page.

## Units

| unit | story | objective it serves | scope confidence |
|---|---|---|---|
| 1 | `story:catalog-operation-template` | `vision:independent-contract-adapters` | cited for both files it writes, inferred for `src/lib.rs` (module declaration) |

N is one. The other two ready candidates are `story:kubernetes-helm-release-reads`, which is the
declared next work of window `cv2-gitlab` and lands in `adapters/kubernetes`, and
`story:kubernetes-spec-service`, which is blocked on tooling. Taking either would put two sessions
on one surface.

## What `aep plan artifact waves --kind story --status draft` returned, verbatim

```
wave 1
  story:catalog-operation-template (inferred)
  story:kubernetes-helm-release-reads
wave 2
  story:kubernetes-spec-service
collision: story:kubernetes-helm-release-reads story:kubernetes-spec-service adapters/kubernetes
2 wave(s), 1 collision(s), 0 unassessed
```

Selection path: the verb, not a pairwise reading. `aep --version` is `protocol 0.55.0`.
The verb places unit 1 beside `kubernetes-helm-release-reads` with no collision, and that is
correct on surfaces — the exclusion is ownership, not overlap, and it is the coordinator's
judgement rather than the verb's.

## Pre-flight

| check | value |
|---|---|
| working tree | clean but for this wave's own story file and journal, committed as the opening commit |
| branch | `main`, at `c730cd7` |
| free disk | **18 G, 98% used** — thin. The unit is package-scoped and its warm target is 183 M |
| build directory | reused warm tree, 183 M, incremental build measured at 3.3 s |
| model budget | the operator reported the weekly limit at 96% earlier today; N=1 is sized to that |
| `AGENTS.md` | read this session |

## Triples

| unit | worktree | build directory | scratch root |
|---|---|---|---|
| 1 | `~/.local/state/worktree/trees/b10x/connectors_v2/wt-35ad348b4293` | that tree's own `target/` | `~/.cache/cv2-catalog-tmp` |

With N=1 the integration branch and the unit branch collapse into that one tree, which is already
leased by this session and already holds the three merged catalog commits.

## Commits this wave makes

The opening store commit, one unit commit, the closing store commit, and the merge into `main`.
Nothing else — no push, no tag, no release.

## Stages

| unit | stage |
|---|---|
| 1 | **left the wave** — red after two attacks. Pass 1: 7 findings, all resolved. Pass 2: 5 new, 2 blockers. `decision-blocker:catalog-template-second-pass` is open and the tree is retained |

## Agent types

`aep-drive:implementor`, then `aep-drive:adversary`.

## Cost

| agent | tokens | tool uses | wall |
|---|---|---|---|
| implementor unit 1 | 88,268 | 42 | 165 s |
| adversary unit 1 pass 1 | 67,603 | 25 | 133 s |
| implementor unit 1 correction | 136,443 | 17 | 142 s |
| adversary unit 1 pass 2 | 114,507 | 12 | 131 s |
| **total** | **406,821** | **96** | **9.7 min** |

## Coordinator actions the implementor could not take

`cargo fmt --package connectors-catalog -- --check` reported 11 hunks, all in files that landed in
the three earlier catalog waves and none in the unit's own two files. The repository gate builds
that check per workspace member, so `main` is currently red on it. The implementor wrote the fix
and left it unapplied because those files were not its to edit; the coordinator applied it, and the
package is fmt-clean, test-green and clippy-clean after it.

## Close

Zero units merged. The unit is green on its own suite — 51 cases, `cargo test`, `cargo clippy
-D warnings` and `cargo fmt --check` all exit 0 with the adversary's eleven cases in the tree —
and red on its second adversary pass, which is the row the skill sends to a person rather than to
a third attack.

The whole-repository gate was not run: there was nothing to merge into the base branch, and a gate
on an unmerged tree answers a question nobody asked.

Retained: worktree `wt-35ad348b4293` with all of unit 1's work uncommitted, and the coordinator's
rustfmt correction to the three earlier catalog commits, which `main` still needs.
