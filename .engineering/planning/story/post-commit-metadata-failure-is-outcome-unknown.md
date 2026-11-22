---
format: aep.planning-md/3
id: story:post-commit-metadata-failure-is-outcome-unknown
kind: story
status: draft
title: A failure after a durable metadata commit is reported outcome_unknown
tags:
- cr-014
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-014 (S3): A failure after a durable metadata commit is reported as a definite `MetadataUnavailable`.

- Citations: `crates/connectors-host/src/local/metadata/er.rs:4115-4122`, `:4137-4144`, `:4197-4207`; `crates/connectors-host/src/local/mutations.rs:112-113`, `:673-678`.
- What happens: the caller is told nothing was stored while the row is durable; an attempt left `dispatching` is later marked `indeterminate` although nothing was sent. Fails closed.
- Status: inferred.
- Fix: map post-receipt failures to `OutcomeUnknown`.

- Review's story note: new.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-014. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
