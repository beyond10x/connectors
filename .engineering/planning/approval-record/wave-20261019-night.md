---
format: aep.planning-md/3
id: approval-record:wave-20261019-night
kind: approval-record
status: draft
title: 'Wave 20261019-night: the operator''s approval of the night plan'
revision: 5
---
## Stop

Stage 1 of `aep:implementing` (skill version 0.23.1), wave `wave/20261019-night`. The operator
approved the night plan (`~/.claude/plans/distributed-wondering-star.md`) at 00:30 local on
2026-10-19 and asked at 00:50 that the units run through `aep:implementing` with opus, sonnet or
haiku sub-agents by complexity. The stage-1 stop is therefore waived by the operator; this record is
the wave page.

## Units

| unit | story | objective | implementor | adversary |
|---|---|---|---|---|
| B1 | story:definition-format-write-members | vision:independent-contract-adapters | fable, resumed twice after a 429; committed by the coordinator | one opus pass, bounded to `crates/connectors-catalog/tests/` |
| B3 | story:parity-jira-issue-create-and-comment, story:parity-jira-issue-writes, story:parity-jira-create-meta-per-type | epic:fluxplane-plugin-parity | opus (one implementor, the three stories land on one selection file) | one opus pass |
| B4 | story:slack-single-user-lookups, story:parity-slack-file-delete | epic:fluxplane-plugin-parity | opus (one implementor, same files) | one opus pass |
| B5 | story:remove-unshipped-material-20261019 (Rust half: #12, #16, #13/#9 edges) | the architecture review of 2026-10-19 | sonnet | gate only, unless time allows |
| N1–N3 (done) | story:remove-unshipped-material-20261019 (documents), story:parity-page-recount-20261019, story:definition-authoring-guide | — | opus, finished before this record | none (documentation) |

## Selection path

Scopes were written as typed entries from the four scout cards of 2026-10-19 (format delta,
removal referrers, Jira/Slack entries, recount), each marked cited or inferred; `aep plan artifact
waves --kind story --status active` output is pasted in the section below. The units run
sequentially in ONE tree, `conn-release-0540`, because the host has 35 G free and one connectors
build directory costs 20–30 G: B3's three stories collide on `adapters/catalog/providers/jira/*`
and B4's two on `adapters/catalog/providers/slack/*`, so each unit is one implementor; B3, B4, B5
collide on `crates/connectors-docs/src/status.rs` and the fixtures directory, so they run in order.
This is a recorded deviation from "one worktree per unit".

## Pre-flight

Disk 35 G free at 00:45 (floor 10 G; the final gate reserves 32 G through `conductor build run`,
which reclaims). One measured build: the release tree's warm `target/` is 20 G; package tests of
`connectors-catalog-provider` took about 2 minutes warm on 2026-10-18 23:00. sccache is on
(port 4301). Model budget: the operator named no number; one implementor and one adversary at a
time. `git worktree list`: no previous wave's trees (`conn-w20261017a` and `conn-arch-review-20261019`
removed by `worktree gc --apply` tonight; `conn-github-reads` and `conn-confluence-v1` belong to
other sessions and are left).

## Commits this approval authorises

One or two bot commits per unit in the release tree and the definitions tree; the merges of the
N1–N3 branches into the integration tree; the closing store commits; the merge of
`wave/20261019-night` into `main` through one pull request after the full gate and CI; the
definitions PR; the tag `v0.54.0` and the release page per `AGENTS.md` "Cutting a release", which
the operator's plan approval named. Nothing else.

## Triples

| unit | worktree | build directory | scratch |
|---|---|---|---|
| B1, B3, B4, B5, B6, B7 | `~/.local/state/worktree/trees/b10x/connectors/conn-release-0540` | `<that tree>/target` | `~/.cache/connectors-night-20261019/{b1,b3,b4,b5,b6}` |
| definitions | `~/.local/state/worktree/trees/b10x/connectors-definitions/defs-night` | none (no Rust) | `~/.cache/connectors-night-20261019/b1` |
| N1 | `…/connectors/conn-n1-evidence` (head `d57f6185a`) | none | `…/n1` |
| N2 | `…/connectors/conn-n2-deletions` (head `2b89a4efd`) | none | `…/n2` |
| N3 | `…/connectors/conn-n3-parity-docs` (head `adf3f3e6e`) | none | `…/n3` |

## Stage

Kept current in `~/.cache/connectors-night-20261019/wave.md` (outside the repository) and
in this record's "Stage" section at each unit change.

- 00:45: B0 base commit `6498a881c`; N1, N2, N3 green and committed; B1 in flight (lowerer step).
- 00:50: B1 green (catalog suite, clippy, fmt EXIT=0); committed by the bot as `7af6211` (definitions, branch `night/20261019`) and `04da73c29` (connectors, branch `wave/20261019-night`). Its implementor stopped before committing; the coordinator committed its drafted messages.
- 00:52: B3 dispatched (opus implementor); B1 adversary dispatched (opus, new test file only).
- 01:00-03:30: B1b, B3, B4, B5, B6 committed; adversary passes on B1, B3, B4 recorded as review results (52 cases, 7 red: 5 fixed, 2 recorded as stories `required-body-key-refuses-null`, `required-parameter-refuses-empty`).
- 03:30-04:20: definitions PR merged; connectors PR #190 opened; gate runs 1 and 2 each stopped on one stale test, both fixed (`f651afcbf`, `ad8a3ce84`); gate run 3 exit 0 (2,805 passed).
- 04:20-08:00: two release review runs of ten reviewers each, every finding verified by a second agent against the known-defect register (open stories, story-tagged ignored tests, CHANGELOG limitations, parity open rows). Ledger rows (one row may carry two related findings): run 1, 48 rows: 15 fixed, 3 filed as stories, 9 already known, 19 refuted, 1 untested, 1 the same as another row; run 2, 33 rows: 20 fixed, 4 stories, 3 known, 5 refuted, 1 untested. Fix commits `2491bd191` (macOS lint, found by CI), `ed06b18d3`, `00fd6e879`, `f19fbf9b4`, run-2 docs, lowerer and engine commits, `439ed1a07` (definition connect names its host, refuses a replaced entry, skips pipes).
- 08:00: gate run 6 at `439ed1a07` exit 0 (318 binaries, 2,825 passed, 0 failed, 144 ignored); gate run 5 had failed 4 registry tests with `OutcomeUnknown` during a 148 s build pause, which passed 115 of 115 alone. CI on #190: 8 of 8 green. Nine wave stories implemented; `parity-confluence-cql-search-v1-held` archived.
