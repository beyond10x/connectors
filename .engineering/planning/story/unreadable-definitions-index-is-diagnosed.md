---
format: aep.planning-md/3
id: story:unreadable-definitions-index-is-diagnosed
kind: story
status: draft
title: An unreadable definitions index is diagnosed on configuration load
tags:
- cr-016
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-016 (S3): An unreadable definitions index is silently ignored on configuration load.

- Citations: `crates/connectors-host/src/local/definitions.rs:369-371` (`if let Ok(entries) = entries(paths) { config.hold(entries); }`).
- What happens: every definition-backed instance disappears with no diagnostic.
- Status: verified at this site; callers not traced.
- Fix: record a diagnostic or propagate.

- Review's story note: new.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-016. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
