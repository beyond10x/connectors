# Wave: CLI output readability — 2026-09-04

Skill version 0.6.1 (`.claude-plugin/plugin.json`). Status: **stage 2, approved and running.**

## Unit

| | |
|---|---|
| story | `story:informative-command-readability` |
| status | draft |
| scope | `crates/connectors-console/src/output.rs`, `crates/connectors-console/src/doctor.rs` — both **cited** |
| blast radius | one package, `connectors-console` |
| objective | O5, the generic agent platform — the catalogue a platform tenant configures connectors from |
| implementor | `adp:implementor` |
| adversary | `adp:adversary` |
| branch | `impl/informative-command-readability`, forked from `wave/cli-output` at `7dc4c07` |
| worktree | `/home/timo/.local/state/worktree/trees/b10x/connectors/wt-d41dd2700a2e` |
| build directory | `/home/timo/.local/state/worktree/trees/b10x/connectors/wt-d41dd2700a2e/target` |
| scratch root | `/home/timo/.cache/connectors-wave/informative-command-readability/scratch` |
| stage | **closed, merged** |

## Acceptance, from the artifact

`connectors doctor`, `providers` and `auth status` each render in `text` as one aligned row per
record with the severity or readiness of a row distinguishable without reading its text, no output
format drops a field the value carries, and the bytes of `-o json` and `-o yaml` are unchanged.

## Selection

Computed, not judged: `aep artifact waves --kind story --status draft`, binary `protocol 0.50.0`.

```
wave 1
  story:informative-command-readability
1 wave(s), 0 collision(s), 24 unassessed
```

Collisions: **none returned.**

Unassessed, verbatim, all 24:

```
story:adopt-token-response-metadata
story:auth-as-tool-result
story:connect-session-oauth-custody-in-personal-posture
story:contract-bundles-are-versioned-and-pinned
story:coverage-regains-its-second-direction
story:declarative-webhook-routing-grammar
story:deployment-catalogs-are-external-packs
story:deployment-declared-destination-aperture
story:header-name-rate-limit-retry
story:kubernetes-joins-the-catalog
story:m2-the-platform-skeleton-serves
story:m3-connect-a-provider-and-invoke-it
story:m4-events-reach-a-client-by-push-and-by-pull
story:mint-source-entries-from-the-mined-catalogs
story:one-composed-local-placement
story:one-connection-config-shape
story:per-service-verification-probes
story:production-credential-custody-closes
story:raw-proxy-is-break-glass-not-a-model-capability
story:satellite-federation-has-a-threat-modeled-contract
story:the-anthropic-api-key-arrives-through-a-connect-session
story:the-claims-journal-survives-a-full-life
story:the-cli-drives-a-hosted-connection
story:the-hosted-posture-connects-a-catalogued-provider
```

Those 24 arrived in tonight's backlog migration and carry no typed scope, because scope was recorded
only for stories that were `active` at the time. They are **unassessed, not safe**. They were not
scoped for this wave because the operator named the unit rather than asking for a selection; scoping
them is the prerequisite for any future wave chosen by the store rather than by name.

N is 1. That is a wave.

## Pre-flight — PASSES

| check | value | verdict |
|---|---|---|
| working tree clean | only this page, untracked | pass |
| free disk | 84 G, 90% used | pass |
| previous waves' trees | none of this wave's; four unrelated trees from other sessions | pass |
| build cache | `~/.cache/sccache` was deleted in tonight's disk incident and rebuilds cold | note |
| model budget | not stated; N is 1 so the default of 4 does not bind | note |
| `AGENTS.md` | read; it declares no branch prefixes, so the skill's defaults are used | pass |

`web/` was 242 MB of `node_modules`, VitePress build output and one generated `catalog.json`, with
no hand-written source and no `package.json`. The operator confirmed it held nothing and directed
its removal; it was deleted rather than overridden, so the dirty-tree refusal is cleared rather
than waived. It was the sole cause of
`json_governance::every_repository_json_is_classified_and_valid` failing locally.

## Commits this wave's approval would authorise

One unit commit on `impl/informative-command-readability`; its merge into the integration branch;
one closing store commit recording evidence and moving the story; and the merge of the integration
branch into `main`. **Nothing else** — no push, no tag, no release, no second wave.

## Deviations

- The opening commit (`7dc4c07`) was made on `main` rather than on `wave/cli-output`. It holds the
  wave page and the story's lifecycle moves, no code, and it passed the cheap gate steps
  (`check-links.py` 0, `check-stories.py` 0, `aep artifact validate` valid) before any tree existed.
  `wave/cli-output` forks from it.
- Worktrees are created through this machine's managed `worktree` CLI rather than under a wave-root
  directory, because the operator's standing rule requires it. The triple above records the paths
  the skill's cleanup step needs.

## Log

| when | event |
|---|---|
| dispatch | `adp:implementor` returned **green**, cases 44 → 55, 9 red first. Commits `facd0b1`, `7daeb22`. |
| attack 1 | `adp:adversary` returned **red**, cases 55 → 62, 6 red. 9 findings introduced, 1 pre-existing. Recorded as `review-result:adversary-informative-readability-pass-1`. |
| routing | 9 introduced returned to the same implementor. 1 pre-existing filed as `story:emit-treats-a-closed-pipe-as-failure`. |
| incident | the implementor was killed mid-correction by an API `authentication_failed`, not a rate limit. Branch held both original commits plus an uncommitted measurement probe; no correction had landed. Resumed rather than re-dispatched. |
| correction 1 | returned **green**, cases 62 → 72, all 6 adversary cases now pass. Commit `06e1e22`, +794/−71 over two files. `providers` last column moved 196 → 120; compact `wc -l` 66 → 65. |
| attack 2 | returned **red**, cases 72 → 77, 4 red, 9 findings. Recorded as `review-result:adversary-informative-readability-pass-2`. |
| ledger | `aep artifact findings` between the two passes: **carried 0, new 9, resolved 10.** The correction landed whole; nothing regressed. The nine are ground the correction itself opened. |
| correction 2 | returned **green**, cases 77 → 80. Commit `fc3a91b`. |
| verification | the coordinator's, replacing the third attack. Two assertions removed against 18 added; the one deleted test was the implementor's own, whose contract the blocker fix overturned. The one edited adversary needle was re-run against the pre-fix renderer: **4 of 5 still fail**, so it was not neutered. |
| integration | unit merged as `e211b07`. The gate lane for `connectors-console` added as `b256d74`. The size fence then refused `output.rs` at 1556 lines, so its tests were split out as `34151f1`, following the pattern `document.rs` and `backend.rs` already use. |
| gate | `scripts/gate.sh` sharded over 12 workspaces plus `--final`: **every step exit 0**, no step skipped. Log `~/.cache/connectors-gate/wave-gate2.log`. |

## What the unit changed, measured

| command | before | after |
|---|---|---|
| `doctor` text | 26 lines for 6 checks | 9 |
| `providers` text, last column begins at | wrapped, 927 lines | column 119 of a 120 budget, every id whole |
| `providers -o compact`, `wc -l` | 65, `healthy` and siblings dropped | 65, every field carried on every line |

## Cost

| agent | tokens | tool uses | wall |
|---|---|---|---|
| `adp:implementor` (3 rounds, one resumed after an auth kill) | 716,493 | 170 | 43m |
| `adp:adversary` pass 1 | 151,468 | 55 | 21m |
| `adp:adversary` pass 2 | 172,859 | 58 | 19m |

## Findings routed out of this unit

- `story:console-clippy-findings` — five pre-existing clippy findings in `enrol.rs`/`envelope.rs`.
- `story:emit-treats-a-closed-pipe-as-failure` — `emit` returns `Err(BrokenPipe)`; pre-existing.

## Coordinator work still owed

- Apply `scratch/gate-console-lane.patch` on the integration branch. `crates/connectors-console` is
  in the root workspace's `exclude` list and in no `scripts/gate.sh` lane, so its ~44 tests are run
  by no gate — verified independently: `gate.sh --list-workspaces` returns 10 workspaces and none is
  this one. Without the lane, this wave's own closing gate would not execute the unit's tests.
