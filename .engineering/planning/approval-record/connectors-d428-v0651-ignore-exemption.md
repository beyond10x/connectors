---
format: aep.planning-md/3
id: approval-record:connectors-d428-v0651-ignore-exemption
kind: approval-record
status: approved
title: 'D428: the two added timing ignores in v0.65.1 are exempt from the release check'
relations:
- decides: story:cold-owner-start-verifies-once
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-03T21:32:06Z", actor: "human:timo", revision: 2}
---
# D428: the two added timing ignores in v0.65.1 are exempt from the release check

## Decision

The orchestrator, 2026-11-03, D428: "I exempt the two N12 FAILs for this tag. ... Why:
cold_owner_start.rs:317 and store_cost_tests.rs:682 are timing measurements ignored with a stated reason.
They belong to the repo's `connectors-build ignored` family (ignored.rs:146, :414), so no failing test is
hidden. verify-release.sh stays unchanged."

## What it covers

`verify-release.sh connectors v0.65.1` reports "release diff adds #[ignore]" at
`apps/connectors/tests/cold_owner_start.rs:317`
(`a_cold_describe_and_read_of_a_large_store_answer_within_60_seconds`, `#[ignore = "timing measurement; run
explicitly on a release build"]`) and `crates/connectors-host/src/local/registry/store_cost_tests.rs:682`
(`fresh_open_passes_of_a_blob_heavy_store`, `#[ignore = "timing measurement; run explicitly"]`), both added by
`16162e6f5` in wave D417 for `story:cold-owner-start-verifies-once`. The exemption holds for tag `v0.65.1`
(`eef7f6616`, peeled `413c60efc`) only; any other FAIL of that check still stops the announcement.
