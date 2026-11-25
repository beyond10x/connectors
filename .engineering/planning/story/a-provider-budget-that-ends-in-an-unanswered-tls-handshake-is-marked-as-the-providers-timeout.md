---
format: aep.planning-md/3
id: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
kind: story
status: draft
title: A provider budget that ends in an unanswered TLS handshake is marked as the provider's timeout
summary: With a provider budget shorter than the connect timeout, a timeout during an unanswered TLS handshake carries upstream_answer = true although no request was sent; a connector layer bounds the connect by the budget so that the cut is reported as a connect error
relations:
- informed_by: story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts
- depends_on: story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/connectors-host/Cargo.toml
- confidence: cited
  path: crates/connectors-host/src/http.rs
- confidence: cited
  path: crates/connectors-host/tests/provider_timeout_adversary.rs
revision: 7
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

Inferred, not read in reqwest's source: reqwest reports a request timeout that passes during
the connect as a timeout that is not a connect error. The run above agrees with it.

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
  client per budget.
- reqwest does not hand a request's extensions to hyper (`execute_request` builds the hyper
  request from method, URI, version, headers and body), so hyper-util's `capture_connection`
  cannot tell `provider_error` whether a request got a connection.

The form:

1. The client is built with one `connector_layer`. The layer's service reads a deadline from
   a tokio task-local when the connector is called, and bounds the connect future it returns
   with that deadline. Past the deadline it fails with an `io::Error` of kind `TimedOut`.
   Without a task-local value it changes nothing. reqwest's own `connect_timeout` stays the
   outermost layer.
2. `send` and the read POST run `request.send()` inside the scope of that task-local, set to
   the end of the budget less a lead of 50 ms (a named constant). The request's own timeout
   stays what it is today. The lead is there so that a connect the budget cuts ends by the
   connect's bound; which of two timers at one instant wins is not relied on. With less than
   the lead left, the connect fails at once.
3. `provider_error` does not change. reqwest reports the layer's failure as a connect error
   (`is_connect()`: the error comes from the connector) that is a timeout (`is_timeout()`:
   an `io::Error` of kind `TimedOut` in its sources), and that branch is unmarked today.
4. For `send` and the read POST the case of the defect becomes an unmarked `Timeout`:
   `Failure::Timeout`, reported as `timeout` at `stage = admission` with
   `next_action = retry_explicitly`. That is what the client's own connect timeout reports
   today. `Failure::from_provider` does not change. `upstream_answer` is a field of the
   internal `Error`, not a wire field; `stage` is the field a caller sees change, in this one
   case.
5. A request that got its connection is as today: the request's timeout ends it, and the
   timeout is marked. A request sent on a pooled connection calls no connector and is as
   today.
6. `upgrade` is not changed. It does not set the task-local, so its connect has the client's
   connect timeout alone, and its two timers and its choice between `Lost` and `NotSent` stay
   as they are.
7. `tower-layer` and `tower-service` become direct dependencies of `connectors-host` at the
   versions `Cargo.lock` already holds (0.3.3 each). `Cargo.lock` gains two dependency lines
   of `connectors-host` and no package.

Inferred, not run: hyper-util calls the connector while it polls the request's future, on
the caller's task, so the layer sees the task-local. Acceptance 1 decides it. If the layer
does not see the value, the test stays red and this story is re-planned before any other
form is built.

A budget that ends before the TCP connection opens is cut by the same bound and is unmarked.
No test builds that case: a TCP connect that hangs needs an address outside the host, and the
test step makes no network request.

## Acceptance

1. Red first: a new test
   `a_provider_budget_cut_during_an_unanswered_tls_handshake_is_not_marked` in
   `crates/connectors-host/tests/provider_timeout_adversary.rs`, with production's client
   timeouts and a provider budget of 300 ms, against a peer that reads the first bytes and
   never answers, asserts: `Timeout`, `upstream_answer = false`, the peer read at least one
   byte, the call ended within 1 s, and `Failure::from_provider` of the error is
   `Failure::Timeout`. Run before the change, on the commit this story branches from (main
   with the five-tests story), it fails with `upstream_answer = true`; "As built" holds that
   run's message. It passes with the change.
2. A stalled body stays the provider's: a new test
   `a_body_that_stalls_under_a_provider_budget_is_the_providers_timeout` in the same file, a
   peer that sends status and headers and then stalls, a provider budget of 300 ms, asserts
   `Timeout`, `upstream_answer = true` and `Failure::ProviderTimeout`. It passes before the
   change and after it.
3. A slow answer stays the provider's: a new test
   `a_request_that_is_sent_and_not_answered_under_a_provider_budget_is_the_providers_timeout`
   in the same file, a plain-HTTP peer that reads the request and sends nothing, a provider
   budget of 300 ms, asserts `Timeout` and `upstream_answer = true`. It passes before the
   change and after it.
4. `a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked` and
   `a_body_that_stalls_after_the_provider_answered_is_the_providers_timeout` pass unchanged.
5. The upgrade path is untouched: the pull request's diff changes no line of the function
   `upgrade`, and the test binaries `http_upgrade` and `upgrade_adversary` of
   `connectors-host` pass in the forge's "repository gate" job.
6. No test added by this story takes over 5 s in the nextest output of the test step of the
   forge's "repository gate" job of the pull request; none is a `slow_lane_` test.
7. `git diff --stat` of the pull request names, outside the planning store, `http.rs`, the
   test file, `crates/connectors-host/Cargo.toml` and `Cargo.lock`, and `Cargo.lock` gains no
   `[[package]]`.

## Scope

- `crates/connectors-host/src/http.rs`: the layer, the task-local, the client's builder,
  `send`, the read POST.
- `crates/connectors-host/tests/provider_timeout_adversary.rs`: the three new tests.
- `crates/connectors-host/Cargo.toml` and `Cargo.lock`: the two direct dependencies.

Other stories on these files:

- story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts writes `http.rs`
  (the client's builder among its lines) and the test file. This story depends on it and
  starts after it is merged.
- `Cargo.lock` is a file every story with a new dependency writes. No story that is planned
  beside this one adds a dependency.
- `contracts/auth/capability/v1alpha1/semantics.md` cites lines 11 to 35 of `http.rs` by
  number; no test pins that citation. This story adds no line above line 36: its imports are
  written with their full paths.

## Not in this story

- The values of the client's timeouts and of the provider budget.
- The upgrade path (`upgrade` in `http.rs`). Read for this story: when its outer timer wins,
  an unanswered handshake is `Lost` with an unmarked `Timeout`; when reqwest's timer wins, it
  is `Lost` with a marked one; it is `NotSent` only for a connect error. `Lost` tells the
  caller that the effect may have started, which is the safe reading for a write. Whether a
  cut before a connection may become `NotSent`, and its mark, are not changed here; no story.
- A test of a budget that ends before the TCP connection opens: see "Change"; no story.
- The contract text: §6 already says what this story makes the client do.
