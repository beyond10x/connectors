---
format: aep.planning-md/3
id: review-result:adversary-owner-rotating-refresh-pass-1
kind: review-result
status: active
title: 'U4 adversary pass 1: owner-rotating-refresh'
relations:
- reviews: story:owner-rotating-refresh
revision: 1
---
unit: story:owner-rotating-refresh, 120b5fac5 plus an uncommitted adversary test file
verdict: red
cases: executed 604→606, red 2
origin: introduced 3, pre-existing 0, undecided 1
reviewer: aep:adversary (opus), pass 1, 2026-10-26

```findings
- file: crates/connectors-host/src/local/registry/refresh.rs
  line: 559
  category: concurrency
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: a read recollection that finishes during the authorized exchange advances the publication fence, so prepare_refresh refuses the response, the rotated refresh token is dropped and the connection goes Stale
- file: crates/connectors-host/src/local/owner/refresh.rs
  line: 304
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: recovery dates the stored response evidence from its store acknowledgement instead of rechecking it, so an owner restarting later than the evidence lifetime discards a durable valid candidate and marks the connection Stale
- file: crates/connectors-host/src/local/registry/refresh.rs
  line: 575
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the 64 retained-version bound is checked in prepare_refresh after the exchange is sent, so a connection refreshing more than 63 times in 24 h burns its token on a local limit it could have refused before authorizing
- file: crates/connectors-host/src/local/metadata/er.rs
  line: 1
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: each refresh grows the state directory by about 0.8 MB and per-refresh latency grows from 0.44 s to 12.6 s over 30 refreshes while plain transactions stay flat; under load late refreshes returned Uncertain after sending
```
