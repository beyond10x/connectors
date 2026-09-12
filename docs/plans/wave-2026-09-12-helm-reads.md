# Wave proposal — helm release reads, 2026-09-12

Skill version 0.9.1. Coordinator: this session, in the primary checkout at `ae9950f`,
branch `main`, clean at dispatch. Operator granted standing approval — *work on waves
yourself* — which waives the stage-1 stop and nothing else.

## Verdict

**N is 1, and it is forced rather than chosen.** One story in the whole store is ready.

`aep plan artifact list --kind story --format json`, 59 stories:

| status | count |
|---|---:|
| implemented | 52 |
| active | 5 |
| draft | 2 |

Of the 7 that are not implemented, 6 are blocked, and the coordinator cannot clear any
of them. `aep plan artifact blocked`:

| blocker | blocks | who clears it |
|---|---|---|
| `credential-blocker:gitlab-runtime-sandbox` — open, withholding `test_result` | all 5 active GitLab stories | operator — sandbox URL, project, protected credential path |
| `tooling-blocker:kubernetes-driver-protocol-loading` | `story:kubernetes-spec-service` | a governed run that resolves `adp/1` |

That leaves `story:kubernetes-helm-release-reads`, `draft`, `blocked_by` empty.

## The operator's goal, against the store

The goal set during this session is **10 more stories handled**. It is not reachable from
this backlog: 7 stories exist that are not implemented, 6 of them blocked on something
only the operator or a governed run can supply. Reaching 10 requires **creating**
stories. `epic:mcp-contracts` is `draft` and undecomposed, and its eight contract
deliverables are authoring work that needs no provider, so a decomposer was dispatched
against it in parallel with this wave. That is recorded here as what the coordinator did,
not as a claim that it satisfies the goal.

## What the verb returned, verbatim

`aep plan artifact waves --kind story --status draft`:

```
wave 1
  story:kubernetes-helm-release-reads
wave 2
  story:kubernetes-spec-service
collision: story:kubernetes-helm-release-reads story:kubernetes-spec-service adapters/kubernetes
2 wave(s), 1 collision(s), 0 unassessed
```

Exit 0. Zero unassessed. The verb's wave 2 is excluded here for a reason the verb cannot
see: `story:kubernetes-spec-service` is blocked by `tooling-blocker:kubernetes-driver-protocol-loading`.
The collision it reports would have excluded it from this wave anyway — both stories land
on `adapters/kubernetes`.

The 5 active GitLab stories were not selected. `aep plan artifact waves --kind story
--status active` returns 5 waves and 57 collisions among them; every one is withheld by
the credential blocker and none can reach `implemented`.

## Scope, cited

`story:kubernetes-helm-release-reads` carries 7 typed scope entries, **all `cited`**, none
`inferred`:

```
adapters/kubernetes
adapters/kubernetes/contracts
adapters/kubernetes/generated/descriptor.json
adapters/kubernetes/spec
adapters/kubernetes/src
adapters/kubernetes/tests
docs/local-kubernetes-cli.md
```

No scoper was dispatched: the story already carries typed scope at full confidence, and
re-deriving it would have cost an agent and changed nothing. Blast radius is one adapter
plus one document.

## Pre-flight

| check | value | source |
|---|---|---|
| primary checkout | `ae9950f`, `main`, clean | `git status --short` empty |
| free disk | **51 G, 94% used** | `df -h .` |
| memory | 62 G total, 16 used, 46 available | `free -g` |
| network | reachable, HTTP 200 | `curl api.github.com` |
| measured gate | 476 s at `CARGO_BUILD_JOBS=1`, warm target, primary checkout | wave-2026-09-12-next-three.md |
| `aep` | protocol 0.55.0 | `aep --version` |

**Refusal taken and overridden, with the reason written down.** `git worktree list`
showed six trees from previous waves still standing, which the pre-flight treats as a
refusal. They were inspected rather than removed:

| tree | state | disposition |
|---|---|---|
| `cv2-ess3-adoption` | 8 uncommitted files, another session's | left alone |
| `wt-35ad348b4293` | clean, head `525ec4d` **not** an ancestor of `main` | left alone — it holds a commit `main` does not |
| `initiative-heading-fix-20260911` | clean, merged | `worktree gc` retains it |
| `kubernetes-local-cli-20260911` | clean, merged | `worktree gc` retains it |
| `kubernetes-rebase-20260911` | clean, merged | `worktree gc` retains it |
| `postgres-local-cli-20260911` | clean, merged | `worktree gc` retains it |

`worktree gc --dry-run` retained every tree it listed, machine-wide, on
`worktree-dirty` or `no-remote-recovery-proof`. This repository has no authorized
publication target — `AGENTS.md` § Workspace says a local recovery remote is not public
distribution, and `configuration-blocker:gitlab-v020-publication-target` is open on
exactly that question — so the four merged clean trees cannot produce the remote
recovery proof `gc` requires and cannot be retired by the manager. None of them collides
with this wave's branch or path. Proceeding with a uniquely named tree, and recording the
four as a standing cleanup that needs the publication question answered first.

## Unit 1

| | |
|---|---|
| story | `story:kubernetes-helm-release-reads` |
| objective served | `vision:independent-contract-adapters` |
| worktree | `/home/timo/.local/state/worktree/trees/b10x/connectors_v2/cv2-helm-reads-20260912` |
| managed id | `cv2-helm-reads-20260912` |
| build output | that worktree's own `target/` |
| scratch root | `~/.cache/cv2-wave-b-20260912/unit-1/` |
| branch | `unit/kubernetes-helm-release-reads-20260912` |
| head at dispatch | `ae9950f` |
| integration branch | `wave/helm-reads-20260912`, created at `ae9950f` |
| brief | `~/.cache/cv2-wave-b-20260912/unit-1-brief.md` |
| `subagent_type` | `aep-drive:implementor`, then `aep-drive:adversary` |

**One worktree, not two.** N is 1, so the integration branch has nothing to integrate
that the unit branch does not already hold. The full gate will run in the unit's own
worktree after the adversary passes, on the integration branch fast-forwarded to the
unit's head. This saves roughly 6 G of build output on a disk at 94%. Recorded as a
deviation from the skill's separate-integration-checkout shape.

## Commits this wave makes

One opening commit (this page and the story's move to `active`), one unit commit, the
merge into `wave/helm-reads-20260912`, one closing store commit, and the merge into
`main`. No push, no tag, no release, no version bump. That is the whole of what the
standing approval authorises.

## Deviation recorded at dispatch

The skill says to write this page before dispatching. It was written immediately after
the two agents were launched, not before. Nothing was lost — every path, branch and id
above was assigned before dispatch and passed to the agents in the brief — but the page
did not exist for the minutes between.

## Stages

| stage | state |
|---|---|
| decomposer on `epic:mcp-contracts` | dispatched |
| implementor dispatched | done |
| implementor returned | pending |
| adversary pass 1 | pending |
| full gate with Rust 1.88 | pending |
| store evidence and body update | pending |
| commits | pending |
