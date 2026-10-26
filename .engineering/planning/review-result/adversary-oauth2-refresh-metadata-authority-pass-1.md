---
format: aep.planning-md/3
id: review-result:adversary-oauth2-refresh-metadata-authority-pass-1
kind: review-result
status: active
title: 'U2 adversary pass 1: oauth2-refresh-metadata-authority'
relations:
- reviews: story:oauth2-refresh-metadata-authority
revision: 1
---
unit: story:oauth2-refresh-metadata-authority, 95b65bc0a plus an uncommitted adversary test file
verdict: red
cases: executed 291→296, red 4
origin: introduced 0, pre-existing 2, undecided 1 (adversary)
reviewer: aep:adversary (opus), pass 1, 2026-10-26

```findings
- file: ess/domains/refresh.yaml
  line: 114
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: ReserveRefresh `reserved` sets no fields, so the committed definitions let the host record any source generation, owner, ownership token and binding revision, and the gate's suite passes with placeholders instead of the input values
- file: crates/connectors-build/src/metadata_conformance.rs
  line: 918
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: pre-existing
  message: emit writes ess-mutation-manifest/2, which ESS 0.68.0 refuses for every sets-drop, precedence-swap or emit-swap mutant, so the emission the acceptance names cannot run (also so on d2842557d)
- file: ess/domains/oauth2.yaml
  line: 144
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: undecided
  message: issued-current lowers refresh_attempt_id to an optional slot the host fills, so the definitions accept an issue naming a refresh it never ran; only a constructed caller supplies it today
```
