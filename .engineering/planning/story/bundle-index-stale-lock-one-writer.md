---
format: aep.planning-md/3
id: story:bundle-index-stale-lock-one-writer
kind: story
status: draft
title: Stale-lock removal never lets two writers hold the bundle index lock
tags:
- cr-008
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- informed_by: story:catalog-index-concurrent-writers
revision: 1
---
## Defect

Code review finding CR-008 (S3): Stale-lock removal lets two writers hold the bundle index lock.

- Citations: `crates/connectors-catalog/src/bundle.rs:212-250`, `:252-256`.
- What happens: after a crashed writer, two writers both see the lock stale; B removes A's fresh lock; one index row is lost.
- Status: verified.
- Fix: rename before unlink, or `flock` a persistent file.

- Review's story note: new (`story:catalog-index-concurrent-writers`, implemented, claims otherwise).

This contradicts `story:catalog-index-concurrent-writers` (implemented), which claims the bundle index holds one writer; that story defers reader atomicity to `decision-blocker:catalog-pipeline-write-atomicity`, which does not exist in the store. The decision is missing and is needed before this is fixed.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-008. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
