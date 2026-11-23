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
revision: 3
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

Question D5 (2026-11-23, with host-operator for Timo; a change of architecture, so no default):
which of the ways below. The acceptance lines under it are written for "removed from the store"
and are rewritten once D5 is answered.

Read from the code by the scoper on 2026-11-23 (connectors 0.75.0, Entity Runtime 0.32.0 and
tag 0.33.0, Eventlog 0.9.4; paths under `crates/connectors-host/src/local/`):

- One settled read is one `connectors.credential_evidence.ReadUse` subject with 4 events
  (Captured, Dispatched, Released, Expired) and about 15 blobs (`registry/use_and_retirement.rs:246-313`,
  `metadata/er.rs:4540-4542`, `:4641-4650`, `:4749-4762`). A write takes a read use too
  (`owner/mutation/execution.rs:203`, `:446`).
- No production code reads a settled read use: an Expired one is not projected to SQL
  (`metadata/er.rs:3198-3219`); the SQL readers ask for open or unexpired rows
  (`registry/use_and_retirement.rs:25`, `:285`, `:361`, `:447`; `registry/revalidation.rs:160`);
  `read_history` has two production callers, both verification (`metadata/er.rs:2531`, `:2593`).
- Every open replays every subject, and the in-memory baseline keeps each terminal row including
  Expired read uses (`metadata/er.rs:2396-2440`, `:3011-3043`); every persist loops the whole
  baseline (`:3998-4035`, `:4384-4405`, `:5076-5092`). That this is the cause of the time and RSS
  growth is inferred, not measured.
- Why the record is durable: "Expiry deletes these transient SQL rows while their ER terminal
  history remains available for audit and stale-token refusal" (`metadata/er.rs:3214-3215`); the
  root decision in story:ess-evolution-er-metadata-adoption (`:240-250`): "model concrete ReadUse
  retention"; `docs/local-er-metadata.md:129-132`. No decision, comment or test says a read use
  must survive an owner restart; `docs/local-connection-registry.md:214-220`: "Restart does not
  resume a captured provider operation."
- Entity Runtime 0.32.0 and 0.33.0 have no delete, truncate, compact or prune
  (`entity-eventlog/src/facade.rs:157-474`), and refuse a store with a missing blob or a redacted
  event (`adapter.rs:4026`, `:4409-4412`, `:2926-2928`, `:4616-4617`). One connectors batch mixes
  the expiry of one use, the capture of the next, a clock advance and the writer marker
  (`metadata/er.rs:4096-4109`), so cutting one chain breaks batches of surviving subjects
  (inferred). A durable checkpoint removes nothing from the blob or event tables.
- An import path exists (`import_legacy`, `facade.rs:393`; `provision_with_lock`,
  `metadata/er.rs:2302-2365`).

The ways:

- A. Read uses are no longer recorded: open uses live in the owner process. connectors only. An
  existing store is not rewritten and stays readable by older builds if the `ReadUse` definition
  stays (`metadata/er.rs:985-1014`, `:1043-1053`); it does not shrink. Changes: the root decision
  above; a CLI revoke no longer releases the owner's undispatched use through the recorded row
  (`registry/lifecycle.rs:469`), dispatch still re-checks state, fence, generation and material
  (`registry/use_and_retirement.rs:295-297`). Surface: `registry/use_and_retirement.rs`,
  `reuse.rs`, `revalidation.rs`, `lifecycle.rs`, `refresh.rs`, `registry.rs`, the command mapping
  in `metadata/er.rs`, the capture and dispatch call sites in `owner/`.
- B. A removal operation in Entity Runtime first (none exists), then compaction here. One-way for
  an existing store, so by decision-blocker:store-checkpoints-one-way it needs an explicit command.
- C. The owner writes a new store from the terminal rows and swaps the file. connectors only.
  Must carry subjects without an SQL row (`StoreWriter`, `RefreshAttempt`, `AccessTokenIssue`) and
  pass the legacy digest check (`metadata.rs:1039-1042`, inferred). One-way unless the old file
  is kept.

Not determined: what an audit of past reads needs and for how long (no requirement found in
code, docs or planning); whether a capture path runs in a CLI process without an owner.

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
