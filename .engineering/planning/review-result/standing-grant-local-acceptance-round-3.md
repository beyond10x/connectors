---
format: aep.planning-md/3
id: review-result:standing-grant-local-acceptance-round-3
kind: review-result
status: active
title: Acceptance critic, round 3, story:local-invoke-admits-a-keyed-write-under-a-standing-grant and story:standing-grant-closes-the-input-to-listed-members
relations:
- reviews: story:local-invoke-admits-a-keyed-write-under-a-standing-grant
- reviews: story:standing-grant-closes-the-input-to-listed-members
revision: 1
---
verdict: needs-revision

Round-2 findings, checked against revision 3:
- Closed: findings 1, 2, 4, 6, 7, 8, 9 and 10 to 12 (the closed-input story's scalar root, line 4, and `git diff -U0`).
  - Finding 1 closes through the "does not prove" paragraph at lines 521-530, which round 2 offered as an alternative.
  - Finding 4 closes as drafted: the controls are added and the failed-prepare and failed-bind cases are declared unproven.
- Partly closed: finding 3 (seam named, but see findings 2 and 3 below) and finding 5 (helpers copied, `requests()` and `attempts()` named, but the table name `mutation_attempts` is not given; see `metadata/mutations.sql:6`).
- Your unverified items:
  - `test(standing)` does match the module path. `local::standing::tests` is an inline module (`standing.rs:595`).
  - The clock offset is feasible. The `clock()` closure at `slack_approval.rs:200` can capture an `Arc<AtomicU64>`.
  - The 3600 s jump is not refused between CLI calls. `BoundedClock` is built per acquisition (`clock.rs:424-436`), the key window runs to `MAX_TIME` (`approval_keys.rs:502`), and the fixture midpoint sits at 21:41 UTC (`server.rs:235`), so the jump does not cross midnight.
  - A changed proof byte gives `approval_refused`: `from_document`, `decode` and `strict` all return `Refused`, which maps to `ApprovalRefused` at `execution.rs:62`, inside the `initial` closure, so `not_attempted`.
  - An `auth.test` error is `not_attempted`: the preflight runs inside `initial` (`execution.rs:277-285`). An HTTP 500 on `chat.postMessage` comes after `open_dispatch`, so it has an attempt.
  - The attempt table is `mutation_attempts` in `metadata.sqlite3`, and `rusqlite` is a dependency of the adapters crate.
  - Line 23's artifact check is roughly right: the shape is `.result.outcomes[]` with `name`, `status` and `selected`. The names carry the prefix `connectors-catalog-provider::local_runtime::`.

```findings
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 402
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Line 7 says 'with the clock 3600 s later it is admitted', but `Uses::take` counts every use with `admitted_at_ms >= now - WINDOW_MS` (crates/connectors-host/src/local/standing.rs:498), so exactly 3600 s later the first use still counts and the answer is `AtCap`; the existing test needs `WINDOW_MS + 1` (standing.rs:738). Say '3600 s plus 1 ms later' or change the comparison and the existing test."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 349
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'`approvals::tests::signer` becomes `pub(crate)`' does not make it reachable from `local::standing::tests`: `mod tests;` in crates/connectors-host/src/local/approvals.rs:11 is private, so the module needs `pub(crate)` under `cfg(test)` as well, and `PUBLIC` for the `ConfiguredApprovalKey` (tests.rs:17, only inline in `policy()`) is private too. Line 539-541 then says the one line of tests.rs; it is a line in approvals.rs too. State the seam as the two lines, or repeat the RFC 8032 constants in `local::standing::tests`."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 291
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Design step 2 puts `resolve` again and `unchanged` inside `standing::admit`, while line 345-348 says `admit` is a function of plain arguments with no keyring, owner or child; `resolve` needs the descriptor snapshot, connection, policy and key store (execution.rs:156), so tests 1, 5 and 9 cannot call `admit` as written. Say that `resolve`/`unchanged` stay in `execute` and `admit` takes the resolved instance, operation, input `Value`, key and clock reading."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 440
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "O2 (line 609) cites acceptance 17 for 'no member: today's bytes', but line 17/437-439 sets the member and omits the key; no line checks `--idempotency-key k` with no `standing_grants` member and no `--approval-file` against the gate bytes at invoke_short.rs:319-342, so a CLI gate that lets every keyed write through (cli/invoke.rs:1155) passes lines 16-18. Add that case to line 17."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 451
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Line 20 names no command for 'the gate's line `gate: executed N authored scenarios, P passed`'. It is printed by `authored_scenarios` (crates/connectors-build/src/gate.rs:632), run by the test `the_registered_authored_scenarios_execute_against_the_model` (gate.rs:716-724), which also runs `ess verify conform synthesize`, the step AGENTS.md names as growing with the model. Name that test with `--nocapture`, run at the merge base and after, and give both wall times against the 60 s bound."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 505
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Line 23 branches on line 21's workstation result, so it has no branch for a sibling that passes on the workstation and fails in the `ignored CLI journeys` job (the sample artifact shows it failing there with 'test reported a failure'; `.github/workflows/rust-gate.yml:160-186`). Add the case (for example both stay `Deferred` and the report names the CI log) and name the check as `.result.outcomes[]` entries whose `name` ends in each test name with `.status == \"passed\"`."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 423
  category: acceptance
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "Line 14 puts test 14 in `local::config::tests`, but crates/connectors-host/src/local/config.rs has no `mod tests` (its test modules are `consumer_tests`, `custody_tests`, `connector_instance_tests`, `derived_instance_tests`, from line 2408). The `test(standing)` filter still finds it by name; say the module is new or name an existing one. Line 461 should also name the table `mutation_attempts`."
- file: .engineering/planning/story/standing-grant-closes-the-input-to-listed-members.md
  line: 131
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The depth case in line 1 ('an object nested 65 levels under `/body/extra`') is outside `input_keys`, so the first leaf under `/body/extra` is `OutsideInput` whether or not the walk stops at 64 levels; it does not exercise the Design's bound (line 107-108). Put it under a listed pointer: `input_keys` `[\"/body/blocks\"]` with `/body/blocks` holding an object nested 65 levels is `OutsideInput` and one nested a few levels is admitted (line 2)."
```

What I read: the two stories in full, the round-2 result, the critic procedure and rubric, and the code cited above. I ran no build or test. I did not run `aep plan artifact kinds` or `lifecycle`.

What I could not establish:
- Whether `test(standing)` plus the existing 12 matches makes `nextest list` print exactly the 15 names. It prints them among others, and "prints the 15 names" is read as inclusion.
- The classification an HTTP 500 on `chat.postMessage` gets (`refused` or `unknown`). Step i's "other than `not_attempted`" holds either way, and the bound grant id is the stronger observer.
- Whether the journey stays under 60 s on the workstation. Line 24 measures it.

Out of my lane: the unresolved `admit`/`resolve` split (finding 3) is a design question for `plan-critic-design`. The ESS-step timing is a test-budget rule from `AGENTS.md`.
