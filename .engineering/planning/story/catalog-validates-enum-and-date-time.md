---
format: aep.planning-md/3
id: story:catalog-validates-enum-and-date-time
kind: story
status: draft
title: The catalog engine refuses values outside an enum or a date-time format
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/catalog/generated/bundles/confluence.bundle.json
- confidence: inferred
  path: adapters/catalog/generated/bundles/gitlab.bundle.json
- confidence: inferred
  path: adapters/catalog/generated/bundles/google-calendar.bundle.json
- confidence: inferred
  path: adapters/catalog/generated/bundles/google-drive.bundle.json
- confidence: inferred
  path: adapters/catalog/generated/bundles/google-gmail.bundle.json
- confidence: inferred
  path: adapters/catalog/generated/bundles/google-slides.bundle.json
- confidence: inferred
  path: adapters/catalog/generated/bundles/hubspot.bundle.json
- confidence: inferred
  path: adapters/catalog/generated/bundles/index.json
- confidence: inferred
  path: adapters/catalog/generated/bundles/jira.bundle.json
- confidence: cited
  path: adapters/catalog/src/lib.rs
- confidence: cited
  path: adapters/catalog/tests/gitlab_commit_reads_adversary.rs
- confidence: inferred
  path: adapters/catalog/tests/parameter_types.rs
- confidence: cited
  path: crates/connectors-catalog/src/authored.rs
- confidence: cited
  path: crates/connectors-catalog/src/inventory.rs
- confidence: inferred
  path: crates/connectors-catalog/tests/template.rs
- confidence: cited
  path: docs/local-catalog-provider.md
revision: 3
---
## Observed

2026-10-01, adversary pass on story:catalog-gitlab-commit-reads (review-result:adversary-gitlab-commit-reads-pass-1):
values outside a pinned enum (`commits.list order=newest`, `projects.list order_by=newest`) and strings that are not
date-times for `format: date-time` parameters (`since=yesterday`, `projects.list last_activity_after=yesterday`) are
sent upstream instead of refused. Pre-existing: `projects.list` shipped before this unit. The guide says a string
parameter is "a string as any string" while its table labels `since`/`until` "date-times".

## Acceptance

- A query or path parameter with an `enum` in the pinned document refuses other values as `invalid_input` before any
  request; one with `format: date-time` refuses a value that is not RFC 3339.
- The adversary cases at `gitlab_commit_reads_adversary.rs:153,167,253,262` assert the refusals.
- The guide states what the engine checks.
