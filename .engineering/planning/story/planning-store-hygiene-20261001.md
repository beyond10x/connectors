---
format: aep.planning-md/3
id: story:planning-store-hygiene-20261001
kind: story
status: active
title: Close shipped release plans and a duplicate story
relations:
- decomposes: epic:tech-debt-review-20260930
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: .engineering/planning/release-plan/connectors-0-15-0.md
- confidence: cited
  path: .engineering/planning/release-plan/gitlab-v020.md
- confidence: cited
  path: .engineering/planning/story/registry-replay-test-flaky-under-load.md
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T07:32:48Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-01T07:32:48Z", actor: "human:timo", revision: 4}
---
## Defect

- `release-plan:connectors-0-15-0` and `release-plan:gitlab-v020` are `active`; tags `v0.15.0` and
  `v0.2.0` exist (`git tag -l`).
- `story:registry-replay-test-flaky-under-load` (draft, filed 2026-09-30) describes the failure that
  `story:grown-store-replay-test-flake` fixed (implemented on `origin/main`).
- `aep plan artifact validate` lists review-results recorded with no outcome;
  `story:archive-closed-review-results` (draft since 2026-09-15) owns those and is not this story.

## Change

Through `aep plan artifact move` only: each release plan to `implemented` with a body line naming
its tag and peeled commit; the duplicate story to `archived` with a line naming the story that
fixed it.

## Scope

- `.engineering/planning/release-plan/connectors-0-15-0.md` — cited.
- `.engineering/planning/release-plan/gitlab-v020.md` — cited.
- `.engineering/planning/story/registry-replay-test-flaky-under-load.md` — cited.

## Acceptance

- `aep plan artifact list --kind release-plan --status active` returns neither plan.
- `story:registry-replay-test-flaky-under-load` is `archived` and its body names
  `story:grown-store-replay-test-flake`.
- `aep plan artifact validate` prints `valid`.
