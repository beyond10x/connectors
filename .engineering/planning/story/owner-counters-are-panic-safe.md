---
format: aep.planning-md/3
id: story:owner-counters-are-panic-safe
kind: story
status: draft
title: Owner client and worker counters are released by drop guards
tags:
- cr-022
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-022 (S3): Owner client and worker counters are not panic-safe.

- Citations: `crates/connectors-host/src/local/owner/transport.rs:2053-2058`; `owner/supervisor.rs:1008`, `:1024`.
- Status: inferred; no reachable panic found.
- Fix: drop guards.

- Review's story note: new.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-022. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
