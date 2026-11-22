---
format: aep.planning-md/3
id: story:served-host-header-timeout-and-review-slots
kind: story
status: draft
title: The served host times out header reads and takes no review slot before authentication
tags:
- cr-020
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-020 (S3): Served host has no header-read timeout and takes review slots before authentication.

- Citations: `crates/connectors-host/src/server.rs:605-644`; `crates/connectors-host/src/server/token_review.rs:284-286`.
- Status: inferred.
- Fix: header-read timeout and a per-peer limit.

- Review's story note: new.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-020. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
