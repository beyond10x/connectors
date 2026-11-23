---
format: aep.planning-md/3
id: story:settled-read-uses-leave-the-store
kind: story
status: draft
title: Settled read uses leave the metadata store (compaction, D3 part B)
relations:
- decomposes: epic:review-20261123-findings
- serves: vision:independent-contract-adapters
- depends_on: story:entity-runtime-0330-records-name-definition-by-digest
- depends_on: story:read-cost-test-reports-bytes-per-read
revision: 2
---
## Outcome

The local metadata store stops growing with the number of reads: the history of a settled read
use (released or expired, and named by no attempt that is still open) leaves the store, so an
instance that reads all day keeps a store whose size follows its open work and its connections.

## Source

Issue #267, wanted item 1. Timo's pick D3 of 2026-11-23 (about 04:57 CET, relayed by
host-operator): "D: A then B (Recommended)". A is the record format (the Entity Runtime 0.33.0
pin, its own story); B is this story and starts after A is on `main`.

Measured on connectors 0.74.0
(https://github.com/beyond10x/connectors/issues/267#issuecomment-7091114772): 103 reads left 387
events and 11.0 MB of blobs; one read took 2.5 s at a 3 MB store and 9.9 s at 14 MB; owner RSS
98 MB at 5 MB and 216 MB at 14.7 MB. On 0.73.0 (issue body): 2,710 of 3,159 recorded entries
were `connectors.credential_evidence.ReadUse`. With `er.record/5` the bytes per read fall to an
estimated third (about 3×; story:entity-runtime-0330-records-name-definition-by-digest), and still grow without a bound.

## Open before design

Not decided here; the scoper and the design critic settle it from the code, and a choice that
changes how the store is laid out goes to Timo as a question with options:

- whether settled read uses are removed from the recorded store (compaction, or snapshot and
  truncate, which needs a statement from Entity Runtime about removing records of a chain), or
  read-use fencing stops being recorded durably in the first place;
- what an audit of past reads still needs after removal, and for how long.

## Acceptance

- `read_invoke_cost_by_store_size` (or its successor from the bytes-per-read story) runs N reads,
  lets them settle, and reports the store's size: after the change the size after 1,000 settled
  reads is within a stated factor of the size after 100.
- A read use that is not settled (captured, dispatched and not released, or named by an open
  attempt) is still in the store after compaction, and its read answers: a test.
- The owner's RSS after 1,000 reads is reported before and after.
- A store written by the previous release opens, and its settled history is compacted without a
  step by the operator; what an older build answers on the compacted store is stated from a run.
- Compaction runs inside the owner, off the read path: the time of a read during compaction is
  reported.

## Out of scope

- The record format (D3 part A, the Entity Runtime 0.33.0 pin).
- `connections list`, `describe` and `status` through the owner: its own story.

## Blocked on

The Entity Runtime 0.33.0 pin on `main`.
