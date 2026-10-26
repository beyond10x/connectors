---
format: aep.planning-md/3
id: review-result:adversary-chatgpt-subscription-connector-pass-1
kind: review-result
status: active
title: 'U7 adversary pass 1: chatgpt-subscription-connector'
relations:
- reviews: story:chatgpt-subscription-connector
revision: 1
---
unit: story:chatgpt-subscription-connector, 9875002bc plus uncommitted adversary tests
verdict: red
cases: executed 875→877 and journeys 3→4, red 1
origin: introduced 2, pre-existing 0, undecided 0
reviewer: aep:adversary (opus), pass 1, 2026-10-26

```findings
- file: website/docs/connectors/chatgpt.md
  line: 162
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the guide and the dialect (MAX_WAIT 15 min, MAX_INTERVAL 600 s) promise a 15-minute device sign-in, but the host grant deadline of 240 s ends it; a 241 s interval times out 59 µs after the code is shown
- file: adapters/catalog/src/local.rs
  line: 1271
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: no case exercised the oauth2_grant-only and leading-slash guards on an id_token subject_pointer; two green adversary cases now pin them
```
