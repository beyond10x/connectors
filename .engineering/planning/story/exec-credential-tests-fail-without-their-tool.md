---
format: aep.planning-md/3
id: story:exec-credential-tests-fail-without-their-tool
kind: story
status: draft
title: Kubernetes exec-credential security tests fail when their tool is missing
tags:
- cr-023
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-023 (S3): Kubernetes exec-credential security tests pass without checking when a tool is missing.

- Citations: `adapters/kubernetes/tests/exec_credential_security.rs:415-418`; grep hits at `:179`, `:305`, `:340`, `:503`, `:557`, `:601`.
- Status: line 415 verified.
- Fix: fail unless an explicit opt-out is set.

- Review's story note: new.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-023. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
