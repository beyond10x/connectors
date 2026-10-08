---
format: aep.planning-md/3
id: story:approval-target-passive-test-flake
kind: story
status: draft
title: approval_target passive test sees an extra store write under suite load
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Observation

`local::registry::tests::approval_target_is_passive_but_requires_exact_retained_admission` failed once in `cargo test -p connectors-host --locked` on 2026-10-08 at commit 908483d0c (a version-only change from cdb264c34, where it passed): `crates/connectors-host/src/local/registry/tests.rs:61`, `PRAGMA data_version` left 3, right 2. The same commit then passed the test 6 of 6 alone and the whole lib suite (366 passed) 2 of 2, at load 11 to 20.

## Hypothesis (inferred, not verified)

Another connection committed to the fixture store between the two `data_version` reads, so the assertion that the passive path writes nothing is sensitive to a background writer. Which writer is not established.

## Acceptance

- The writer is named from a run that reproduces the failure, and the test either excludes it or the passive path is shown to have written.
- The test passes 50 of 50 full-suite runs under load.
