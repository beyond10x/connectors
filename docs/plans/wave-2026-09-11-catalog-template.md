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
| 1 | `/home/timo/.local/state/worktree/trees/b10x/connectors_v2/wt-35ad348b4293` | that tree's own `target/` | `/home/timo/.cache/cv2-catalog-tmp` |

With N=1 the integration branch and the unit branch collapse into that one tree, which is already
leased by this session and already holds the three merged catalog commits.

## Commits this wave makes

The opening store commit, one unit commit, the closing store commit, and the merge into `main`.
Nothing else — no push, no tag, no release.

## Stages

| unit | stage |
|---|---|
| 1 | dispatched |

## Agent types

`aep-drive:implementor`, then `aep-drive:adversary`.
