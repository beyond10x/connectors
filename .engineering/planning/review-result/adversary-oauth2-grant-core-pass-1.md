---
format: aep.planning-md/3
id: review-result:adversary-oauth2-grant-core-pass-1
kind: review-result
status: active
title: 'U1 adversary pass 1: oauth2-grant-core'
relations:
- reviews: story:oauth2-grant-core
revision: 1
---
unit: story:oauth2-grant-core (worktree conn-oauth-u1-oauth2-core, commit 50facde24 plus the adversary's uncommitted test files)
verdict: red
cases: executed 28→34, red 4
origin: introduced 7, pre-existing 0, undecided 0 (adversary)
reviewer: aep:adversary (opus), pass 1, 2026-10-25

```findings
- file: crates/connectors-oauth2/src/client.rs
  line: 304
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a server-sent device interval is accepted unbounded and panics the client at device.rs:52 (ThreadWaiter) and client.rs:344 (slow_down)
- file: crates/connectors-oauth2/src/transport.rs
  line: 7
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: derived Debug on Request prints the body bytes, exposing the refresh token, client secret, code and verifier to any transport that logs it
- file: crates/connectors-oauth2/src/receiver.rs
  line: 92
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a connection whose head completes after the decision is dropped unanswered, not answered 400 as documented and as the host Drain does
- file: crates/connectors-oauth2/src/receiver.rs
  line: 327
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: no original case sends an error redirect, so mapping access_denied to Rejected survived the suite
- file: crates/connectors-oauth2/src/client.rs
  line: 331
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the device code's elapsed-clock expiry clause is reached by no original case and its removal survived the suite
- file: crates/connectors-oauth2/src/declaration.rs
  line: 120
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: rotation is read nowhere, so a non_rotating declaration accepts a replaced refresh token despite the NonRotating doc
- file: crates/connectors-build/src/gate.rs
  line: 74
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the oauth2 boundary step's cargo tree misses target-specific and feature-gated connectors-* dependencies
```
