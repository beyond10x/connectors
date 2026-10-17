---
format: aep.planning-md/3
id: story:clock-quorum-windows-unreachable-flake
kind: story
status: draft
title: A verified clock source is never reported unreachable on Windows
relations:
- serves: vision:independent-contract-adapters
revision: 1
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
