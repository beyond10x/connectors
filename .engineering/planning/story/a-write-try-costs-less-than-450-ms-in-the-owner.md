---
format: aep.planning-md/3
id: story:a-write-try-costs-less-than-450-ms-in-the-owner
kind: story
status: draft
title: A write try costs less than 450 ms in the owner before the provider is reached
summary: Three ledger observations and an issuance resolve cost about 450 ms per try of a keyed write; two retry tests are over 5 s because of it
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:invoke-retries-a-write-that-was-not-attempted-and-never-sends-one-twice
revision: 1
---
## Defect

One try of a keyed write in the owner costs about 450 ms before the provider is reached,
measured by the implementation of
story:invoke-retries-a-write-that-was-not-attempted-and-never-sends-one-twice on the build host
on 2026-11-27: three ledger observations of about 70 ms each (one before the try in
`crates/connectors-host/src/local/owner/supervisor.rs`, two at
`crates/connectors-host/src/local/owner/mutation/execution.rs:282`) and `issuance::resolve` at
about 28 ms. A test site's setup takes 1.9 s, of which `connect` is 1.16 s.

Two tests of `apps/connectors/tests/invoke_retry.rs` are over 5 s because of it:
`two_writes_take_their_tries_one_after_the_other` 5.83 s (6 tries, 2 commits) and
`a_same_key_replay_names_no_try` 5.04 s. Under the 20 s write deadline every 450 ms is budget a
retried write does not have.

## Change, to be designed

The observations of one try read the same ledger rows; read them once per try. Measure before
and after with the two tests above.

## Acceptance

To be written with the measurement. The aim: both tests under 5 s on the build host.
