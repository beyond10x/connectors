---
format: aep.planning-md/3
id: story:store-tests-end-within-60s-under-load
kind: story
status: draft
title: Five store tests end within 60 s at host load
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
- depends_on: story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process
scope:
- confidence: inferred
  path: crates/connectors-host/src/local/owner/governed/tests/bounded_realization_security.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/refresh/tests/issue.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/store_cost_tests.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/tests.rs
- confidence: inferred
  path: crates/connectors-host/tests/metadata_reopen_adversary.rs
revision: 12
---
## Defect

Five tests of `connectors-host` run longer than 60 s on this workstation at host load 68 to 85
(20 CPUs), so cargo-nextest kills them (`.config/nextest.toml`: `slow-timeout` 30 s,
`terminate-after` 2) and a run over their modules exits 100. The budget of
`~/beyond10x/AGENTS.md` ("Tests stay fast") is 60 s per test and 120 s per test step.

Measured 2026-11-23 by the implementor of
story:entity-runtime-0330-records-name-definition-by-digest, each pair run at the same time
straight from the two lib test binaries, without the 60 s kill, all exit 0:

| Test | Entity Runtime 0.32.0 (0.75.0) | Entity Runtime 0.33.0 (pin branch) |
|---|---|---|
| `store_cost_tests::a_reopen_after_its_own_write_refuses_every_blob_altered_since` | 114.67 s | 128.35 s |
| `registry::tests::a_read_invoke_against_a_grown_store_replays_it_at_most_once` | 122.62 s | 64.92 s |
| `registry::tests::a_reconnect_below_the_refresh_retention_bound_is_not_refused_capacity` | 81.66 s | 105.44 s |
| `registry::tests::concurrent_revoke_repair_publication_and_read_dispatch_share_one_fence` | 114.16 s | 130.18 s |

The fifth, `metadata_reopen_adversary a_reopen_refuses_every_altered_blob_a_full_replay_refuses`,
ran alone at the pin in 57.01 s and was killed at 60 s in both runs of the filterset; not
measured on the base.

The filterset of that story's acceptance line 8 (76 tests) took 228 s and 239 s, exit 100, with
these five killed. Slowest passing: `a_reconnect_at_the_retention_bound_is_refused_capacity`
36.2 s, `cold_owner_start a_cold_read_without_a_connection_waits_for_a_slow_owner_start` 31.6 s.

Not known: their times on an idle machine and on the forge's runner; the forge's repository
gate passed with them on 0.75.0 (pull request 283).

Two tests of the owner modules are over the budget too. Measured 2026-11-23, each run alone
from a lib test binary of b3eab7f8575 (Entity Runtime 0.33.0, before
story:connections-list-answers-through-the-owner), without the 60 s kill, both passed:

| Test | Time | Host load (1 min) |
|---|---|---|
| `local::owner::governed::tests::bounded_realization_security::bounded_owner_serves_exactly_the_realizations_the_projection_serves` | 181.90 s | 39 to 61 |
| `local::owner::refresh::tests::issue::adversary_concurrent_issues_within_the_lead_send_one_exchange` | 544.69 s | 61 to 128 |

Under nextest both are killed at 60 s, so `test(owner::)` exits 100
(`177 tests run: 175 passed (1 slow), 2 timed out`, 130.4 s, on the branch of that story);
its acceptance 11 leaves the two out by name. Not known: their times on an idle machine.

More tests killed at 60 s on 2026-11-23, on the branch of
story:connections-list-answers-through-the-owner rebased onto 2e5ddabe9d8 (0.76.0), run
`binary(cold_owner_start) | test(owner_observe) | test(owner::)` without the two tests above:
`177 tests run: 169 passed (9 slow), 8 timed out`, 403.8 s, host load 78 to 100. Not timed
alone, except where a time is given:

- `local::owner::mutation::tests::final_audit_recovery_preserves_every_live_business_result`
  (killed again in a run of 8 tests at load 39 to 81).
- `local::owner::governed::tests::bounded_realization_security::worker_admission_refuses_an_implemented_realization_the_projection_would_not_serve`
  (killed again in that run).
- `local::owner::governed::tests::bounded_reads::bounded_owner_refuses_an_implemented_resource_read_without_inline_schemas`.
- Five tests of the binary `cold_owner_start`: `a_cold_read_verifies_the_store_once`,
  `a_cold_read_without_a_connection_verifies_the_store_once`,
  `a_cold_read_without_a_connection_waits_for_a_slow_owner_start`,
  `adversary_pages_a_revoked_connection_and_lost_custody_answer_alike_beside_the_owner_and_without`
  and `connections_commands_answer_through_a_running_owner_without_verifying_the_store`. The
  last one passes alone in 9.4 s at load 81. In the run of 8 it failed after 32.1 s at its
  assertion that a command beside a running owner opens no store: `connections list` read the
  store itself once, which is what the command does when the owner's answer does not arrive
  (story:observe-fallback-waits-behind-the-owners-open).
- On the branch of story:settled-read-uses-leave-the-store:
  `adversary2_a_same_build_owner_at_capacity_is_never_another_build_over_100_runs`, killed at
  60 s, no time on its base.
- On the branch of the standing-grant stories: two tests of the `http_standing_grant*`
  binaries run 31 to 49 s, and one of them was killed at 60.3 s once at load 60 to 93.

- 2026-11-23, three more at the bound. `connectors::cold_owner_start
  connections_commands_answer_through_a_running_owner_without_verifying_the_store` alone, three
  runs: 9.366 s, 45.635 s, 58.388 s (the last at load 81). The ignored release measurement
  `a_cold_describe_and_read_of_a_large_store_answer_within_60_seconds` (133 MB store) was
  killed by the runner at 60.496 s at load 74 (pool job 58148), so the branch
  `connections-through-owner` has no number for it. On the branch `read-uses-in-memory`,
  `local::registry::tests::a_read_invoke_against_a_grown_store_replays_it_at_most_once` was
  killed at 60.395 s in a run of 21 tests (pool job 58108).

- 2026-11-23, later the same day, four more over the bound and one near it, at load 65 to 115.
  On the branch `read-uses-in-memory`,
  `local::registry::expiry_batch_tests::due_expiries_past_the_former_32_member_bound_join_the_callers_batch`
  was killed at 60 s in three runs of 18 tests; run without the kill at load 74 to 78 it took
  76.06 s on the unchanged `er.rs` and 80.35 s on the changed one, both exit 0. On the branch
  `er-0331-pin`, each run alone from the built test binary (not through nextest, so nothing
  killed them): `tests::a_read_invoke_against_a_grown_store_replays_it_at_most_once` 134.78 s,
  `tests::a_reconnect_below_the_refresh_retention_bound_is_not_refused_capacity` 100.80 s,
  `tests::concurrent_revoke_repair_publication_and_read_dispatch_share_one_fence` 149.11 s,
  and `store_cost_tests::a_reopen_after_its_own_write_refuses_every_blob_altered_since`
  58.19 s.

Earlier stories of the same class, both implemented: story:owner-path-tests-hold-under-load,
story:grown-store-replay-test-flake.

## Acceptance

- Each of the five store tests and the two owner tests is timed alone on an idle machine and
  at host load of at least 60, on `main`. The two times of a test are recorded with one
  evidence record on this story, in the form
  `aep plan artifact evidence story:store-tests-end-within-60s-under-load --kind test_result --source '<the command of the run, then the two times>' --ref <the log of the run>`.
  The verb has no other field for free text, so the times stand in `--source` and in the log.
  The run prints `uptime` before and after the test, so the host load stands beside each time in
  the log the record names. Those `uptime` lines show whether the load of 60 was reached; a run
  below it is recorded with its load and does not meet this statement.
- Each ends within 60 s at that load: by a smaller store or fewer iterations that still shows
  what the test shows (stated per test), not by a longer timeout. The other route this
  statement named, a saved store built once per run under `target/`, is measured and closed
  for these tests (the last paragraph of this story).
- The filterset of story:entity-runtime-0330-records-name-definition-by-digest, acceptance
  line 8, exits 0 within 120 s at that load.
- `cargo nextest run -p connectors-host -E 'test(owner::)'` exits 0 within 120 s at that load,
  and acceptance 11 of story:connections-list-answers-through-the-owner no longer names an
  exception.

## Out of scope

What each test asserts.

## Order with the test-step stories

The stories that decompose story:the-gates-test-step-ends-within-120-s-on-the-forge own the change
for the tests they name
(story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process for
eight tests named here and the two of the `http_standing_grant` binaries; until 2026-11-25 that
was story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run, which was measured and
is not built,
story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate for the stress test,
story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait for one test);
this story keeps its acceptance, 60 s at host load, and is measured after the registry story
lands, which its `depends_on` edge to that story records.

The registry story does not promise 60 s at load: it removes 30.6 % of the eleven tests' time on
the build host (7.7 % in `a_read_invoke…`) and its acceptance is a sum of forge durations. When a
test named here is still over 60 s at host load after it, this story keeps the change for that
test. The route "a shared fixture" is measured and closed for it: a template store saves 33 to
38 ms per store (the section "Measured (2026-11-25): not built as written" of the archived
story).
