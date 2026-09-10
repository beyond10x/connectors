---
format: aep.planning-md/1
id: review-result:approval-keys-acceptance-round-1
kind: review-result
status: active
title: Approval keys acceptance review round 1
relations:
- reviews: story:local-approval-keys
revision: 1
---
needs-revision

story:local-approval-keys — the acceptance joins independently checkable creation/restart, rotation preservation, recovery and authority-refusal outcomes instead of naming one observable transition with the remaining cases as verification prerequisites — .engineering/planning/story/local-approval-keys.md:20

story:local-approval-keys — the acceptance does not distinguish rotation interrupted before publication from interruption after the candidate becomes Active, leaving the expected key state after restart ambiguous — .engineering/planning/story/local-approval-keys.md:20

Read: all 8 requested artifacts through `aep plan artifact show`: initiative:complete-local-connectors and story:local-approval-keys, story:local-approval-binding, story:local-execution-audit, story:local-mutation-ledger, story:persistent-gitlab-journey, story:gitlab-ci-runtime and story:gitlab-mr-reads; also inspected `list`, `graph`, `kinds`, initiative/story lifecycles, issuer contract/model, referenced acceptance scenarios and source paths; `validate` exited 0 with `valid` and existing findings-format notices.

Could not establish runtime correctness through this non-interactive, read-only review; no implementation tests ran. Sonnet was unavailable, so the inherited model substituted. Parent coverage, design and parallel safety remain outside this lane; issuer-key completion alone does not complete the initiative.

```findings
- file: .engineering/planning/story/local-approval-keys.md
  line: 20
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: the acceptance joins independently checkable creation/restart, rotation preservation, recovery and authority-refusal outcomes instead of naming one observable transition with the remaining cases as verification prerequisites
- file: .engineering/planning/story/local-approval-keys.md
  line: 20
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: the acceptance does not distinguish rotation interrupted before publication from interruption after the candidate becomes Active, leaving the expected key state after restart ambiguous
```

