---
format: aep.planning-md/3
id: story:invoke-retries-a-write-that-was-not-attempted-and-never-sends-one-twice
kind: story
status: draft
title: invoke retries a write that was not attempted and never sends an applied or unknown write twice
summary: 'invoke --retry-not-attempted <n>: the owner retries a write that was not attempted for a closed set of causes, inside the write''s deadline with a reserve; tries in the answer; exit 3 for unknown with the option'
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:landed-write-is-not-unknown-after-the-owners-budget
revision: 5
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
  (`crates/connectors-host/src/local/mutations/types.rs:189`). Between tries the owner keeps
  only a count. The wrapper's store is not needed.
- A refused clock acquisition already gets one owner-side second attempt
  (story:an-approved-write-survives-one-refused-clock-acquisition).
- An applied, refused or unknown write is never sent again under the same idempotency key: the
  ledger replays its answer (`crates/connectors-host/src/local/owner/mutation.rs:603-610`,
  `apps/connectors/src/local/operations.rs:223-227`).

What is missing is the retry of a write that was not attempted. The read-back is
story:an-unknown-write-is-settled-by-a-declared-read-back.

## Design first

- `connectors.cli.InvokeInput` (`ess/domains/cli.yaml:1825`) gains `retry_not_attempted`, an
  integer from 1 to 10, absent by default; it needs `idempotency_key`.
  `connectors.cli.MutationObservation` (`ess/domains/cli.yaml:1944`) gains `tries`: present only
  when `retry_not_attempted` was given; 0 when the answer is a same-key replay answered before
  any try (`operations.rs:223-227`, `crates/connectors-host/src/local/owner/mutation/execution.rs:282-284`),
  else the number of tries made.
- The answer keeps today's envelope: `.result.mutation` on stdout with exit 0,
  `.error.data.mutation` on stderr otherwise, under `--output json`.
- The exit table (`contracts/cli/v1alpha1/semantics.md:1984`) gains a row: a write classified
  `unknown` exits 3, only when `--retry-not-attempted` was given. Without the option an unknown
  write exits 1 as today. Exit 3 is free: invoke uses 0, 1, 2 and 130.
- The specification change is validated with `ess specify validate` before code.

## Change

`connectors invoke` gains `--retry-not-attempted <n>`. The CLI passes `n` to the owner.

- The loop wraps the supervisor's write job (`crates/connectors-host/src/local/owner/supervisor.rs:1295-1298`,
  `:1328`), on the instance's serialized worker. Each try launches a fresh adapter child: a
  retried cause ends the child of the try (`execution.rs:442`,
  `crates/connectors-host/src/local/runtime/process.rs:1388-1393`), and a later try on it would
  answer `unavailable` (`process.rs:1027-1028`).
- A try is repeated only when it answered `not_attempted` with `mutation.attempt` null and the
  cause is the provider's own timeout or capacity answer to the preflight (`ProviderTimeout`,
  origin provider; `next_action = retry_explicitly`, `semantics.md:2443-2444`). That is the
  closed set. Every other `not_attempted` ends the loop with its answer: the host's own
  timeout, a clock refusal, `idempotency_conflict`, `approval_required`, `rate_limited`,
  `forbidden`, an aborted attempt.
- Reserve: another try starts only while the duration of the try just ended plus 3 s (the
  first reading's reserve of `contracts/service/clock.md:251-256`) is left before the write's
  deadline (today 20 s per write, `apps/connectors/src/local/session.rs:68`). Otherwise the
  loop ends with the last try's answer, so a `not_attempted` near the deadline is not turned
  into `outcome_unknown` by the write client (`transport.rs:1329-1340`).
- Approvals: every retried try ends before `prepare`, where a per-input proof is spent and a
  grant use is bound (`execution.rs:565-614`); an unbound grant use is given back
  (`execution.rs:133`). So `--approval-file` and a standing grant both work with the option,
  and the retried tries spend no proof and no grant use.

## Test fixture

None exists. The owner-level child of `crates/connectors-host/src/local/runtime/process/write_tests.rs`
gains a counted preflight script (the answer of the k-th preflight, with an optional delay)
and an ordered log of one line per preflight and per commit carrying the input's `value`; it
accepts `{"value": true}` and `{"value": false}`. The CLI-level statements run it through a
stand-in adapter like `apps/connectors/tests/support/standin.rs`. A provider timeout is the
fixture's answer `Failure::from_provider(Timeout.answered())`, returned at once or after the
named delay, not the 15 s HTTP cutoff.

## Acceptance

All calls with `--output json` and `--idempotency-key`, against the fixture above.

1. Under a standing grant of `max_per_hour` 1, a fixture whose preflights 1 and 2 answer the
   provider's timeout, called with `--retry-not-attempted 3`, prints one stdout line with
   `result.mutation.classification` `applied` and `result.mutation.tries` 3, and exits 0. The
   log reads preflight, preflight, preflight, commit. Before the change the call exits 2
   (unknown flag); without the option it exits 1 with `not_attempted`.
2. A fixture whose commit answers an unknown effect, called with `--retry-not-attempted 3`,
   exits 3 with `error.data.mutation.classification` `unknown` and `tries` 1; the log holds 1
   commit. Without the option the same call exits 1 and `error.data.mutation` has no `tries`.
3. A fixture whose commit refuses, called with `--retry-not-attempted 3`, exits 1 with
   `refused` and `tries` 1; the log holds 1 commit.
4. A fixture whose every preflight answers the provider's timeout, called with
   `--retry-not-attempted 2`, exits 1 with `not_attempted` and `tries` 2; the log holds 2
   preflights and 0 commits.
5. A fixture whose every preflight answers the provider's timeout after 6 s, called with
   `--retry-not-attempted 10`, exits 1 with `not_attempted` and `tries` 2, within 20 s: the
   third try would not leave 6 s plus 3 s.
6. `--retry-not-attempted 2` without `--idempotency-key`, and `--retry-not-attempted` 0 and 11
   with one, each exit 2 with `error.data.code` `invalid_input`, `field` `retry_not_attempted`,
   `stage` `arguments`; the log is empty.
7. Two processes with inputs `value` true and false, each `--retry-not-attempted 3`, against a
   fixture whose preflights 1, 2, 4 and 5 answer the provider's timeout, both exit 0 with
   `tries` 3, and the log reads three preflights and a commit of one value, then three
   preflights and a commit of the other.
8. A preflight answering 403 (`not_attempted`, `forbidden`), called with
   `--retry-not-attempted 3`, exits 1 with `tries` 1; the log holds 1 preflight.
9. A same-key replay of statement 1's call exits 0 with `applied` and `tries` 0; the log gains
   nothing.
10. The exit table of `contracts/cli/v1alpha1/semantics.md` has the row for `unknown` with
    `--retry-not-attempted`, exit 3, and `ess specify validate` passes on the changed domain.
11. Each test runs under 5 s, except 5, which runs under 21 s.

## Not in this story

- A retry of a clock refusal at `admission`: the host answers it as a bare `unavailable`
  without a clock marker (`crates/connectors-host/src/local/owner.rs:261-272`), and the owner
  already makes one second attempt. It can join the closed set once
  story:a-clock-refusal-of-issue-and-invoke-names-its-sources marks it.
- The read-back of an unknown write: story:an-unknown-write-is-settled-by-a-declared-read-back.
- An unknown whose provider request landed and settled after the owner's budget:
  story:landed-write-is-not-unknown-after-the-owners-budget.
- The host's own timeout of a hung preflight: it ends the loop at its try.
- A write deadline named by the caller; a change to how the host classifies a call;
  re-preparing an aborted key in the ledger; retry of a read.

## Order

Filed from a consumer repository's request on 2026-11-26, ordered after pull request 315.
Revised after rounds 1 and 2 of the plan critics (acceptance and design lanes).
