---
format: aep.planning-md/3
id: story:invoke-retries-a-write-that-was-not-attempted-and-never-sends-one-twice
kind: story
status: draft
title: invoke retries a write that was not attempted and never sends an applied or unknown write twice
summary: 'Client-side write safety for invoke: --retry-not-attempted <n> and --read-back <op>, one JSON line, exit 0/1/3, so a consumer needs no wrapper'
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Gap

A consumer that calls `connectors invoke` for a write writes its own wrapper around the call
today. Each wrapper holds the same five parts:

- a lock shared by parallel calls to one instance;
- a retry of a call that connectors classifies `not_attempted` (the provider unavailable, a
  timeout before the write was sent, a refused approval-clock acquisition), up to N tries
  within one deadline;
- no second send after a write classified applied or unknown;
- an optional read-back operation that settles an unknown write;
- a bound on the size of the store the wrapper keeps.

`classification` and the `not_attempted` settlement exist in the host today
(`crates/connectors-host/src/local/owner/mutation.rs`, `Classification::NotAttempted`;
`crates/connectors-host/src/local/service.rs`, the `not_attempted` settlement). What is missing
is a client-side contract that uses them, so that a consumer needs no wrapper.

## Change

`connectors invoke` gains two options:

- `--retry-not-attempted <n>`: a call that answers `not_attempted` is sent again, up to `n`
  tries in total, all inside the call's one deadline. A call that answers applied, refused,
  failed or unknown is never sent again.
- `--read-back <operation>`: after an unknown write, the named read operation runs once; when
  its answer shows the write, the result names it in `confirmed_by`.

Calls of one process to one instance take one lock, so parallel calls do not interleave their
retries. Whatever the client keeps between tries is bounded in size.

The answer is one JSON line with `classification`, the number of tries, and `confirmed_by`
after a read-back. Exit codes:

- 0: applied, also when a read-back confirmed an unknown write;
- 1: refused or failed;
- 3: unknown, and the read-back (if any) did not find the write.

## Acceptance

1. A fake provider that answers `timeout` (classified `not_attempted`) twice and then success,
   called with `--retry-not-attempted 3`, gives one JSON line with `classification` applied and
   3 tries, exit 0; the provider received 3 sends.
2. A fake provider that answers so the write is `unknown`, called with
   `--retry-not-attempted 3`, gives exit 3, and the provider received one write.
3. With `--read-back <op>` and a fake provider whose read shows the unknown write, the line
   names `confirmed_by: <op>` and the exit is 0; the provider received one write.
4. Two parallel `invoke` calls to one instance with `--retry-not-attempted` do not overlap their
   sends at the fake provider.
5. Each test runs under 5 s; no test waits out a production timeout.

## Not in this story

- Retry of a read: a read is idempotent and has its own path.
- A change to how the host classifies a call.

## Order

Filed from a consumer repository's request on 2026-11-26. Ordered after the current wave
(`integration/wave5`): no implementation starts before that wave's pull request merges.
