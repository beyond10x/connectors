---
format: aep.planning-md/3
id: story:read-cost-test-reports-bytes-per-read
kind: story
status: draft
title: The read-cost test reports bytes appended per read invoke
relations:
- decomposes: epic:review-20261123-findings
- serves: vision:independent-contract-adapters
- depends_on: story:entity-runtime-0330-records-name-definition-by-digest
revision: 2
---
## Outcome

`read_invoke_cost_by_store_size` reports the bytes one read invoke appends to the store, and the
test fails when that number passes a bound written beside it.

## Observed

Issue #267 (wanted item 4). Measured 2026-11-23 on connectors 0.74.0: 103 reads left 1,446 blobs
of 11.0 MB, about 107 KB per read; 673 blobs with an embedded entity definition hold 10.4 MB of
it (https://github.com/beyond10x/connectors/issues/267#issuecomment-7091114772). No test reports
this number today, so a change that doubles it passes the gate.

## Acceptance

- `crates/connectors-host/src/local/registry/store_cost_tests.rs`
  `read_invoke_cost_by_store_size` keeps the `blob_bytes_per_invoke` print that
  story:entity-runtime-0330-records-name-definition-by-digest added (this story depends on it).
- The bound is the number measured on the commit that adds it, which is after the Entity Runtime
  0.33.0 pin; the test fails above it.
- The test keeps its place in the lane it runs in today, and its time is reported before and
  after (budget: 60 s per test, 120 s for the test step).

## Out of scope

Lowering the number: issue #267 items 1 and 2 (question D3, 2026-11-23).
