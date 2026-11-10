---
format: aep.planning-md/3
id: story:definition-lower-leaves-no-mixed-output
kind: story
status: draft
title: definition lowering leaves no new bundle beside an old selection on a late failure
tags:
- cr-010
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-010 (S3): `definition::lower` leaves mixed output on a late failure.

- Citations: `crates/connectors-catalog/src/definition.rs:3626-3632`; doc comment `:3515-3519`.
- What happens: the bundle and index are written, then a later plain write fails; new bundle beside old selection.
- Status: verified.
- Fix: stage and rename last, or correct the doc.

- Review's story note: new.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-010. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
