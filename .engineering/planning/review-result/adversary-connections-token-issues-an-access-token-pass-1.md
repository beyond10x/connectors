---
format: aep.planning-md/3
id: review-result:adversary-connections-token-issues-an-access-token-pass-1
kind: review-result
status: active
title: 'U5 adversary pass 1: connections-token-issues-an-access-token'
relations:
- reviews: story:connections-token-issues-an-access-token
revision: 1
---
unit: story:connections-token-issues-an-access-token, 0928d2f53 plus an uncommitted adversary test file
verdict: red
cases: executed 714→717, red 3
origin: introduced 4, pre-existing 0, undecided 0
reviewer: aep:adversary (opus), pass 1, 2026-10-26

```findings
- file: crates/connectors-host/src/local/owner/refresh/issue.rs
  line: 91
  category: concurrency
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the refresh decision is read outside the refresh reservation, so concurrent issues send a second exchange (2 to 3 per trial in 2 to 5 of 8 trials), against auth acquisition §4.4 item 2
- file: crates/connectors-host/src/local/owner/refresh/issue.rs
  line: 157
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a scope refusal surfaces as code insufficient_scope, which is not a connectors.cli.FailureCode; the CLI contract table names not_granted
- file: crates/connectors-host/src/local/owner/refresh/issue.rs
  line: 150
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: an issue inside the lead for a confidential-client connection is refused as invalid_input, a usage error at stage arguments that is in neither refusal table
- file: crates/connectors-host/src/local/owner/transport.rs
  line: 1
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the owner checks the caller deadline only before the issue, so a joined refresh outlasting the client 30 s can record a revealed or library issue that was never delivered (from reading, not run)
```
