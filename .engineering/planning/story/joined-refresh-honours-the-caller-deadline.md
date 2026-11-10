---
format: aep.planning-md/3
id: story:joined-refresh-honours-the-caller-deadline
kind: story
status: draft
title: A joined refresh waits no longer than the caller's deadline
tags:
- cr-017
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-017 (S3): A joined refresh ignores the caller's deadline.

- Citations: `crates/connectors-host/src/local/owner/refresh.rs:291-292`, `:43`.
- What happens: waits up to 60 s holding one of 32 client slots.
- Status: verified.
- Fix: wait until min(deadline, `JOIN_WAIT`).

- Review's story note: new.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-017. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
