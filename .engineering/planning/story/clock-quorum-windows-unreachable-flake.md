---
format: aep.planning-md/3
id: story:clock-quorum-windows-unreachable-flake
kind: story
status: active
title: A verified clock source is never reported unreachable on Windows
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: inferred
  path: apps/connectors/tests/local_cli.rs
- confidence: inferred
  path: crates/connectors-host/tests/fixtures/clock/server.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:58:27Z", actor: "human:operator", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:58:28Z", actor: "human:operator", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Observed

On the pull request that brought the approval clock quorum, the `host and CLI on windows-latest`
job failed once in `apps/connectors/tests/local_cli.rs:266` (`clock_check_below_quorum_refusal`),
and passed on a re-run of the same commit. Of four fixture sources, the fourth (a valid source
with a 5 s radius, which answers exactly like the first) was reported `unreachable` instead of
`verified`:

```
left:  ["verified", "unreachable", "unverified", "unreachable"]
right: ["verified", "unreachable", "unverified", "verified"]
```

Linux and macOS passed. The client shares one 2 s deadline across the sources' exchange threads;
the fixture server (`crates/connectors-host/tests/fixtures/clock/server.rs`) panics its thread on
any receive error other than `WouldBlock`/`TimedOut`, which on Windows includes the
`ConnectionReset` a UDP socket gets after an ICMP port-unreachable.

## Acceptance

- The cause is named with evidence (a Windows run that shows it, or a reproduction), not guessed.
- The fix is in the fixture or the client, never a widened assertion: a source that answers
  within the deadline is `verified` on every platform.
- The test passes 20 consecutive runs on `windows-latest`.

## Wave status

Merged c27b360829 (commit a99bc0d342), 2026-11-10.

- Cause, reproduced on Linux: the client gives every source one shared 2 s deadline (`QUERY_TIMEOUT`); a fixture source whose serving thread starts late is reported `unreachable`. Only the first source was warmed before; the fourth (5 s radius) was not. Red: `local_cli -- clock_` 4 passed, 1 failed with two 2.5 s-late unwarmed sources.
- Reasoned for Windows, not reproduced: the fixture thread panicked on a `ConnectionReset` receive (ICMP port-unreachable) and on a failed `send_to`.
- Fix in the fixture only (`crates/connectors-host/tests/fixtures/clock/server.rs`): every fixture source answers one warm-up request before it is returned, and keeps serving after `ConnectionReset`/`ConnectionAborted`. No assertion widened; `local_cli` 33 passed.
- Not met: 20 consecutive runs on `windows-latest`. No Windows runner was available; the pull request CI runs the Windows job once. The story stays active until that line is observed.
