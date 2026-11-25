---
format: aep.planning-md/3
id: story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts
kind: story
status: draft
title: 'Five tests no longer wait out the HTTP and SQL client timeouts: 56.2 s become an estimated 5.4 s'
summary: The HTTP client's 15 s and 5 s timeouts and the SQL QUERY_TIMEOUT of 10 s are literals and constants; two debug-only HTTP overrides and a constructor parameter for SQL let tests shorten them
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- depends_on: story:the-lane-slow-runs-the-ignored-timing-family
- depends_on: story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build
scope:
- confidence: cited
  path: adapters/sql/src/lib.rs
- confidence: cited
  path: adapters/sql/src/mysql.rs
- confidence: cited
  path: adapters/sql/tests/mysql_protocol.rs
- confidence: cited
  path: adapters/sql/tests/protocol.rs
- confidence: cited
  path: crates/connectors-host/src/http.rs
- confidence: cited
  path: crates/connectors-host/tests/http_budget.rs
- confidence: cited
  path: crates/connectors-host/tests/provider_timeout_adversary.rs
revision: 6
---
## Defect

Five tests live through a client timeout of production: the HTTP client's 15 s request timeout
and 5 s connect timeout (group H, 36.0 s) and the SQL query timeout of 10 s (group I, 20.2 s).
Together they take 56.2 s of the test step.

Source of the durations: the ones nextest printed in the forge's gate job 138270416028 of pull
request 291, 2026-11-24, as listed in test-plan:where-the-gates-test-step-spends-its-time (groups H
and I). The plan read the sources at 0.77.2. The function names and lines below were read in the
tree of 0.78.1. The sums are the plan's and equal the sums of its rows.

Group H:

- 16.0 s, binary `connectors-host::http_budget`, `crates/connectors-host/tests/http_budget.rs`
  (line 181), `provider_budget_generic_30_seconds_is_not_clipped_to_legacy_15`: the fixture answers
  after 16 s to prove that the 15 s client timeout does not clip a 30 s budget.
- 15.0 s, binary `connectors-host::provider_timeout_adversary`,
  `crates/connectors-host/tests/provider_timeout_adversary.rs`,
  `a_body_that_stalls_after_the_provider_answered_is_the_providers_timeout`: the server stalls the
  body; the test ends at the 15 s client timeout.
- 5.0 s, same binary and file,
  `a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked`: the server never
  answers the ClientHello; the test ends at the 5 s connect timeout.

Group I:

- 10.1 s, binary `connectors-sql::mysql_protocol`, `adapters/sql/tests/mysql_protocol.rs`,
  `mysql_deadline_kills_the_running_query`: the query hangs until `QUERY_TIMEOUT` (10 s).
- 10.1 s, binary `connectors-sql::protocol`, `adapters/sql/tests/protocol.rs`,
  `deadline_sends_the_backend_cancel_key_and_drains_the_connection`: the query hangs until
  `QUERY_TIMEOUT` (10 s).

Read at 0.78.1, the 15 s in `crates/connectors-host/src/http.rs` are two things:

- line 239: `.timeout(Duration::from_secs(15))` on the client builder, beside
  `.connect_timeout(Duration::from_secs(5))` at line 238. This is the "legacy 15" the three tests
  wait on. `with_provider_budget` does not reach it (its comment: "Existing constructors retain
  the legacy per-request timeout").
- line 79: `READ_POST_DEFAULT_TIMEOUT`, 15 s, the default deadline of a read sent as a POST (used
  at line 255). No test of the five waits on it. It is out of this story.

## Change

HTTP, two overrides through the helper of
story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build:

1. `CONNECTORS_TEST_OVERRIDE_HTTP_REQUEST_MS` replaces the literal at `http.rs` line 239
   (production 15,000 ms). `CONNECTORS_TEST_OVERRIDE_HTTP_CONNECT_MS` replaces the literal at line
   238 (production 5,000 ms).
2. The three tests are integration tests and build `ScopedHttp` in their own process. Each runs
   its body in a child process of its test binary with the variables set (the pattern of
   `policy_child` in `crates/connectors-host/src/local/approval_policy/tests.rs`). No other test
   of the binary sees the variables under plain `cargo test`, and no test calls
   `std::env::set_var`.
3. Values (this story's choice; the plan gives 1 s per test and no values): request timeout
   500 ms, connect timeout 300 ms. The fixture of
   `provider_budget_generic_30_seconds_is_not_clipped_to_legacy_15` answers after 700 ms in place
   of 16 s: 200 ms past the request timeout, under the 30 s budget, which stays.

SQL, no environment variable:

4. `Sql` (`adapters/sql/src/lib.rs` line 191) gets a field for the query timeout with the default
   `QUERY_TIMEOUT` (line 23) and a builder method `with_query_timeout`, hidden from the
   documentation, after the precedent `with_read_profiles` (line 259). The two uses of
   `QUERY_TIMEOUT` (`lib.rs` line 394, `adapters/sql/src/mysql.rs` line 415) read the field.
5. The helpers `stalled` of the two test files (`protocol.rs` line 515, `mysql_protocol.rs` line
   556) build the adapter with `with_query_timeout` of 1,000 ms.

Outside the gate three tests keep the production values, as ignored tests that share their body
with the gate's test through one helper:

- `slow_lane_a_body_that_stalls_after_the_provider_answered_is_the_providers_timeout`
  (`provider_timeout_adversary.rs`): the 15 s request timeout, no variable set.
- `slow_lane_a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked` (same file):
  the 5 s connect timeout, no variable set. The plan names no run at 5 s; this story adds it.
- `slow_lane_deadline_sends_the_backend_cancel_key_and_drains_the_connection` (`protocol.rs`):
  `QUERY_TIMEOUT` of 10 s, no `with_query_timeout`.

The rule of story:the-lane-slow-runs-the-ignored-timing-family puts the three in the timing family
by their names. This story edits neither `ignored.rs` nor `slow.yml`.

Marked inferred in the plan: that a short timeout gives the same Timeout and `upstream_answer`
result as the 15 s clip.

Estimate after (the plan; not timed): group H 1 s each, 3.0 s. Group I 1.2 s each, 2.4 s. Sum
5.4 s.

## Acceptance

1. The five tests pass, and their durations in the test step of the forge's "repository gate" job
   of this story's pull request sum to at most 9 s. Before: 56.2 s. Estimate: 5.4 s. Bound: 1.5 ×
   5.4 = 8.1, rounded up to 9 s. The numbers are read from the durations nextest prints in that
   job.
2. The three lane tests pass in the lane `slow`. The lane's command
   `cargo run --locked -p connectors-build -- ignored --family timing --required --only-prerequisite provider`
   lists each as `passed`. Alone they run with
   `cargo test --locked -p connectors-host --test provider_timeout_adversary -- --ignored --exact <name>`
   and
   `cargo test --locked -p connectors-sql --test protocol -- --ignored --exact slow_lane_deadline_sends_the_backend_cancel_key_and_drains_the_connection`.
3. The production 5 s connect timeout is covered by
   `connectors-host::provider_timeout_adversary::slow_lane_a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked`:
   it asserts that the refusal comes no sooner than 5 s after the connect started.
4. `with_query_timeout` has no caller outside `adapters/sql/tests/`: `grep` over `adapters/sql/src`
   and `apps/` finds only its definition. The MySQL path at the production 10 s has no lane test;
   `mysql_deadline_kills_the_running_query` runs at 1,000 ms only.
5. The gate's ignored step still passes: the three new ignored tests are not unknown to it.

That a release build ignores the two HTTP overrides is held once, for the helper, by
story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build.

## Scope

- `crates/connectors-host/src/http.rs`: the client builder of `ScopedHttp` (lines 238 and 239).
- `crates/connectors-host/tests/http_budget.rs`: the one test above.
- `crates/connectors-host/tests/provider_timeout_adversary.rs`: the two tests above and two lane
  tests.
- `adapters/sql/src/lib.rs`: `Sql`, `with_query_timeout`, the use of `QUERY_TIMEOUT`.
- `adapters/sql/src/mysql.rs`: the use of `QUERY_TIMEOUT`.
- `adapters/sql/tests/protocol.rs`: `stalled`, the one test above and one lane test.
- `adapters/sql/tests/mysql_protocol.rs`: `stalled` and the one test above.

No other story of this set writes these seven files.

## Not in this story

- `READ_POST_DEFAULT_TIMEOUT` (15 s, `http.rs` line 79).
- The lane `slow` itself: story:the-lane-slow-runs-the-ignored-timing-family.
- The tests of every other group of the test plan.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not established: every "after" figure on the forge; whether the three host tests can run their
  body in a child process as said in point 2 (their fixtures were read at the lines named only);
  whether a hidden public method on `Sql` is acceptable, the other form being a field of the
  adapter's `Config`.
