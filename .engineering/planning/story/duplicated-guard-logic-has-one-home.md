---
format: aep.planning-md/3
id: story:duplicated-guard-logic-has-one-home
kind: story
status: draft
title: Duplicated receiver, lease and header-bound guard logic has one home
tags:
- cr-021
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-021 (S3): Duplicated guard logic that can drift.

- Citations: `crates/connectors-host/src/local/oauth.rs:234-530` against `crates/connectors-oauth2/src/receiver.rs`; `approval_keys.rs:664-747` against `approval_policy.rs:387-470`; `crates/connectors-host/src/http.rs:1090-1107`, `:1194-1200` lack the header bound the other four call sites have.
- Status: reviewer's reading; I did not diff the copies.
- Fix: one receiver, one `Lease`, one header helper.

- Review's story note: new.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-021. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
