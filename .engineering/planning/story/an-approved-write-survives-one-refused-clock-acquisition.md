---
format: aep.planning-md/3
id: story:an-approved-write-survives-one-refused-clock-acquisition
kind: story
status: draft
title: An approved write survives one refused clock acquisition
summary: One clock acquisition per write, one repeat of a below-quorum refusal inside the caller's deadline, and the refusal names its sources
relations:
- serves: vision:independent-contract-adapters
revision: 2
---
## What happened

2026-11-24, 03:58 to 04:42 CET, on the workstation: Slack posts through an approved write
(`approvals issue`, then `invoke chat.postMessage`, connectors 0.74.0) failed on and off before
any provider request. 12 of 12 attempts of a 3-minute loop failed from 03:58:34 to 04:16:59, with
single successes and failures after it. The channel was the operator's only one that night.

```
{"step":"issue","code":"unavailable","kind":"operational","next_action":"retry_status","stage":"readiness"}
{"step":"invoke","code":"unavailable", ... "classification":"not_attempted" ... "stage":"admission"}
```

## Cause (measured and read from the code)

- The approval clock asks every configured Roughtime source once, in parallel, under one
  two-second deadline, and does not repeat (`crates/connectors-host/src/local/clock.rs:28`,
  `:375`; `contracts/service/clock.md`).
- `approvals issue` acquires the clock once (`owner/approval_issuance.rs:381-388`). The owner's
  write path acquires it again through `time()` (`owner/mutation/execution.rs:84-97`), which the
  path calls twice per write. One approved write needs three acquisitions in a row.
- Of the four sources of `docs/local-clock.md`, two did not answer at all: Cloudflare at wire
  version `roughtime-0x8000000c`, and `time.txryan.com`, whose address had moved from
  148.113.255.71 to 149.56.214.17 (same key). `quorum = 2` was therefore two of two.
- The two answering sources each dropped out for some seconds at a time: 20 checks 4 s apart,
  04:40:10 to 04:41:28 CET, gave 18 ok and 2 `below_quorum`; 8 rounds of 3 parallel checks gave
  0 of 3, 1 of 3, then 6 rounds of 3 of 3. Ping to both showed no loss.
- The dropouts follow the host's uplink: 90 checks 4 s apart with three answering sources,
  04:44:45 to 04:50:48 CET. While the uplink sent 128 to 138 Mbit/s (a copy to a build server),
  23 of 117 source answers were missing and 6 of 39 checks were refused; under 62 Mbit/s, 2 of
  153 answers were missing and 1 of 51 checks was refused. Host load was the same in both
  (median 42 and 41).
- The refusal the caller sees names no source and no `clock_refusal`: `acquire()` drops the
  detail that `acquire_explained()` has (`clock.rs:377`), so `issue` and `invoke` answer a bare
  `unavailable` and the caller cannot tell a clock refusal from another one.
- None of 0.74.1 to 0.76.2 changed these files.

Done outside the product the same night: the callers' post script retries the two refusals that
carry no write, and the seven local configurations got the source's address of today.

## What changes

1. One approved write acquires the clock once in the owner: the bounded clock of the first
   acquisition is read again where the path acquires a second time today. A bounded clock already
   answers later readings with its rate error (`BoundedClock::now`).
2. An acquisition that ends `below_quorum` with no source `unverified` is repeated once, with
   fresh nonces, inside the caller's deadline, in `approvals issue` and in the owner's write path.
   `split`, `too_imprecise`, an `unverified` source and a local timer refusal are not repeated.
3. A clock refusal of `approvals issue` and of `invoke` carries `clock_refusal` and the
   `clock_sources` lines `approvals clock-check` prints today.
4. `contracts/service/clock.md` and `docs/local-clock.md` state the repeat and the single
   acquisition per write; `docs/local-clock.md` holds 149.56.214.17:2002 for `time.txryan.com`
   and says that a numeric address is rechecked against the source's name when a source stops
   answering.

## Acceptance

1. A test with local UDP sources: every source drops the first exchange and answers the second.
   `approvals issue` succeeds, and an approved write is admitted and dispatched once. Before the
   change the same test ends `unavailable`.
2. A test counts the exchanges each local source receives during one admitted approved write
   through the owner: one per source (two when the first is dropped), not two.
3. A test: sources that never answer end `approvals issue` as `unavailable` within the caller's
   deadline, after exactly two exchanges per source, with `clock_refusal` `below_quorum` and one
   `clock_sources` line per source in the error.
4. A test: one source answering with a bad signature (`unverified`) ends the acquisition after
   one exchange per source.
5. The second reading of the reused clock is no narrower than the first and no later than the
   approval's expiry check allows: the existing expiry and ledger tests pass unchanged.
6. `rg -n '149.56.214.17' docs/local-clock.md` finds the line; the contract and the guide state
   the repeat; the CLI contract lists `clock_refusal` and `clock_sources` for both commands.
7. No test added runs over 5 s; none over the 60 s bound.

## Open before it is proposed

- Whether a repeat changes what an attacker on the path gains. Reading: nothing, a dropped
  exchange already refuses the write and a repeat signs nothing more. The security critic rules
  on it.
- Whether the repeat belongs in `acquire()` for every caller or only in the two write paths.

## Out of scope

- Admitting RFC 10049's wire version (`roughtime-1`), which the Cloudflare source may need.
- A clock kept by the owner across writes.
