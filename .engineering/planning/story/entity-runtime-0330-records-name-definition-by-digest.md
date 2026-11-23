---
format: aep.planning-md/3
id: story:entity-runtime-0330-records-name-definition-by-digest
kind: story
status: draft
title: 'connectors pins Entity Runtime 0.33.0: a record names its definition by digest'
relations:
- decomposes: epic:review-20261123-findings
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

connectors builds on Entity Runtime 0.33.0, so a recorded entry names its entity definition by
digest (`er.record/5`) and a read invoke appends about a fifth of the bytes it appends on 0.74.

## Source

Issue #267, wanted item 2. Timo's pick D3 of 2026-11-23 (about 04:57 CET, relayed by
host-operator): "D: A then B (Recommended)", A being this record format, B compaction of settled
read records (a separate story, after this one). Entity Runtime side:
beyond10x/entity-runtime#94, design fixed by that repository's session on 2026-11-23:

- `er.record/5` is a decision record whose `definition` is `sha256:<64 hex>`; the definition is
  stored once per tenant as an `er.definition/1` blob, in the same append group as the first
  record naming it.
- `er.record/1` to `/4` decode as in 0.32.0; old bytes, digests, blob keys and chains are
  unchanged, so a store written by connectors 0.74 opens without a rewrite.
- A record whose digest the store does not hold, or whose blob does not hash to it, is refused
  `ProviderIntegrity`.
- Estimated from #267: a single-record commit goes from about 22 KB to about 4 KB, plus about
  9 KB once per definition.

Measured on connectors 0.74.0 (Entity Runtime 0.32.0): 103 reads left 11.0 MB of blobs, 10.4 MB
of it in 673 blobs that embed a definition, about 107 KB per read
(https://github.com/beyond10x/connectors/issues/267#issuecomment-7091114772).

## Acceptance

- `Cargo.toml` and `Cargo.lock` select Entity Runtime 0.33.0; the gate passes.
- A store written by connectors 0.74.2 opens on the new build, and a read on it answers: a test
  uses a committed fixture store or one written by the 0.32.0 encoding.
- `read_invoke_cost_by_store_size` (or the test of story bytes-per-read, if it landed first)
  reports bytes appended per read invoke before and after; the number after is at most a third
  of the number before.
- An older connectors cannot open a store that holds `er.record/5`: the CHANGELOG says so under
  the release, and what it answers is stated from a run, not assumed.
- The owner's RSS after 100 reads on a fresh store is reported before and after.

## Out of scope

- Rewriting old records to `er.record/5` to reclaim their bytes: a state directory is rotated
  instead, as operators do today.
- Compaction of settled read records (D3 part B): its own story, after this one.

## Blocked on

The Entity Runtime 0.33.0 release (beyond10x/entity-runtime#94).
