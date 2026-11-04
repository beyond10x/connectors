---
format: aep.planning-md/3
id: approval-record:connectors-wave-2026-11-04
kind: approval-record
status: approved
title: 'Wave 2026-11-04: MCP revision rule, GitHub reads remainder, ESS 0.85.0, provider tests in CI'
relations:
- decides: story:claude-connects-to-the-mcp-server
- decides: story:catalog-github-reads
- decides: story:provider-ignored-tests-run-in-ci
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-04T08:46:50Z", actor: "human:operator", revision: 2}
---
## Wave 2026-11-04 (connectors)

Skill: aep implementing 0.26.0, wave mode. Integration branch `wave/2026-11-04-connectors`, managed
tree `conn-w1104` (`<tree>/target`), from `origin/main` `a99bab09c` (v0.65.2). Targeted tests per
unit, one staged local full gate, one bot pull request merged through `PUT /pulls/<n>/merge`, one
release, v0.66.0.

Selection: `aep plan artifact waves --kind story --status active --format json` at `a99bab09c`
(aep 0.79.0). Before scoping: wave 1 `story:catalog-github-reads`; wave 2
`story:claude-connects-to-the-mcp-server`; collision on `crates/connectors-docs/src/status.rs` (cited);
unassessed `story:connect-names-a-failed-provider-start`,
`story:macos-release-provider-starts-through-the-private-copy`, `story:provider-programs-run-on-macos`,
`story:sustained-live-read-loop`. After `story-scoper` passes on both stories (scope sections and typed
entries written), the same placement, with collisions on `CHANGELOG.md` (inferred), `README.md`,
`crates/connectors-docs/src/status.rs`, `website/data/status.json` and `website/docs/status.md` (cited).

Split by function instead of a second wave: each unit adds its own `status.rs` item and neither edits
`AS_OF`; U1 edits the README MCP paragraph, U2 adds a support-table row; the generated status files and
`CHANGELOG.md` are the coordinator's, regenerated and written at integration; `git merge-tree` dry run
before the first of the two merges.

| unit | story | branch | tree | build directory | scratch | stage |
|---|---|---|---|---|---|---|
| U1 | `story:claude-connects-to-the-mcp-server`: the revision rule only | `unit/2026-11-04-mcp-revision` | `conn-w1104-u1` | `<tree>/target` | `~/.cache/connectors-wave-20261104/u1` | approved |
| U2 | `story:catalog-github-reads`: the remainder as restated | `unit/2026-11-04-github-reads` | `conn-w1104-u2` | `<tree>/target` | `~/.cache/connectors-wave-20261104/u2` | approved |
| U3 | ESS pin 0.84.0 to 0.85.0 (workspace rule, no story) | `unit/2026-11-04-ess-0.85.0` | `conn-w1104-u3` | `<tree>/target` | `~/.cache/connectors-wave-20261104/u3` | approved |
| U4 | `story:provider-ignored-tests-run-in-ci` | `unit/2026-11-04-provider-ignored-ci` | `conn-w1104-u4` | `<tree>/target` | `~/.cache/connectors-wave-20261104/u4` | approved |

Subagent types: `aep:implementor`, `aep:adversary` (U1, U2, U4), `aep:security-reviewer` (U1: it
relaxes a refusal on the tool-call path). U3 is pin-only: no adversary. At most three sub-agents at once.

Build limits (tell / stop): U1 11G and 60 min / 13G and 80 min; U2 8G and 45 min / 10G and 60 min; U3
and U4 8G and 20 min / 10G and 30 min. Every cargo call through the machine's build slot. Gate: 8
serial stages, tell 15G, stop 18G per stage, cache discarded between stages, none under 25G free.

## Decisions taken for this wave (2026-11-04)

The directing session `b10x-chef-orchestrator`, verbatim apart from a name:

- U1: "U1 recording: C. A Claude Code login in an isolated HOME is a credential, and that is
  [the operator's] alone. U1 delivers the revision rule and its tests; the story stays active."
- U2: "your restatement is accepted as written, with the two walks deferred and the moved-file lines
  dropped. Record it in the story."
- U4: "provider-ignored-tests-run-in-ci: accepted, and it is U4 in this wave." The coordinator ran the
  move proposed -> active on that word.
- Pull request #238 (container image) belongs to another session: "Leave it alone; do not merge or
  rebase it."

## Store close riding the wave

| story | merge | PR | verdict |
|---|---|---|---|
| `story:connect-names-a-failed-provider-start` | `a99bab09c` (fix `035b14c7a`) | #237, v0.65.2 | met; moved to implemented |

## Left out

- `story:macos-release-provider-starts-through-the-private-copy`: the macOS kill waits on the
  downstream adopter's answers.
- `story:provider-programs-run-on-macos`: its open cause is the same macOS kill; its other gap, the
  Grafana CLI journey on the macOS job, is verifiable on macOS CI only and does not close the story.
- `story:sustained-live-read-loop`: `credential-blocker:live-read-loop-no-configured-connections`
  (open); depends on drafts `story:upgrade-moves-catalog-adapters` and `story:refusals-name-the-field`.

## Commits this wave makes

The opening store commits (the close above, the scopes and this page), one or more commits per unit
branch, their merges into `wave/2026-11-04-connectors`, fix commits from reviews, the store-close
commit, the release-prepare commit, the push through `b10x-gates bot`, one pull request and its merge
through the App, then the v0.66.0 tag and release page.
