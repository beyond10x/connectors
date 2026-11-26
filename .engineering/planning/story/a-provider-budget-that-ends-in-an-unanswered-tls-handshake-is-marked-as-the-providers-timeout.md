---
format: aep.planning-md/3
id: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
kind: story
status: active
title: A provider budget that ends in an unanswered TLS handshake is marked as the provider's timeout
summary: With a provider budget shorter than the connect timeout, a timeout during an unanswered TLS handshake carries upstream_answer = true although no request was sent; with at least 500 ms of the budget left, a connector layer bounds the connect by the budget so that the cut is reported as a connect error
relations:
- informed_by: story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts
- depends_on: story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/connectors-host/Cargo.toml
- confidence: cited
  path: crates/connectors-host/src/http.rs
- confidence: cited
  path: crates/connectors-host/tests/provider_timeout_adversary.rs
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T14:38:58Z", actor: "human:operator", revision: 13, decided_on: {"recorded":{"review_outcome":19}}}
- {from: "proposed", to: "active", at: "2026-11-25T16:33:14Z", actor: "human:operator", revision: 14, decided_on: {"recorded":{"review_outcome":19}}}
---
## Defect

Found on 2026-11-25 by the independent read of
story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts. It is not caused by that
story: the function below has the same text on its base commit.

`contracts/cli/v1alpha1/semantics.md` §6 says of the provider's timeout: "A connection
that never opened sent nothing and is not one of them." `provider_error` in
`crates/connectors-host/src/http.rs` holds that for the client's connect timeout:
a timeout that reqwest reports as a connect error is not marked, every other timeout is marked
as the upstream's answer (`Error::answered`).

A client with a provider budget (`ScopedHttp::with_provider_budget`) sets the time that is
left of the budget as the timeout of each request. When that time ends while the TLS handshake
is still unanswered, the timeout is marked as the upstream's answer, although no HTTP request
was sent.

One run on the build host, on the tree of the five-tests story, with production's two client
timeouts (15 s and 5 s): a peer on a loopback listener reads the first bytes of the connection
and never answers; the client carries a provider budget of 300 ms. The call ended after 301 ms
with `Timeout` and `upstream_answer = true`; the peer had read 251 bytes. Without a provider
budget the same peer is cut by the connect timeout and the timeout is not marked
(`a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked` in
`crates/connectors-host/tests/provider_timeout_adversary.rs`).

That run's budget of 300 ms is under the 500 ms below which this story changes nothing (item 3
of "Change"): the same run ends marked after this story too. The test of acceptance 1 runs with
1,000 ms for that reason.

Read in the source of reqwest 0.13.5: `PendingRequest::poll` polls the request's own timers
before the request and, when one has passed, returns a timeout that carries no connect error
(`src/async_impl/client.rs:3096-3113`). The run above agrees with it.

Three places set a timeout from the budget:

- `send`: `request.timeout(remaining)`, then `provider_error`;
- the read POST: `request.timeout(remaining.min(read_post_timeout))`, then `provider_error`;
- `upgrade`: `.timeout(until - now)` inside `tokio::time::timeout_at(until, …)`. When the outer
  timer wins, the result is `Upgraded::Lost` with an unmarked `Timeout`. When reqwest's timer
  wins, it is `Upgraded::Lost(provider_error(..))`, marked. `Upgraded::NotSent` is returned
  only for `error.is_connect()`.

Who reaches it: `adapters/catalog/src/local.rs` calls `with_provider_budget` with the
execution deadline at three places. A call whose budget has less than the connect
timeout of 5 s left when it connects, to a provider that accepts the connection and does not
answer the handshake, becomes `Failure::ProviderTimeout` (`Failure::from_provider` in
`crates/connectors-host/src/local/runtime.rs`) and reports `timeout` at `stage = dispatch`
with `next_action = retry_explicitly`. By the contract it is not the provider's timeout.

Not measured: the same run on the base commit; how often a provider call connects with less
than 5 s of its budget left.

## Change

Decided: the connect and the TLS handshake get a bound of their own from the budget, inside
the client's connector, so that reqwest reports a connect cut by the budget as a connect
error and the branch of `provider_error` that exists leaves it unmarked.

Read in the sources of reqwest 0.13.5 and hyper-util 0.1.21, which rule out two other forms:

- `connect_timeout` is set on the `ClientBuilder` alone, the client is built once and
  `with_provider_budget` clones it. A bound per budget through that setting would mean a
  client per budget, and a client of its own has a connection pool of its own, so no call
  with a budget would reuse a connection.
- reqwest does not hand a request's extensions to hyper (`execute_request` builds the hyper
  request from method, URI, version, headers and body), so hyper-util's `capture_connection`
  cannot tell `provider_error` whether a request got a connection.

The form:

1. The client is built with one `connector_layer`. The layer's service reads a deadline from
   a tokio task-local when its `call` runs, keeps it in the future it returns, and bounds the
   connect with it. When the deadline has passed at `call`, the inner service is not called
   and the future is ready with an `io::Error` of kind `TimedOut`; a connect that is cut
   later fails with the same error. Without a task-local value the layer changes nothing.
   reqwest's request and connection types are private to it, so the layer and its service
   are generic over the inner service and the request type, and the service maps its error
   to reqwest's boxed error. reqwest's own `connect_timeout` stays the outermost layer.
2. `send` reads the time the budget has left, as today. When at least 500 ms are left (twice
   the lead, a named constant), it runs `request.send()` inside the scope of that
   task-local, set to the end of the budget less a lead of 250 ms (a named constant). With
   less left, and without a budget, it sets nothing and the call is as today. The request's
   own timeout stays what it is today. The lead is there so that a connect the budget cuts
   ends by the connect's bound: reqwest polls the request's own timer before the request
   (`PendingRequest::poll`, `src/async_impl/client.rs:3096-3113`), so when both have passed
   at one poll the marked timeout wins, and the lead is the time the task may go unpolled
   without that.
3. A call that connects with less than 500 ms of its budget left is as today: a cut during
   the handshake is marked. Two tests that exist hold that band as it is, on a client that
   has no connection yet: `provider_budget_cannot_be_reset_by_another_request_or_derived_capability`
   (200 ms left, the first `get` answers 200) and
   `provider_budget_uses_the_earlier_execution_deadline_during_http` (150 ms left, one
   request sent, a marked `Timeout`), both in `crates/connectors-host/tests/http_budget.rs`.
   A lead taken from so short a time would cut their connects.
4. `provider_error` does not change. reqwest reports the layer's failure as a connect error
   (`is_connect()`: the error comes from the connector) that is a timeout (`is_timeout()`:
   an `io::Error` of kind `TimedOut` in its sources), and that branch is unmarked today.
5. For `send` with at least 500 ms left the case of the defect becomes an unmarked `Timeout`:
   `Failure::Timeout`, reported as `timeout` at `stage = admission` with
   `next_action = retry_explicitly`. That is what the client's own connect timeout reports
   today. `Failure::from_provider` does not change. `upstream_answer` is a field of the
   internal `Error`, not a wire field; `stage` is the field a caller sees change, in this one
   case.
6. A request that got its connection is as today: the request's timeout ends it, and the
   timeout is marked. A request sent on a pooled connection calls no connector and is as
   today. When hyper-util finds the pooled connection closed before the request was sent, it
   calls the connector again inside the same scope; that connect has the same bound, and no
   request was sent.
7. `upgrade` and the read POST are not changed. Neither sets the task-local, so their
   connects have the client's connect timeout alone. `upgrade` keeps its two timers and its
   choice between `Lost` and `NotSent`.
8. `tower-layer` and `tower-service` become direct dependencies of `connectors-host` at the
   versions `Cargo.lock` already holds (0.3.3 each). Both entries carry their version in
   `crates/connectors-host/Cargo.toml`, as `libc` does there; the root `Cargo.toml` does not
   change. `Cargo.lock` gains two dependency lines of `connectors-host` and no package.

Read in the source of hyper-util 0.1.21, not run: the connector is called inline while the
request's future is polled, on the caller's task (`src/client/legacy/client.rs:216-238`,
`416-422`, `498-519`), so the layer's `call` sees the task-local. When a pooled connection
wins over a connect that has started, that connect ends in a background task under the
deadline its future kept. Acceptance 1 decides it. If the layer does not see the value, the
test stays red and this story is re-planned before any other form is built.

A budget with at least 500 ms left at `send` that ends before the TCP connection opens is cut
by the same bound and is unmarked. With less left the call is as today (item 3).
No test builds that case: a TCP connect that hangs needs an address outside the host, and the
test step makes no network request.

## Acceptance

1. Red first: a new test
   `a_provider_budget_cut_during_an_unanswered_tls_handshake_is_not_marked` in
   `crates/connectors-host/tests/provider_timeout_adversary.rs`, with production's client
   timeouts and a provider budget of 1,000 ms, against a peer that reads the first bytes and
   never answers, asserts: `Timeout`, `upstream_answer = false`, the peer read at least one
   byte, the call ended within 2 s, and `Failure::from_provider` of the error is
   `Failure::Timeout`. Run before the change, on the commit this story branches from (main
   with the five-tests story), it fails with `upstream_answer = true`; "As built" holds that
   run's message. It passes with the change.
2. A stalled body stays the provider's: a new test
   `a_body_that_stalls_under_a_provider_budget_is_the_providers_timeout` in the same file, a
   peer that sends status and headers and then stalls, a provider budget of 1,000 ms, asserts
   `Timeout`, `upstream_answer = true` and `Failure::ProviderTimeout`. It passes before the
   change and after it.
3. A slow answer stays the provider's: a new test
   `a_request_that_is_sent_and_not_answered_under_a_provider_budget_is_the_providers_timeout`
   in the same file, a plain-HTTP peer that reads the request and sends nothing, a provider
   budget of 1,000 ms, asserts `Timeout` and `upstream_answer = true`. It passes before the
   change and after it. The budget of the three tests is 1,000 ms so that each connect has
   750 ms after the lead.
4. `a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked` and
   `a_body_that_stalls_after_the_provider_answered_is_the_providers_timeout` pass unchanged.
5. The upgrade path and the read POST are untouched: the pull request's diff changes no line
   of the function `upgrade` and no line of the read POST (the function `post_json`), and the test binaries
   `http_upgrade` and `upgrade_adversary` of `connectors-host` pass in the forge's
   "repository gate" job.
6. A budget with less than 500 ms left is as today: the pull request's diff changes no line
   of `crates/connectors-host/tests/http_budget.rs`, and its tests pass in the same job,
   among them `provider_budget_cannot_be_reset_by_another_request_or_derived_capability`
   and `provider_budget_uses_the_earlier_execution_deadline_during_http`.
7. No test added by this story takes over 5 s in the nextest output of the test step of the
   forge's "repository gate" job of the pull request; none is a `slow_lane_` test.
8. `git diff --stat` of the pull request names, outside the planning store, `http.rs`, the
   test file, `crates/connectors-host/Cargo.toml` and `Cargo.lock`, and `Cargo.lock` gains no
   `[[package]]`.

## Scope

- `crates/connectors-host/src/http.rs`: the layer, the task-local, the client's builder,
  `send`.
- `crates/connectors-host/tests/provider_timeout_adversary.rs`: the three new tests.
- `crates/connectors-host/Cargo.toml` and `Cargo.lock`: the two direct dependencies.

Other stories on these files:

- story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts writes `http.rs`
  (the client's builder among its lines) and the test file. This story depends on it and
  starts after it is merged.
- `Cargo.lock` is a file every story with a new dependency writes. No story that is planned
  beside this one adds a dependency. story:mcp-outbound-stdio-runtime (draft) carries
  `Cargo.lock` as an inferred scope; it is not planned beside this story.
- story:test-binary-per-package-rest (draft, no scope) would move the test files of
  `crates/connectors-host` under one test binary and change the binary names of
  acceptance 5. It is not planned beside this story.
- Three contract texts cite lines of `http.rs` by number, and no test pins a citation:
  `contracts/auth/capability/v1alpha1/semantics.md` (lines 11 to 35),
  `contracts/auth/connection/v1alpha1/semantics.md` (lines 30 to 36) and
  `adapters/prometheus/contracts/series/v1alpha1/semantics.md` (lines 11 to 25). This story
  adds no line above line 37: its imports are written with their full paths.

## Not in this story

- The values of the client's timeouts and of the provider budget.
- The upgrade path (`upgrade` in `http.rs`). Read for this story: when its outer timer wins,
  an unanswered handshake is `Lost` with an unmarked `Timeout`; when reqwest's timer wins, it
  is `Lost` with a marked one; it is `NotSent` only for a connect error. `Lost` tells the
  caller that the effect may have started, which is the safe reading for a write. Whether a
  cut before a connection may become `NotSent`, and its mark, are not changed here; no story.
- A test of a budget that ends before the TCP connection opens: see "Change"; no story.
- The read POST. It sets its timeout from the budget as `send` does, and a cut during the
  handshake is marked there today and after this story. No production port has both: read
  POSTs are admitted by `adapters/grafana/src/local.rs` (30 s) and
  `adapters/tavily/src/local.rs` (160 s), a provider budget is set by
  `adapters/catalog/src/local.rs` alone. Without a budget both read POST timeouts are longer
  than the connect timeout of 5 s, which cuts first and is unmarked. No story.
- A budget with less than 500 ms left when the call connects: a cut during the handshake
  stays marked (Change, item 3). Inside the lead the request's own timer can win over any
  bound of the connect, so a bound there decides nothing. No story.
- The contract text. §6 already says what this story makes `send` do with at least 500 ms
  left; the two lines above are the cases where the client still differs from it.

## As built

Commit 55210be72 on the branch `story/provider-budget-tls-handshake`, not pushed. The branch
starts at 7e0bb9401, the head of the integration branch that holds the five-tests story. The
commit changes 4 files, 250 lines added and 2 removed: `crates/connectors-host/src/http.rs`,
`crates/connectors-host/tests/provider_timeout_adversary.rs`,
`crates/connectors-host/Cargo.toml` and `Cargo.lock`. `Cargo.lock` gains two dependency lines
of `connectors-host` and no `[[package]]` (acceptance 8).

The form is the one of "Change": `BudgetConnectLayer` and its service `BudgetConnect`, the
task-local `CONNECT_UNTIL`, the constants `CONNECT_LEAD` (250 ms) and
`CONNECT_BOUND_MIN_REMAINING` (twice the lead), one `connector_layer` call in the client's
builder after `connect_timeout`, and the scope in `send`. The hunks of `http.rs` start at
lines 100, 350, 680 and 685 of the new file; none is above line 37, none is in `upgrade` and
none is in `post_json` (acceptance 5, the diff half).

Acceptance 1, seen red. The run was made after the change was written, on the three tests
with the product change taken out of the tree for the run, so the product code was the
branch's base. 5 tests ran, 4 passed and 1 failed, the 2 `slow_lane_` tests were skipped. The
failing test was `a_provider_budget_cut_during_an_unanswered_tls_handshake_is_not_marked`,
after 1.165 s, with:

    a provider budget that ended during an unanswered handshake sent no request, after 1.002467993s: upstream_answer = true

The tests of acceptances 2 and 3 passed in that run (1.104 s and 1.124 s): they pass before
the change.

With the change, one run on the workstation: the 5 tests of the binary passed.
`a_provider_budget_cut_during_an_unanswered_tls_handshake_is_not_marked` took 0.866 s,
`a_body_that_stalls_under_a_provider_budget_is_the_providers_timeout` 1.180 s and
`a_request_that_is_sent_and_not_answered_under_a_provider_budget_is_the_providers_timeout`
1.262 s. The two tests of acceptance 4 passed unchanged (0.513 s and 0.551 s). The layer's
`call` saw the task-local value: the question "Change" left to acceptance 1 is answered by
that pass.

One run of the binaries `http_budget`, `http_upgrade` and `upgrade_adversary` with the
change: 21 of 21 tests passed. The commit changes no line of
`crates/connectors-host/tests/http_budget.rs`.

Clippy with `-D warnings` over all targets of `connectors-host` and the format check pass.

One departure from the story: the test of acceptance 3 also asserts that the peer read a
whole request head before the call ended. The story names `Timeout` and
`upstream_answer = true` alone; the added assertion is what shows the request was sent.

Not run: the two `slow_lane_` tests of the file, and no mutant of the change. The figures of
the forge's "repository gate" job for acceptances 5, 6 and 7 are added with the pull request.

## As built, second commit: no lead (decision D1)

An adversary pass on 55210be72 found two defects of the lead. A handshake answered at 800 ms of
a 1,000 ms budget was refused at 751 ms, and a call with 560 ms left was refused where one with
495 ms left was answered 200. The operator decided on 2026-11-26 (D1 = A): no early cut; the
connect is bounded by the end of the budget, and a budget that ends before this request's
connection opened is reported as a connection that did not open.

Commit 3082871f9 on the same branch, on top of 55210be72: 3 files, 587 lines added and 56
removed (`crates/connectors-host/src/http.rs`,
`crates/connectors-host/tests/provider_budget_connect_adversary.rs`, new,
`crates/connectors-host/tests/provider_timeout_adversary.rs`).

- `BudgetConnectLayer` became `WatchedConnectLayer`. It bounds no connect. `CONNECT_LEAD`,
  `CONNECT_BOUND_MIN_REMAINING` and `CONNECT_UNTIL` are gone.
- `send` opens a task-local `CONNECTS` watch per request. A connect the layer started that is
  dropped before it ends is recorded there. When reqwest answers a timeout and such a connect
  was dropped, `send` answers `not_opened()`, an unmarked `Timeout`. Every other error goes
  through `provider_error` as before.
- The pool race: when a pooled connection wins, hyper-util moves the started connect into a
  task of its own and runs it to its end, so it is not dropped while the request waits, and
  the request sent on the pooled connection stays the provider's timeout. A flag "a connection
  was delivered" would have classed that request as not opened; the commit does not use one.

Red first, at 55210be72: 6 adversary cases ran, 3 passed and 3 failed (the 560/495 ms pair,
the handshake answered in the last 250 ms, and an unpolled thread over the lead). With the
change, `provider_budget_connect_adversary` and `provider_timeout_adversary`: 12 ran, 12
passed, 2 `slow_lane_` skipped, slowest 1.61 s. A seventh case,
`a_request_sent_on_a_pooled_connection_while_its_own_connect_runs_is_the_providers_timeout`,
builds the pool race (2 connections accepted, the budgeted request read on the first) and
passes. `connectors-host` lib tests under `http::`: 17 of 17. `http_budget`, `http_upgrade`,
`upgrade_adversary` and `http`: 25 of 25, slowest 2.55 s. Clippy with `-D warnings` and the
format check pass.

Two adversary cases asserted the lead and were changed: the call of
`a_budget_that_ends_before_the_tcp_connection_opens_is_not_marked` now ends between 900 ms and
2 s, and in `a_budget_bounds_its_own_connect_and_not_that_of_a_request_beside_it` the request
without a budget is answered after 1,200 ms.

Open: a connection whose TCP and TLS are up and whose HTTP/2 handshake is pending when the
budget ends is still marked as the provider's timeout (HTTP/1 does no network I/O in that
step). `upgrade()` does not go through `send`, so its connect is not watched, as before.
