---
format: aep.planning-md/3
id: review-result:adversary-launch-hands-an-oauth2-access-token-pass-1
kind: review-result
status: active
title: 'U6 adversary pass 1: launch-hands-an-oauth2-access-token'
relations:
- reviews: story:launch-hands-an-oauth2-access-token
revision: 1
---
unit: story:launch-hands-an-oauth2-access-token, 7a2c2f281 plus uncommitted adversary tests
verdict: red
cases: executed 3→8 launch journeys, red 2
origin: introduced 2, pre-existing 2, undecided 0
reviewer: aep:adversary (opus), pass 1, 2026-10-26

```findings
- file: apps/connectors/src/local/connections.rs
  line: 204
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a launch whose refresh needs reauthorization answers next_action retry_explicitly through owner_failure, where consumer-launch.md section 9 promises repair_connection
- file: crates/connectors-host/src/local/owner.rs
  line: 953
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: launch admission refuses a grant whose evidence lapsed with unavailable and repair_connection while connections token refreshes the same state, so refresh-on-launch only happens while evidence collected at most 300 s earlier is current
- file: crates/connectors-host/src/local/owner/refresh/tests.rs
  line: 897
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: this test and refresh/tests/issue.rs:376 panic with missing field scope_in_refresh on the integration branch since a369955f3 added the field
- file: crates/connectors-host/src/local/runtime/launch/tests.rs
  line: 516
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the disposable launch journey second registry.begin is refused InstanceConnected since 0ed21e606; test and registry byte-identical between d6b9bb28b and HEAD
```
