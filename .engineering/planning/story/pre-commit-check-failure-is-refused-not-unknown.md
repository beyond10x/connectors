---
format: aep.planning-md/3
id: story:pre-commit-check-failure-is-refused-not-unknown
kind: story
status: draft
title: A write failing its last pre-commit check is recorded refused, not unknown
tags:
- cr-018
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-018 (S3): A write failing its last pre-commit check is recorded `Unknown` although nothing was sent.

- Citations: `crates/connectors-host/src/local/owner/mutation/execution.rs:500-520`; `crates/connectors-host/src/local/clock/timer.rs:111-117`.
- Status: inferred; may be deliberate.
- Fix: retry the clock bracket; classify cancel-before-commit as refused.

- Review's story note: new.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-018. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
