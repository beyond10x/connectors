---
format: aep.planning-md/3
id: story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts
kind: story
status: implemented
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
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T11:44:03Z", actor: "human:operator", revision: 9}
- {from: "proposed", to: "active", at: "2026-11-25T11:44:03Z", actor: "human:operator", revision: 10}
- {from: "active", to: "implemented", at: "2026-11-26T20:08:51Z", actor: "human:operator", revision: 13, decided_on: {"recorded":{"test_result":2}}}
---
## Defect

Five tests live through a client timeout of production: the HTTP client's 15 s request timeout
and 5 s connect timeout (group H, 36.0 s) and the SQL query timeout of 10 s (group I, 20.2 s).
Together they take 56.2 s of the test step.

Source of the durations: the ones nextest printed in the forge's gate job 138270416028 of pull
request 291, 2026-11-24, as listed in test-plan:where-the-gates-test-step-spends-its-time (groups H
and I). The plan read the sources at 0.77.2. The function names and lines below were read in the
tree of 0.78.1 and are the same at 0.78.2. The sums are the plan's and equal the sums of its rows.

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
   `a_body_that_stalls_after_the_provider_answered_is_the_providers_timeout` asserts today that
   at least 10 s passed (`provider_timeout_adversary.rs` lines 57 to 61). The shared body takes
   that lower bound from its caller: the gate's test passes the request timeout it set, the lane
   test keeps 10 s.

SQL, no environment variable:

4. `Sql` (`adapters/sql/src/lib.rs` line 191) gets a field for the query timeout with the default
   `QUERY_TIMEOUT` (line 23) and a builder method `with_query_timeout`, hidden from the
   documentation, after the precedent `with_read_profiles` (line 259). The two uses of
   `QUERY_TIMEOUT` (`lib.rs` line 394, `adapters/sql/src/mysql.rs` line 415) read the field.
5. The helpers `stalled` of the two test files (`protocol.rs` line 515, `mysql_protocol.rs` line
   556) build the adapter with `with_query_timeout` of 1,000 ms.

What each wait protects, and why the shorter test value is safe:

- The 15 s request timeout (`http.rs` line 239) is the per-request cutoff of a client built with
  no provider budget: it ends a request whose provider stops answering, a body that stalls for
  example, so the caller gets `timeout` and does not wait without end. The two tests that wait on
  it assert what kind of timeout that is (`Timeout` with `upstream_answer` set once the request
  was sent) and that a provider budget of 30 s is not cut by it. Both are properties of where the
  cutoff fell and of which cutoff applies, not of its length, so 500 ms proves them as 15 s
  does. The length is proved by the lane test at 15 s.
- The 5 s connect timeout (`http.rs` line 238) bounds the opening of a connection, the TLS
  handshake included, to a provider that does not answer. The test that waits on it asserts that
  a connection that never opened sent no request and is not marked as the provider's answer.
  That is a property of the stage at which the cutoff fell, so 300 ms proves it as 5 s does. The
  length is proved by the lane test at 5 s.
- `QUERY_TIMEOUT` of 10 s (`adapters/sql/src/lib.rs` line 23) is the time one query may run
  inside the 15 s request bound, with `CLEANUP_TIMEOUT` reserved for the cancel (the comment at
  `lib.rs` lines 392 and 393). The two tests assert what happens at that cutoff: the adapter
  cancels the running query on the server and drains the connection. The same code runs at
  1,000 ms: the work deadline is the smaller of the query timeout and the request bound less the
  cleanup time, and 1,000 ms is the smaller as 10 s is today. The length is proved by the lane
  test at 10 s for PostgreSQL.

A release build ignores the two HTTP overrides, and `with_query_timeout` has no caller in the
product (acceptance 4).

Outside the gate three tests keep the production values, as ignored tests that share their body
with the gate's test through one helper:

- `slow_lane_a_body_that_stalls_after_the_provider_answered_is_the_providers_timeout`
  (`provider_timeout_adversary.rs`): the 15 s request timeout, no variable set.
- `slow_lane_a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked` (same file):
  the 5 s connect timeout, no variable set.
- `slow_lane_deadline_sends_the_backend_cancel_key_and_drains_the_connection` (`protocol.rs`):
  `QUERY_TIMEOUT` of 10 s, no `with_query_timeout`.

A decision beyond the plan: the lane test at the 5 s connect timeout. The plan names runs at 15 s
and at 10 s and none at 5 s. This story adds the run at 5 s so that each production value a gate
test no longer waits out is proved in one run somewhere. The parent story lists it among the
decisions beyond the plan.

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
   `cargo run --locked --bin connectors-build -- ignored --family timing --required --only-prerequisite provider`
   lists each as `passed`. Alone they run with these three commands:
   - `cargo test --locked -p connectors-host --test provider_timeout_adversary -- --ignored --exact slow_lane_a_body_that_stalls_after_the_provider_answered_is_the_providers_timeout`
   - `cargo test --locked -p connectors-host --test provider_timeout_adversary -- --ignored --exact slow_lane_a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked`
   - `cargo test --locked -p connectors-sql --test protocol -- --ignored --exact slow_lane_deadline_sends_the_backend_cancel_key_and_drains_the_connection`
3. The production 5 s connect timeout is covered by
   `connectors-host::provider_timeout_adversary::slow_lane_a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked`:
   it asserts that the refusal comes no sooner than 5 s after the connect started. It runs with
   the second command of acceptance 2.
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
  adapter's `Config`; whether the TLS handshake is counted under the client's connect timeout
  (inferred from the 5.0 s of
  `a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked`, which asserts no time
  today).

## As built

Built on 2026-11-25 on the head of
story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build's
pull request. Seven files under `crates/` and `adapters/`: `crates/connectors-host/src/http.rs`,
`adapters/sql/src/lib.rs`, `adapters/sql/src/mysql.rs` and the four test files of the Scope
section.

Where the code differs from the Change section:

- Point 2, one variable per test. Each gate test's child gets only the override its case waits
  for: `CONNECTORS_TEST_OVERRIDE_HTTP_REQUEST_MS` for the stalled body and for the budget test,
  `CONNECTORS_TEST_OVERRIDE_HTTP_CONNECT_MS` for the handshake. With both set, 200 ms lay between
  the connect cutoff and the request cutoff, and a request cutoff that came first failed the
  handshake test.
- Point 2, the child runs through the test binary itself (`current_exe`, `--exact <name>`,
  `--nocapture`) with a marker variable, and the parent holds that the child printed
  `test result: ok. 1 passed`. A child that runs no test, or is killed, fails the parent.
- The three HTTP gate tests are ignored in a build without `debug_assertions`, as the four older
  override tests are.
- Point 3, the lane test of the stalled body asserts at least 15 s, not 10 s. With 10 s a request
  timeout of 10 s to under 40 s passed, and this story says the lane test proves the length.
- The handshake fixture is reached by its address, not by the name `localhost`, and both
  handshake tests assert that the peer read at least one byte before the refusal. A name with
  two address families can spend a connect timeout of 300 ms before a handshake starts; the test
  then passed with no handshake.
- Point 4, a function `work_deadline` in `adapters/sql/src/lib.rs` holds the one expression both
  paths used. It ends at the request bound less the cleanup time when the query timeout is too
  large to add to the time now. Before, `with_query_timeout(Duration::MAX)` panicked on that sum.
  No product code passes such a value.

Shown red on the build host, one run each, with the tests on their short values and the
product still on its literals:

- the budget test: failed with "the legacy timeout did not cut a client with no budget";
- the stalled body: "refused after 15.000583146s, outside 500ms..10s";
- the handshake: "refused after 5.000945064s, outside 300ms..5s";
- the PostgreSQL and the MySQL deadline tests: no refusal within their 6 s bound
  (6.009 s and 6.011 s);
- with the product change, the stalled body with the request variable not given to the
  child: "refused after 15.001702863s, outside 500ms..10s".

Shown red after the independent read, one run each, each taken back or fixed:

- `the_largest_query_timeout_is_capped_by_the_request_bound` (`protocol.rs`, added by the read):
  failed with "overflow when adding duration to instant" before `work_deadline`, passes with it;
- the handshake test against a fixture that accepts nothing for 40 s: failed with "the peer
  read no byte before the refusal after 302.649612ms".

Green on the build host, final tree, one run of the four test binaries
(`provider_timeout_adversary`, `http_budget`, `protocol`, `mysql_protocol`): 53 run, 53 passed,
3 skipped (the lane tests). The five tests of this story: handshake 0.313 s, stalled body
0.514 s, budget 1.215 s, PostgreSQL deadline 1.105 s, MySQL deadline 1.296 s, 4.443 s summed
(arithmetic). None is over 5 s.

The lane tests, single runs on the build host: stalled body 15.008 s with the bound of 15 s,
handshake 5.009 s, PostgreSQL deadline 10.10 s (run before the read; its code did not change
after it).

Lints and guards, exit status 0 each: `cargo fmt --check` for the two crates,
`cargo clippy --locked -p connectors-host -p connectors-sql --all-targets -- -D warnings`, the
same lint for `x86_64-pc-windows-gnu` with `-p connectors-host -p connectors`, and
`cargo nextest run --locked -p connectors-build --test main debug_override_reads` (3 passed).

The independent read found no path by which a release build reads the two variables, no test of
another binary that inherits them, and no caller of `with_query_timeout` outside
`adapters/sql/tests/`. A query timeout of 3,600 s was refused after 13.07 s, the request bound
less the cleanup time. What an override value does in a debug build, by one probe run that
waited 2.5 s: 0 cuts at once; a value that does not parse, and the largest 64-bit value, gave no
cutoff within the 2.5 s.

One finding of the read is outside this story and has a draft of its own,
story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout.

Read from the pull request's jobs and not from a run on the build host: acceptance 1 (the five
durations in the gate job, at most 9 s), acceptance 2 and 5 (the lane `slow` lists the three
lane tests as passed; the gate's ignored step passes), and the three HTTP gate tests on Windows.

Not run: a build without `debug_assertions`; the MySQL path at `Duration::MAX` (it calls the
same `work_deadline`).

Read from the forge on 2026-11-26, pull request 313, whose branch merged this story's commit
15be8ec95 with three other stories. The diff of 15be8ec95 against its first parent names the
seven files of the scope and no other file outside the planning store.

- Acceptance 1: "repository gate" job 139714078337 (run 45748988866, attempt 2; attempt 1, job
  139671926182, was cancelled after 90 minutes in `apt-get update`, before the gate started).
  The test step printed `Summary [ 466.033s] 4546 tests run: 4546 passed (4 slow), 229
  skipped`. The five tests, as nextest printed them:
  `provider_budget_generic_30_seconds_is_not_clipped_to_legacy_15` 1.213 s,
  `a_body_that_stalls_after_the_provider_answered_is_the_providers_timeout` 0.513 s,
  `a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked` 0.312 s,
  `mysql_deadline_kills_the_running_query` 1.011 s,
  `deadline_sends_the_backend_cancel_key_and_drains_the_connection` 1.088 s. Sum 4.137 s
  (arithmetic), under the bound of 9 s by 4.863 s. Before: 56.2 s.
- Acceptance 2 and 3: "slow" job 139671925405 (run 45748988824) of head 606a611d5, step "Run
  the ignored slow_lane_ tests, 60 s each" (`ignored --family timing --required
  --only-prerequisite provider`): `executed` 6, `failed` 0, `missing` 0, `unknown` 0; the three
  lane tests have `"status": "passed"`. Time between the job's `ignored: running` lines: the
  stalled body 15.014 s, the handshake 5.025 s, the PostgreSQL deadline 10.091 s (to `ignored:
  step ran 92.5 s: ignored tests`).
- Acceptance 4: `git grep -n with_query_timeout 606a611d5 -- adapters/sql/src apps/` prints one
  line, the definition at `adapters/sql/src/lib.rs:263`.
- Acceptance 5: the gate job's ignored step: `executed` 4, `failed` 0, `missing` 0, `unknown` 0,
  `inventory` 229.
- Windows: "host and CLI on windows-latest" job 139676682004 (run 45748988645), step "Test the
  host and the CLI" ended `success`; the three HTTP gate tests print `... ok`, the two lane tests
  of `provider_timeout_adversary` `ignored, lane slow: …`.

Every acceptance holds; the story moved to implemented on this record.
