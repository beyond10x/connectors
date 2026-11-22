---
format: aep.planning-md/3
id: story:bundle-index-replacement-atomic-for-readers
kind: story
status: draft
title: A reader never sees a half-replaced bundle index
tags:
- cr-009
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- informed_by: story:catalog-index-concurrent-writers
revision: 1
---
## Defect

Code review finding CR-009 (S3): Bundle index replacement is not atomic for readers.

- Citations: `crates/connectors-catalog/src/bundle.rs:163-169`, `:363-373`, `:378-410`.
- What happens: a `load` during a replacing write sees a truncated index or a digest mismatch. Transient.
- Status: verified.
- Fix: temp-and-rename the index; rename the bundle first.

- Review's story note: partly known. `story:catalog-index-concurrent-writers` line 49 defers to `decision-blocker:catalog-pipeline-write-atomicity`, which does not exist in the store.

This contradicts `story:catalog-index-concurrent-writers` (implemented), which claims the bundle index holds one writer; that story defers reader atomicity to `decision-blocker:catalog-pipeline-write-atomicity`, which does not exist in the store. The decision is missing and is needed before this is fixed.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-009. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
