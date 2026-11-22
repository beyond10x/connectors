---
format: aep.planning-md/3
id: story:adapter-child-answers-timeout-for-a-late-frame
kind: story
status: draft
title: The adapter child answers timeout for a late frame and keeps running
tags:
- cr-019
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-019 (S3): The adapter child exits instead of answering `timeout` for a late frame.

- Citations: `crates/connectors-host/src/local/runtime/server.rs:327-330`.
- Status: inferred.
- Fix: reply `Failed { Timeout }` and continue.

- Review's story note: new.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-019. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
