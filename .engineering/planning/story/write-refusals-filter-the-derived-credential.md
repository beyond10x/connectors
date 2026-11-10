---
format: aep.planning-md/3
id: story:write-refusals-filter-the-derived-credential
kind: story
status: draft
title: Write refusals are filtered against the derived credential as reads are
tags:
- cr-013
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-013 (S3): Write refusals are not filtered against the derived credential.

- Citations: `adapters/catalog/src/local.rs:3037-3043` (read path filters), `:3070-3091` and `adapters/catalog/src/served.rs:698-734` (write path does not).
- What happens: an upstream 4xx echoing a derived token is not dropped on writes. `crates/connectors-core/src/reason.rs:66-86` withholds token-shaped text, so exposure is low.
- Status: inferred.
- Fix: pass the derived value into the write path.

- Review's story note: new.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-013. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
