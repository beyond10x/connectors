---
format: aep.planning-md/1
id: review-result:approval-keys-design-round-1
kind: review-result
status: active
title: Approval keys design review round 1
relations:
- reviews: story:local-approval-keys
revision: 1
---
needs-revision

story:local-approval-keys — its selected contract imports credential deletion guards requiring terminal acquisition and 24-hour retention without binding those predicates to the signing-key lifecycle, so specify signing-key reclamation separately from the shared custody backend guarantees — .engineering/planning/story/local-approval-keys.md:24; contracts/service/approval-issuers.md:83; docs/local-secret-service.md:103

Read all eight artifact bodies through `aep plan artifact show`, the role/rubric, relations, graph, validation, issuer contract/model and referenced custody contracts; traversed 61 reachable declared edges beyond the set and ten prerequisite/blocking edges, finding no cycles.

Runtime custody, locking correctness and provider acceptance remain outside this design review. Validation returned `valid` with 114 historical findings-block notices. Sonnet was unavailable; the inherited model was substituted.

```findings
- file: .engineering/planning/story/local-approval-keys.md
  line: 24
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: its selected contract imports credential deletion guards requiring terminal acquisition and 24-hour retention without binding those predicates to the signing-key lifecycle, so specify signing-key reclamation separately from the shared custody backend guarantees
```

