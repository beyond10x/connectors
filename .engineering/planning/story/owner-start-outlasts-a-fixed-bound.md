---
format: aep.planning-md/3
id: story:owner-start-outlasts-a-fixed-bound
kind: story
status: implemented
title: A read times out for ever when the owner's start outlasts the CLI's fixed 10-second wait
relations:
- serves: vision:independent-contract-adapters
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T00:30:55Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-30T00:30:56Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-30T05:24:39Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Defect

Every read through `operations invoke` on one operator's machine answered, after about 22 s,
`{"code":"timeout","kind":"operational","next_action":"retry_explicitly","stage":"admission"}`,
and so did every retry. `operations describe` answered at once. The state directory's
`metadata.sqlite3` held 217 MB: 6,031 events and 21,830 blobs (172 MB), no snapshots.

Observed on 2026-10-30 on Linux with connectors 0.59.1, the host's load average at 21 to 31:

- `strace -f` of one read: the CLI read the store for 27 s (`pread64`), then started the owner
  (`execve("/proc/self/fd/21", ["__connectors-owner", ...])`). The owner read the store for 9 s
  and was still reading when the CLI ended it: `+++ killed by SIGKILL +++`, and the CLI answered
  `timeout`.
- Sampled each second without strace: the owner started at 12.4 s; the CLI answered at 21.8 s,
  the owner's CPU time then 9 s and still rising.
- `perf record` of the same read: the time goes to `sha2::sha256::compress256`, `serde_json`
  parsing and `BTreeMap` inserts under `entity-eventlog` threads: verification of the whole store.

## Cause

`operations invoke` without a running owner admits the read directly, which verifies the whole
store, and then starts an owner through `owner::Client::connect(&paths, true)`. That call waited
for the new owner's greeting at most 10 s, a fixed bound independent of the read's own 30 s
deadline. The owner verifies the whole store (`Metadata::start_owner`) before it answers, so its
start grows with the store. Past 10 s the CLI killed the owner it had started
(`transport.rs`, `connect_as`) and answered `timeout`; every later read started and killed an
owner of its own, so none ever got one. `connections revalidate` and every other command that
starts an owner through `Client::connect` had the same 10 s bound. The `admission` stage label is
the one every host-side timeout carries; it does not locate the wait.

## Outcome

- A read with no owner running waits for the owner it starts within the read's own deadline.
- A command without a deadline of its own waits for the owner it started up to a startup bound of
  60 s from that start instead of 10 s, and for an owner another process is starting up to 60 s
  from when it found it starting. It still reaches a running owner within 10 s.
- A caller that stops waiting with `timeout` leaves the owner it started to finish its start; the
  next command reaches it. A caller that fails otherwise still ends it.

## Acceptance

- `apps/connectors/tests/owner_slow_start.rs::a_read_waits_for_a_slow_owner_start_within_its_deadline`:
  with the owner taking 12 s to answer its greeting, a read succeeds, and the next read reaches
  that owner in under 12 s.
- `apps/connectors/tests/owner_slow_start.rs::a_revalidation_waits_for_a_slow_owner_start`: with
  the same owner, `connections revalidate` answers `ready`.
- `apps/connectors/tests/owner_slow_start_adversary.rs::a_read_after_one_that_outlasted_its_deadline_reaches_the_owner_it_left`:
  with the owner taking 34 s, the first read answers `timeout` and the next read succeeds in
  under 34 s.
- `apps/connectors/tests/owner_slow_start_adversary.rs::a_revalidation_during_another_commands_slow_owner_start_reaches_that_owner`:
  `connections revalidate` issued while a read's owner is in a 14 s start answers `ready`.
- `apps/connectors/tests/owner_slow_start_adversary.rs::owner_stop_during_a_slow_owner_start_stops_that_owner`.
- The first two fail on the parent commit with the reported `timeout` at `admission`.

## Not in scope

- Writes: `WriteClient::connect` keeps its 10 s wait for the owner's greeting inside the write's
  20 s budget, so a write that has to start an owner on a large store can still time out.
- `connections launch` sets its 30 s deadline before it reaches the owner, so on a slow start the
  owner wait can use part of it.
- The cost itself: every cold start verifies the whole store twice (the CLI's direct admission,
  then the owner's start), and the store keeps no snapshot.
