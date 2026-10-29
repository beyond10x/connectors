---
format: aep.planning-md/3
id: story:mcp-reply-after-hangup-owner-latency-under-load
kind: story
status: draft
title: A call at the owner when stdin closes can miss the live lease on a loaded host
relations:
- serves: vision:independent-contract-adapters
revision: 1
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
