# Hand-over: connectors session, 2026-10-06

Uncommitted on purpose: no approval covers a hand-over commit. Commit it with the wave-7 work.

## Shipped

Release [v0.31.0](https://github.com/beyond10x/connectors/releases/tag/v0.31.0): the tag points
at merge commit `fac31be`, and the bot is the tagger and the release page author (Latest).
`gate --msrv` passed on `6db05b3`: 1,394 passed, 0 failed. Website checks exited 0.

| PR | Story (`.engineering/planning/story/`) | State on main |
|---|---|---|
| #113 | dependency-refresh-20261006 | implemented |
| #116 | upgrade-adapter-identity-mismatch-names-new-connection | implemented |
| #117 | forge-issue-create (closed #81) | implemented |
| #118 | service-failure-carries-upstream-reason | implemented |
| #119 | registry-clock-floor-growth (part of #101) | implemented |
| #121 | catalog-honours-retry-after | implemented |
| #122 | owner-memory-bounded (part of #103) | active, blocked |
| #123 | expired-evidence-invoke-advises-revalidate | implemented |
| #125 | release-plan:connectors-v0310-probe-waves | active (0.30.0's plan also stayed active) |

## Open

- `story:owner-memory-bounded` is active behind `upstream-blocker:er-model-record-copies`
  (https://github.com/beyond10x/entity-runtime/issues/59). Re-measure with
  `read_invoke_cost_by_store_size` once an Entity Runtime release fixes #59 or
  https://github.com/beyond10x/entity-runtime/issues/55.
- `story:cli-json-answers-as-json` (wave 7, #105) is draft behind
  `upstream-blocker:ess-cli-json-primitive` (https://github.com/beyond10x/ess/issues/468, which
  the ess-ship session plans for ESS 0.55.0). It is on branch `feat/cli-json-answers-as-json`
  (pushed, `5300858`) in worktree `conn-w7`. The branch holds the `cli.yaml` `Json` types, the
  contract fixtures and four failing tests. The detail is in `.local/blocker-ess-json.md` in this
  tree.
- Issues #101, #103 and #105 stay open. Each has a v0.31.0 comment where it applies.

## Next step

When ESS 0.55.0 ships:
1. Move connectors from ESS 0.53.0 to 0.55.0, skipping 0.54.0, in `toolchain.json` and the
   seven `Cargo.toml` ESS crates. Follow the adoption procedure in `docs/development.md`.
2. Merge `main` into `feat/cli-json-answers-as-json`.
3. Regenerate the CLI contract.
4. Implement the handler change in `apps/connectors/src/local/operations.rs` until the four
   tests pass.

## Environment

- `ess` on PATH is 0.54.0, but connectors pins 0.53.0. Gates and the website build pass only
  with `CONNECTORS_ESS=~/.cache/ess-0.53.0/ess-0.53.0-x86_64-unknown-linux-gnu/ess`, the 0.53.0
  release binary checked against SHA256SUMS.
- `google_calendar_gmail_adversary_pass2::an_attachment_under_the_named_limit_…` is flaky on
  main: about 1 failure in 12 runs, `Unavailable` on a 3 MiB read.
- Four evidence records from waves 2a and 3 carry hand-typed `at` times later than when they
  were recorded. They are on main, and the store has no command to correct them.

## Worktrees and branches

- Worktree `conn-w7` is kept. It holds wave 7 and this uncommitted file.
- Every other worktree of this session was finished and removed, with archives under
  `~/.local/state/worktree/archives/connectors/`.
- No build directories, unpushed commits, open PRs or open dispatches remain from this
  session.
