---
format: aep.planning-md/3
id: story:invoke-admission-queues-under-load
kind: story
status: draft
title: Parallel invokes under load are admitted in turn, not refused timeout
relations:
- decomposes: epic:review-20261123-findings
revision: 2
---
## Outcome

Under load, an invoke that cannot be admitted within its wait waits in a queue instead of being
refused `timeout` at `admission`, or its refusal carries a retry hint with a delay; a burst of
parallel reads on one instance completes.

## Observed

Consumer: dev-operator (session bf-runbook), run `refresh-read-20261123-032902`, connectors
0.65.2, host load 70-80: `issues.search` answered `metadata_unavailable` at `observation`, then
`timeout` at `admission` twice; 4 parallel `issue.comments` calls refused 14 of 16 with `timeout`
at `admission`. Called as `connectors --output json invoke jira/<instance> <op> --input-json ...`.
The main store was 650 MB (issue #267), which every CLI call replays.

## Acceptance

- On a store of the size the read-cost test uses, 16 parallel reads on one instance at a load
  the test sets all answer, none `timeout` at `admission`; or each refusal names a retry delay.
- Measured once against 0.74.x with a fresh store before any change, to separate #267 from
  admission.
