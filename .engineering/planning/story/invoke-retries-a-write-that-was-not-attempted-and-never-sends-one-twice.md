---
format: aep.planning-md/3
id: story:invoke-retries-a-write-that-was-not-attempted-and-never-sends-one-twice
kind: story
status: draft
title: invoke retries a write that was not attempted and never sends an applied or unknown write twice
summary: 'Client-side write safety for invoke: --retry-not-attempted <n> and --read-back <op>, one JSON line, exit 0/1/3, so a consumer needs no wrapper'
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:landed-write-is-not-unknown-after-the-owners-budget
revision: 2
---
## Gap

A consumer that calls `connectors invoke` for a write writes its own wrapper around the call
today. Each wrapper holds a lock shared by parallel calls to one instance, a retry of a call
that connectors classifies `not_attempted` up to N tries within one deadline, no second send
after a write classified applied or unknown, an optional read-back for an unknown write, and a
bound on the size of the store the wrapper keeps.

What connectors already does, so this story does not build it again:

- Sends to one instance are serialized across processes: one owner per state directory under
  its lifetime lock (`crates/connectors-host/src/local/owner/transport.rs:1564-1572`) runs an
  instance's exchanges on one serialized worker (`contracts/service/local-mutations.md:147-149`).
  The wrapper's lock is not needed.
- The host's keyed ledger is the store: at most 10,000 attempts per instance
  (`crates/connectors-host/src/local/mutations/types.rs:189`). Between tries the client keeps
  only a count. The wrapper's store is not needed.
- A refused clock acquisition already gets one owner-side second attempt
  (story:an-approved-write-survives-one-refused-clock-acquisition).
- An applied, refused or unknown write is never sent again under the same idempotency key: the
  ledger replays its answer (`crates/connectors-host/src/local/owner/mutation.rs:604`,
  `apps/connectors/src/local/operations.rs:223-227`).

What is missing is the retry of a write that was not attempted.

## Design first

- `connectors.cli.InvokeInput` (`ess/domains/cli.yaml:1825`) gains `retry_not_attempted`, an
  integer from 1 to 10, absent by default. `connectors.cli.MutationObservation`
  (`ess/domains/cli.yaml:910`) gains `tries`, an integer of 1 or more, present when the input
  was given. The answer keeps today's envelope: `.result.mutation` on stdout with exit 0,
  `.error.data.mutation` on stderr otherwise, under `--output json`.
- The exit table of `contracts/cli/v1alpha1/semantics.md:2012` gains exit 3: a write classified
  `unknown`, only when `--retry-not-attempted` was given. Without the option an unknown write
  exits 1 as today. Exit 3 is free: invoke uses 0, 1, 2 and 130.
- The specification change is validated with `ess specify validate` before code.

## Change

`connectors invoke` gains `--retry-not-attempted <n>`. The CLI passes `n` to the owner; the
owner runs the tries on the instance's serialized worker, inside the write's one deadline
(today 20 s per write, `apps/connectors/src/local/session.rs:68`,
`crates/connectors-host/src/local/owner/mutation.rs:272-276`), in the same layer that runs the
clock's second attempt.

- The option needs `--idempotency-key` and the keyed standing-grant path. Beside
  `--approval-file` it is refused before any send: a per-input approval proof is spent at the
  first try (`contracts/service/local-mutations.md:131`).
- A try is repeated only when it answered `not_attempted` with `mutation.attempt` null and one
  of these causes, a closed set: the clock's `unavailable` at stage `admission` after its
  second attempt; the provider's timeout or capacity answer to the preflight
  (`next_action = retry_explicitly`, `contracts/cli/v1alpha1/semantics.md:2443-2444`). Every
  other `not_attempted` (`idempotency_conflict`, `approval_required`, `rate_limited`, an
  aborted attempt) ends the loop with its answer.
- Each try after `bind` uses one of the grant's `max_per_hour`.
- The loop ends at `n` tries or at the write's deadline, whichever comes first.

## Acceptance

All calls with `--output json`, `--idempotency-key`, a standing grant and a fake provider whose
log records each preflight and each commit.

1. A fake whose preflight answers a timeout twice and then succeeds, called with
   `--retry-not-attempted 3`, prints one stdout line with `result.mutation.classification`
   `applied` and `result.mutation.tries` 3, and exits 0. The fake logs 3 preflights and 1
   commit. Before the change the same call without the option exits 1 with `not_attempted`.
2. A fake whose commit answers an unknown effect, called with `--retry-not-attempted 3`, exits
   3 with `error.data.mutation.classification` `unknown` and `tries` 1, and the fake logs 1
   commit. Without the option the same call exits 1, as before.
3. A fake whose commit refuses, called with `--retry-not-attempted 3`, exits 1 with
   `refused` and `tries` 1, and the fake logs 1 commit.
4. A fake whose every preflight answers a timeout, called with `--retry-not-attempted 2`,
   exits 1 with `not_attempted` and `tries` 2; the fake logs 2 preflights and 0 commits.
5. A fake whose every preflight answers a timeout after 8 s, called with
   `--retry-not-attempted 10`, ends within the write's deadline plus 1 s with `not_attempted`
   and `tries` under 10.
6. `--retry-not-attempted 2` beside `--approval-file` exits 2 before any send; the fake logs
   nothing.
7. Two `invoke` processes started together against one instance, each with
   `--retry-not-attempted 3` and a fake that times out each call's first two preflights, leave a
   fake log in which the three preflights of one call (told apart by idempotency key) come
   before the first preflight of the other.
8. Each test runs under 5 s, except 5, which runs under the write's deadline plus 2 s; no test
   waits out a production timeout it does not assert.

## Not in this story

- The read-back of an unknown write: story:an-unknown-write-is-settled-by-a-declared-read-back.
- An unknown whose provider request landed and settled after the owner's budget:
  story:landed-write-is-not-unknown-after-the-owners-budget.
- A change to how the host classifies a call, or re-preparing an aborted key in the ledger.
- Retry of a read.

## Order

Filed from a consumer repository's request on 2026-11-26. Ordered after pull request 315
(merged 23:18 CET). Revised after round 1 of the plan critics (acceptance and design lanes).
