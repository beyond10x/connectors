---
format: aep.planning-md/3
id: story:gate-runs-a-dependency-advisory-check
kind: story
status: draft
title: The gate and CI run a dependency advisory check
tags:
- cr-025
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-025 (S3): No dependency advisory check in the gate or CI.

- Citations: `crates/connectors-build/src/gate.rs:110-380`; `.github/workflows/rust-gate.yml:57-65`. The last recorded `cargo audit` is `.engineering/planning/review-result/independent-repository-a-20260908.md:27`.
- Status: absence verified; no advisory claimed against any crate.
- Fix: a `cargo audit` or `cargo deny` step.

- Review's story note: new.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-025. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
