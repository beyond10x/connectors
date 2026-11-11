---
format: aep.planning-md/3
id: story:mcp-reply-after-hangup-owner-latency-under-load
kind: story
status: implemented
title: A call at the owner when stdin closes can miss the live lease on a loaded host
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: inferred
  path: apps/connectors/tests/local_mcp_server.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:58:33Z", actor: "human:operator", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:58:34Z", actor: "human:operator", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "active", to: "implemented", at: "2026-11-11T06:19:18Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Observation

`local_mcp_server::stdin_close_still_answers_requests_already_read` failed 1 of 10 whole-binary
runs at 1d2e8f468, and 1 of 50 runs of that test alone on an instrumented build of the same head.
Both runs were pinned to 4 CPUs beside 4 `nice -n 19` workers, with the host load average between
8 and 43 from other sessions.

The instrumented failure traced (server stderr):

- `hang_up lease left 1863 ms` (renewed on time just before the close).
- 1.87 s later: `permit stdin closing decision Expired`, the serve loop ends with Timeout, and the
  server prints `mcp_unavailable` and exits 1.
- The call was at the owner (provider requests 1, provider hold 300 ms). The owner's reply did
  not arrive within the lease that was live at the close.

This is the behaviour `docs/local-mcp-stdio.md` (stdin close) documents: "A reply the lease
cannot carry is discarded and the exit is unsuccessful." It shows that the owner's work after the
provider answered took over 1.5 s on that host. Where the owner spent that time (audit records
under `synchronous=FULL`, the store lock) has not been measured.

## Outcome

Decide from a measurement whether the owner's latency for a bounded read under load is a defect,
or whether the test's assumption (a 300 ms read is answered within the lease at the close) is
wrong under load. Then fix the owner, or fix the test without loosening what it asserts.

## Acceptance

- The owner's time for that read is broken down by stage on a loaded host.
- The decision is recorded, with either a failing test first or the reason the test changes.
- No `#[ignore]` and no retry.

## Decision

Decided on 2026-11-10 (single defect wave, merge 56067e56f, commit d5111627f): the defect is the
test's premise. The owner is not slow in any one stage. The lease at the stdin close is the contract
(`docs/local-mcp-stdio.md`; renewal after the terminal frame is forbidden).

Stage times for the bounded read, measured on 2 CPUs at nice 19 with load average 77 to 128. Times
are milliseconds after the server sent the read to the owner.

| Stage | Passing run | Failing run |
|---|---|---|
| admission to plan resolved | 143 | 44 |
| audit anchor write (store lock, `synchronous=FULL`) | 430 | 857 |
| confirm | 45 | 16 |
| re-resolve | 112 | 66 |
| adapter start until the provider sees the request | 1232 | 2355 |
| provider hold | 302 | 302 |
| provider answer to dispatch end | 148 | 1019 |
| final audit append | 265 | 1022 |
| reply enqueue and write | about 5 | about 5 |

Every stage was 3 to 7 times slower than on an idle host; none stands out.

- **The test fix.** The provider now holds the call until the test has closed stdin, so the close
  lands while the call is at the owner and the 300 ms hold no longer counts against the lease. The
  test also waits longer for the call to reach the provider (5 s to 30 s) and for the reply after
  the close (5 s to 15 s). Its assertions are unchanged.
- **Results with the fix.** 20 of 20 runs passed with 1 busy worker, and 19 of 20 with 2.
- **Left open.** With 3 workers, at load 100 and more, 14 of 20 passed. The owner's work after the
  provider answered still outran the lease. Closing that window needs a decision on the contract
  for the post-close lease.
