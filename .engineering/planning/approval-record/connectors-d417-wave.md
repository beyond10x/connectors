---
format: aep.planning-md/3
id: approval-record:connectors-d417-wave
kind: approval-record
status: approved
title: 'Wave D417: cold owner start, and the store close of merged stories'
relations:
- decides: story:cold-owner-start-verifies-once
revision: 4
transitions:
- {from: "draft", to: "approved", at: "2026-11-03T13:09:52Z", actor: "human:operator", revision: 2}
---
## Wave D417

Skill: aep implementing 0.26.0, wave mode. Integration branch `wave/20261103`, managed tree
`conn-w1103b`, from `origin/main` `8150245c2` (v0.65.0). Targeted tests per unit, one local full gate
after the orchestrator's go, one bot pull request merged through `PUT /pulls/<n>/merge`, one release.

Selection: wave 1 of `aep plan artifact waves --kind story --status active --format json` at
`8150245c2`, after `story-scoper` scoped the unassessed `story:cold-owner-start-verifies-once`.
Verb output, wave 1: `story:catalog-feed-engine-extensions`, `story:cold-owner-start-verifies-once`,
`story:docs-first-run-path`. The first and third are merged on main (below) and leave the wave as
code units. N = 1.

| unit | story | branch | tree | build directory | scratch | stage |
|---|---|---|---|---|---|---|
| U1 | `story:cold-owner-start-verifies-once` | `unit/d417-cold-owner-start` | `conn-w1103-u1` | `<tree>/target` | `~/.cache/connectors-wave-20261103/u1` | dispatched |

Subagent types: `aep:implementor`, `aep:adversary`, `aep:security-reviewer` (the unit changes store
verification before any provider call).

## Store close of stories already merged on main

Checked 2026-11-03 against git (each merge an ancestor of `origin/main`, `git merge-base
--is-ancestor`) and the acceptance read against the tree at `8150245c2`:

| story | merge | PR | verdict |
|---|---|---|---|
| `story:catalog-feed-engine-extensions` | `c83272c5b` | #197 | met |
| `story:connection-follows-configuration-upgrade` | `25ca4eb3d` | #197 | met |
| `story:definition-reader-feed-extensions` | `f1089f007` | #201 | met |
| `story:docs-first-run-path` | `5c2fb81e9` | #201 | met at merge |
| `story:mcp-stdio-runtime-branch` | `7b38a2984` | #201 | met; finding J3 deferred to `story:owner-cancels-a-bounded-read` (draft) |
| `story:cli-lists-available-connectors` | `d1309030d` | #201 | met |
| `story:catalog-provider-failure-is-named` | `8150245c2` | #235 | met |
| `story:program-start-failure-reason` | `8150245c2` | #235 | met |
| `story:kubernetes-exec-plugin-on-macos` | `8150245c2` | #235 | met as amended by D372 |

Stay active, with the gap named:
- `story:provider-programs-run-on-macos`: the Grafana CLI journey is ignored on the macOS job (run
  41463903366), and the cause the story asks to record is not recorded.
- `story:catalog-github-reads`: its acceptance names `adapters/catalog/providers/github/operations.json`,
  a bundle and `docs/catalog-github.md`, surfaces that moved to connectors-definitions
  (https://github.com/beyond10x/connectors-definitions/pull/9, merged); no two-page paging test and
  no 1,000-result cap test on main. The acceptance needs restating by the plan owner first.

## Left out

- `story:sustained-live-read-loop`: `credential-blocker:live-read-loop-no-configured-connections`;
  depends on drafts `story:upgrade-moves-catalog-adapters` and `story:refusals-name-the-field`.
- `story:claude-connects-to-the-mcp-server`: its session fixture is recorded from a real Claude Code
  client, which needs a client login in an isolated home (a credential).

## Release scope

0.65.1: U1 (`story:cold-owner-start-verifies-once`, merged at `e60844a1d`; its D422 review round 2 at
`d62f07e40`, round 3 at `7ef244880`) and U2 (ESS 0.84.0, merged at `011ec3139`). The reviews are
`review-result:wave-d417-u1-adversary`, `-adversary-2`, `-adversary-3` and
`review-result:wave-d417-u1-security`; the decision on them is
`approval-record:connectors-d417-d422-u1-reviews`, and round 3 (a locked open waits for an
inspection's handle) was the lead's call on the leftover inside the acceptance.

The full gate ran staged on `7ef244880` under host-monitor's go, with `worktree discard-cache` between stages
(`~/.cache/connectors-wave-20261103/gate/`): S1 drift and boundaries exit 0 (305 passed, peak 5G);
S2 thirteen library packages exit 0 (1525 passed, 8G); S3 `apps/connectors` failed on its first run
only because `-p connectors` does not build the `connectors-catalog-provider` binary two tests
look up beside it, and passed on its approved rerun after building it (532 passed, 10G); S4 adapters
exit 0 (1447 passed, 16G against a 10G row, revised to 18G during the run); S5a to S5c clippy
`-D warnings` on all 23 packages exit 0. CI's repository gate on the pull request runs what the
staged gate leaves out: the authored scenarios, metadata conformance and the OAuth2 boundary.

## Commits this wave makes

The opening store commit, one or more commits on U1's branch, its merge into `wave/20261103`, fix
commits from its reviews, the store-close commit, the release-prepare commit, the push of
`wave/20261103` through `b10x-gates bot`, one pull request and its merge through the App, then the
tag and release after the orchestrator's go.

## Approval

The orchestrator, 2026-11-03, D417: "run a connectors wave from main's 13 active stories. Selection:
wave 1 of `aep plan artifact waves --kind story --status active` on a tree at origin/main 8150245c2.
Leave out any story that is blocked (dependency, credential, live account, device) or whose body
records it as delivered (check that against git). ... At most 4 units."
