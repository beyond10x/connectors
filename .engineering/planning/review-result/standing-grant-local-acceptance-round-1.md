---
format: aep.planning-md/3
id: review-result:standing-grant-local-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1, story:local-invoke-admits-a-keyed-write-under-a-standing-grant
relations:
- reviews: story:local-invoke-admits-a-keyed-write-under-a-standing-grant
revision: 1
---
verdict: needs-revision

```findings
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 224
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the run command cannot decide lines 1-16: `connectors-build ignored` has no name filter (ignored.rs Options), so it runs the whole disposable lane one process per test, sequentially, each allowed 600 s (ignored.rs:801), and the CI job is allowed 120 minutes; that breaks the 120 s step and 10 min pool limits. Replace with one named command that runs only the new module (for example `slot cargo test -p connectors-catalog-provider --test local_runtime -- --ignored slack_standing` with CONNECTORS_TEST_CLI set, or a `--name` filter added to the runner) and say that `.local/ignored-suites.json` lists all 16 names as selected and passed."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 224
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the harness it names, `slack_approval::a_slack_bot_post_is_issued_...`, is classified Deferred in ignored.rs:542 because it fails in the CI journeys container on main (story:main-failing-ci-journeys), and Deferred tests are never selected by `--required --only-prerequisite provider,cli,custody` (ignored.rs:1550); add a line that the 16 tests are classified Disposable with prerequisites provider,cli,custody and pass in the CI journeys job, or name the prerequisite they lack."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 266
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "line 23 is a report, not a check: each test would rebuild Slack TLS fixture, dbus keyring, `setup init`, `connections connect`, `key-init`, two `policy-set`, `describe` (slack_approval.rs:247-346) in its own process, and the story has no measured time for one. Replace with: one measured run of a single new test is stated in the story before the other 15 are written, the state is built once and copied per test, and the runner's report shows every new test under 60 s."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 234
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "lines 2-6 and 11-15 (and 16) assert only a refusal that the gate already gives today for a keyed write without a proof, so they pass for the wrong reason; add to each a control, an in-grant send admitted under the same grant file (exit 0, `.result.mutation.standing.grant_id`), and for 2-6, 11-16 state the shape once: exit 1, `.error.data.code` `approval_required`, `.error.data.mutation.classification` `not_attempted`, no `.error.data.mutation.standing`, the listener's total request count (including auth.test) unchanged, and no attempt row in the ledger."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 246
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "no line checks the signature cases or custody refusals the Design promises; add tests: a per-input proof from `approvals issue` placed at the `standing_grants` path (wrong `typ`), wrong `iss`, wrong `aud`, a grant signed before `approvals key-rotate`, a grant signed by a revoked key, and a grant file that is group- or world-writable, foreign-owned or a symlink or directory (`unavailable`, `not_attempted`, no request)."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 246
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "line 11 does not name a replay after expiry; add: after `not_after` the first key still replays with the same `grant_id` and one post (line 7 covers only the cap, line 12 only revocation). Line 10 and 11 also need the clock fixture (slack_approval.rs:200-221, elapsed-time only) to take a settable offset; say that the test sets it."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 242
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the use-count promises have no line: add (unit on `Uses` or the executor) a refusal after `take` releases the use (max_per_hour 1: refused call, then an admitted one), a failed `bind` sends no chat.postMessage and leaves no counted use, and after `unknown` or a provider refusal with max_per_hour 1 a fresh key answers `rate_limited` (once an attempt exists its use stays counted). Line 9's fixture also needs a mode that answers `ok: false` for the granted channel; the current one does not (slack_approval.rs:113)."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 252
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the two commands have no line of their own and `--approval-file` with a grant is untested; add: `grant-standing` prints `{grant_id, path}`, the file mode is 0600, a second grant keeps the first; both commands exit 1 `invalid_configuration` without a `standing_grants` member; `revoke-standing` of an unknown id; with the member set and `--approval-file` given (valid and invalid proof) the call takes today's path, carries no `standing`, and an invalid proof is not retried as a grant; with the member set and no `--idempotency-key` the bytes are today's."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 255
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "lines 18-20 give a name but no expected result: add that the unit test asserts the `Uncovered::OutsideInput` variant per leaf, covers `input_keys` absent (open), `[]`, a null leaf and the 16-pointer and 256-byte limits refused at `grant-standing`, and that line 20 states the served host's status and listener count for an uncovered member; line 19 adds that a rewriter (migrate.rs, select.rs) keeps the member."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 259
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "line 21 diffs against the moving `origin/main`, so any other merged story touching those files fails it, and omits `apps/connectors/tests/invoke_short.rs` which line 17 calls unchanged; use `git diff --quiet $(git merge-base origin/main HEAD) --` with literal paths (no brace expansion in sh), add invoke_short.rs, and name the command that runs the tests (all listed paths exist)."
```

Read: 1 story (`aep plan artifact show`), `slack_approval.rs`, `cli_journey.rs` (Custody, Cli), `ignored.rs`, `rust-gate.yml`, `main-failing-ci-journeys`. I found no log or note with the lane's time today. I did not read the code behind the Design's `file:line` citations.
