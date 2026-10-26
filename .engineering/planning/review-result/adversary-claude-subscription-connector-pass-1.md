---
format: aep.planning-md/3
id: review-result:adversary-claude-subscription-connector-pass-1
kind: review-result
status: active
title: 'U8 adversary pass 1: claude-subscription-connector'
relations:
- reviews: story:claude-subscription-connector
revision: 1
---
unit: story:claude-subscription-connector, a369955f3 plus uncommitted adversary tests
verdict: red
cases: executed 909→912, red 3
origin: introduced 2, pre-existing 1, undecided 0
reviewer: aep:adversary (opus), pass 1, 2026-10-26

```findings
- file: website/docs/connectors/claude-subscription.md
  line: 222
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the guide pinned instance permits only claude.subscription while it tells the reader to configure and connect claude.subscription-pasted, which the owner refuses as Forbidden
- file: crates/connectors-oauth2/src/client.rs
  line: 240
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: scope_in_refresh with empty requested scopes sends an empty scope member, which RFC 6749 section 3.3 does not allow; no shipped declaration reaches it
- file: crates/connectors-oauth2/src/receiver.rs
  line: 156
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: the pasted line is split on the last #, so code#junk#state is accepted with code code#junk where the provider split takes junk as the state and refuses
```
