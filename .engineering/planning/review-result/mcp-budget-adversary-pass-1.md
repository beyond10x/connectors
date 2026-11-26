---
format: aep.planning-md/3
id: review-result:mcp-budget-adversary-pass-1
kind: review-result
status: active
title: Adversary read of the first commit of the MCP budget change
relations:
- reviews: story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds
revision: 1
---
Verdict: no finding holds the change. For the first commit of the change of
story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds. Each of the four shortened tests
still goes red for the defect it guards; five mutants of product code show it. Two introduced
findings and three pre-existing ones are settled in the second commit of the same pull request;
two are not changed and say why; one is the story's acceptance, which the operator's decision
changes.

Adversary read of the committed change. The adversary ran five mutants of product code in the
tree, one run each on the build host of the shared pool, and took each back; it ran eight cases
of its own from copies of the two test files and removed them. It did not run a `slow_lane_`
test. Both test files are compiled on Linux alone (`#![cfg(target_os = "linux")]`).

## Mutants

- The session closes after the late refusal is written: the control is red ("pinged false").
- End-of-file reads the store's authority: the stdin-closed test is red ("the stdio server did
  not exit").
- Renewal waits for the store: the long-read test and the control are red; the stdin-closed test
  is green.
- The owner ignores a read's cancel and the closed stream: the lost-session test is red (the
  session ended 2.31 s after dispatch, the read was still open at 8 s); the teardown test is
  green.
- Nobody waits for the store's lock: the long-read test as committed is green; with one more
  assertion it is red (finding 1).

## Findings

1. Existed before, shown by a mutant: the long-read test has no assertion that notices whether
   the owner's record met the held store.
   Outcome: fixed. Before the store is released the test asserts that no reply has come. It
   cannot show an owner that arrives after the release.
2. Introduced, shown by a case: the hold ended 0.5 s after the provider's answer, before 1 s. An
   owner slower than that left the test green without the record having met the held store.
   Without a hold the reply comes 55 ms after the provider's answer on the build host.
   Outcome: the shortening of this test is taken back (provider 3 s, hold 4 s). It saved 0.7 s.
3. Existed before, shown by a case: the control released the store 600 ms after the budget as the
   test counts it. A server that reads the request late answers `result`, and the control is red
   for no defect.
   Outcome: fixed. The control holds the store until its reply and the following ping are read.
4. Existed before, shown by a run: the control's comment said the reader starts 0.5 s after the
   deadline; the helper sleeps the release first, so it starts 0.6 s after it, and the refusal is
   in the pipe before the reader starts.
   Outcome: the comments are corrected.
5. Introduced, read and not run: the 4 s budget also bounds the server's start (before 10 s).
   Outcome: not changed. The start took about 0.3 s on the build host; the window is over 2 s.
6. Introduced, read and not run: a close in the provider's last poll before its 8 s mark counted
   as neither open nor closed; the lost-session test would then fail at 20 s without a verdict.
   Outcome: fixed. After the loop the provider counts open or closed in every case.
7. Introduced, read and not run: the lost-session test ends at the first verdict and would miss
   a second stalled connection.
   Outcome: not changed. One call is sent and nothing retries it.
8. Introduced: acceptance 1 (42 s for the six tests) and acceptance 3 and 4 (the lane test) are
   not met.
   Outcome: the operator's decision. A request budget under 20 s lists no tool, so the walk test
   and the on-time test stay in the gate at 20 s, and
   story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs (draft) holds them.
   The story's "As built" section says which acceptance statements hold.

## Attacked and not broken

- The control's `tools/list` is held by the store as before, in the wait for the store's
  authority, before any tool is selected. Without a hold the reply is `result`, and the test
  asserts `timeout`.
- The stdin-closed test holds the store for the whole exit; a failing test releases it through
  the dropped channel.
- The teardown test, which shares the provider's stalled branch, means what it meant: the
  provider acts differently only after the client closed.
- On Linux `peek` reports a close only after the peer's end of stream or a reset.

## Not established

- Any figure on the forge; nothing ran under load.
- Whether the control at 20 s reached `timeout` by the same wait on the store; the base was not
  run.
