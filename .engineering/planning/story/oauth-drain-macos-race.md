---
format: aep.planning-md/3
id: story:oauth-drain-macos-race
kind: story
status: archived
title: A request queued on the OAuth loopback listener is answered on macOS, never dropped
revision: 2
transitions:
- {from: "draft", to: "archived", at: "2026-10-19T18:40:42Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"artifact":1}}}
---
## Observed
The OAuth loopback tests in `crates/connectors-host/src/local/oauth.rs` fail intermittently on
the macOS runner only:
- `local::oauth::tests::a_request_queued_before_the_flow_returns_is_answered` failed on pull
  request https://github.com/beyond10x/connectors/pull/170, run 38633201731, 2026-10-14:
  `oauth.rs:1090` asserts a 400 answer and read `""`, an empty answer. Nothing in that pull
  request touches `oauth.rs`.
- `local::oauth::tests::second_request_refused` failed on `main`, run 38455605662, 2026-10-13.
- Linux and Windows have shown neither.

The test queues a request on the loopback listener, starts a `Drain` and drops it at once. It
expects the queued request to be answered 400 before the listener closes. On macOS the
connection is sometimes closed unanswered. Commit `be3f6f201` ("answer a request queued before
the consent flow returns") targeted this race, and it still occurs.

## Acceptance
- The cause is shown from a macOS reproduction (for example the test repeated 500 times on the
  macOS runner) and recorded here: the drain's accept or poll order, the listener's
  non-blocking accept on macOS, or the test's read of a reset connection.
- Every request queued on the loopback listener before the flow returns is answered (400 when it
  does not decide) or the client sees a defined refusal, never an empty read, on macOS as on
  Linux.
- The two named tests pass 500 consecutive repetitions on the macOS runner.
