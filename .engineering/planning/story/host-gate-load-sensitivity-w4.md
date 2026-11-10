---
format: aep.planning-md/3
id: story:host-gate-load-sensitivity-w4
kind: story
status: draft
title: More host and adapter tests assert on events, not elapsed time
tags:
- cr-024
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-024 (S3): Load-sensitive wall-clock bounds and fixed sleeps in tests.

- Citations: `crates/connectors-host/src/local/metadata/deadline.rs:134-138`; `owner/admission_tests.rs:84`, `:105`, `:133`, `:248`, `:293`; `owner/transport.rs:3462`; `owner/supervisor.rs:2736`, `:2797`; `runtime/bounded.rs:242-249`; `adapters/catalog/tests/git_relay.rs:1071`; `adapters/catalog/tests/security_git_relay.rs:782` (fails open); `adapters/catalog/tests/local_runtime/oauth2_refresh.rs:684-693`.
- Status: reviewers' citations, not re-opened by me; flakiness inferred.
- Fix: assert on events, not elapsed time.

- Review's story note: new instances of a known class (`story:host-gate-load-sensitivity-w3`, active).

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-024. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
