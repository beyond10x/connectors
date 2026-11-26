---
format: aep.planning-md/3
id: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
kind: story
status: draft
title: A provider budget that ends in an unanswered TLS handshake is marked as the provider's timeout
summary: With a provider budget shorter than the connect timeout, a timeout during an unanswered TLS handshake carries upstream_answer = true although no request was sent; the contract says a connection that never opened is not the provider's timeout
relations:
- informed_by: story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts
scope:
- confidence: cited
  path: crates/connectors-host/src/http.rs
- confidence: cited
  path: crates/connectors-host/tests/provider_timeout_adversary.rs
revision: 3
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

One run on the build host, on the story's tree, with production's two client timeouts (15 s
and 5 s): a peer on a loopback listener reads the first bytes of the connection and never
answers; the client carries a provider budget of 300 ms. The call ended after 301 ms with
`Timeout` and `upstream_answer = true`; the peer had read 251 bytes. Without a provider budget
the same peer is cut by the connect timeout and the timeout is not marked
(`a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked` in
`crates/connectors-host/tests/provider_timeout_adversary.rs`).

Inferred, not read in reqwest's source: reqwest reports a request timeout that passes during
the connect as a timeout that is not a connect error.

Who reaches it: `adapters/catalog/src/local.rs` calls `with_provider_budget` with the
execution deadline at three places. A call whose budget has less than the connect
timeout of 5 s left when it connects, to a provider that accepts the connection and does not
answer the handshake, reports `timeout` at `stage = dispatch` with
`next_action = retry_explicitly`. By the contract it is not the provider's timeout.

Not measured: the same run on the base commit; a budget that ends before the TCP connection
opens (a peer that accepts nothing); how often a provider call connects with less than 5 s of
its budget left.

## Change

Not decided in this draft. The client must know, when the budget's timeout fires, whether the
request was sent. Two forms to weigh before the story is proposed:

1. the connect and the handshake get a bound of their own, the smaller of the connect timeout
   and what is left of the budget, so that reqwest reports the cut as a connect error;
2. `provider_error` learns from the connection whether the request's head was written.

## Acceptance

1. Red first: a test of `connectors-host` with production's client timeouts and a provider
   budget of 300 ms, against a peer that reads the first bytes and never answers, asserts
   `Timeout` with `upstream_answer = false` and that the peer read at least one byte. It fails
   on the code as it is (the run above) and passes with the change.
2. A body that stalls after the provider's status and headers, under a provider budget, stays
   marked as the upstream's answer.
3. `a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked` and
   `a_body_that_stalls_after_the_provider_answered_is_the_providers_timeout` pass unchanged.
4. No test added by this story takes over 5 s in the gate's test step.

## Scope

- `crates/connectors-host/src/http.rs`: `provider_error` and the place a request gets the
  budget's timeout.
- `crates/connectors-host/tests/provider_timeout_adversary.rs`: the new test.

story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts writes both files. This
story starts after it is merged.

## Not in this story

- The values of the client's timeouts and of the provider budget.
- The upgrade path (`upgrade` in `http.rs`), which sets its own timeout from the same budget:
  whether it has the same mark is checked when this story is planned.
