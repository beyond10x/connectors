---
format: aep.planning-md/3
id: story:catalog-validates-enum-and-date-time
kind: story
status: draft
title: The catalog engine refuses values outside an enum or a date-time format
relations:
- serves: vision:independent-contract-adapters
revision: 1
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
