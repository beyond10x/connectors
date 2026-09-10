---
format: aep.planning-md/1
id: review-result:gitlab-validation-acceptance-round-1
kind: review-result
status: active
title: MR validation acceptance round 1
relations:
- reviews: story:gitlab-mr-validation
- reviews: initiative:complete-local-connectors
revision: 1
---
needs-revision

story:gitlab-mr-validation — the acceptance joins passing-result and negative-result cases that can pass independently, so select one observable transition and retain the other cases as verification prerequisites — .engineering/planning/story/gitlab-mr-validation.md:32

story:gitlab-mr-validation — the positive acceptance requires checks_passed=true from matching SHA and successful pipeline alone, omitting the opened, non-draft and mergeable conditions that the owning contract also requires — .engineering/planning/story/gitlab-mr-validation.md:32; adapters/gitlab/contracts/merge-requests/v1alpha1/validation.md:20–33

Read: nine complete artifacts through `aep plan artifact show`: the supplied story and initiative plus seven sibling stories discovered through `aep plan artifact graph --format json`; also read artifact kinds, story/initiative lifecycles, the native validation contract and its ESS values.

Could not establish: runtime or dedicated sandbox success; this review assesses the added acceptance and preserves historical milestones.

```findings
- file: .engineering/planning/story/gitlab-mr-validation.md
  line: 32
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: the acceptance joins passing-result and negative-result cases that can pass independently, so select one observable transition and retain the other cases as verification prerequisites
- file: .engineering/planning/story/gitlab-mr-validation.md
  line: 32
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: the positive acceptance requires checks_passed=true from matching SHA and successful pipeline alone, omitting the opened, non-draft and mergeable conditions that the owning contract also requires
```
