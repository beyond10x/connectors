---
format: aep.planning-md/3
id: story:standing-grant-slot-released-before-dispatch
kind: story
status: draft
title: A standing grant's hourly slot is released when a keyed write aborts before dispatch
tags:
- cr-015
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-015 (S3): A standing grant's hourly slot stays consumed when a keyed write aborts before dispatch.

- Citations: `crates/connectors-host/src/server/v1alpha2.rs:654-664`; release only at `:382-384`.
- Status: verified at these sites. Fails closed.
- Fix: release on every pre-dispatch abort.

- Review's story note: new.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-015. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
