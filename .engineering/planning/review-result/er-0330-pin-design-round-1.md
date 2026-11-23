---
format: aep.planning-md/3
id: review-result:er-0330-pin-design-round-1
kind: review-result
status: active
title: Design critic, round 1, story:entity-runtime-0330-records-name-definition-by-digest
relations:
- reviews: story:entity-runtime-0330-records-name-definition-by-digest
revision: 1
---
verdict: needs-revision

1. story:read-cost-test-reports-bytes-per-read, split surface. Both stories add the same print to `read_invoke_cost_by_store_size`, and no edge orders them. The read-cost story's acceptance (`.engineering/planning/story/read-cost-test-reports-bytes-per-read.md:26-29`) says the test prints bytes appended per read. The pin story's acceptance 4 (`.engineering/planning/story/entity-runtime-0330-records-name-definition-by-digest.md:102-106`) adds `blob_bytes_per_invoke` to the same test. The pin story also needs the number to measure before and after, which is the consumer's gap (the 650 MB store). Accepted change:
   - The pin story owns the print.
   - The read-cost story gets `depends_on story:entity-runtime-0330-records-name-definition-by-digest`.
   - Its first acceptance line shrinks to "keeps the print the pin story added", and it keeps only the bound. The bound is then the post-pin number, which matches its "measured on the commit that adds it" line.
   - This avoids two stories setting the bound constant. `aep plan artifact graph` shows no edge between them today.

2. story:settled-read-uses-leave-the-store, hidden dependency. Its acceptance reads "`read_invoke_cost_by_store_size` (or its successor from the bytes-per-read story)" (`.engineering/planning/story/settled-read-uses-leave-the-store.md:44`). It declares an edge only to the pin story. After finding 1, the read-cost story and this one are siblings that both edit that test function. Accepted change: add `depends_on story:read-cost-test-reports-bytes-per-read`, so the chain is read-cost after pin, then settled-read. Every edge has a stated reason: one record format, one test function. I found no cycle.

3. story:entity-runtime-0330-records-name-definition-by-digest, the `LEVEL` question is decidable now. My decision is no raise, and the story should say so.
   - What 0.74.2 answers: `metadata_unavailable`. Every Entity Runtime open and replay failure is flattened to `Failure::MetadataUnavailable` (`er.rs:2435`, `er.rs:3004`). The writer marker check `er::admit_writer` runs only after `er::open` has replayed the store (`metadata.rs:1084`), so it never sees `er.record/5`.
   - The story's own trigger therefore fires: "Decided after acceptance 5" (`.engineering/planning/story/entity-runtime-0330-records-name-definition-by-digest.md:123-128`) would flip to yes mid-run and stall on a question to host-operator.
   - Why not raise:
     - The 0.29 durable-checkpoint change was also one-way and shipped at 6995a513909 with `LEVEL` still 9. It has been 9 since 02d2c069eeb.
     - Raising it is a 9 to 10 store migration, not a constant. It touches `version == er::LEVEL` at `metadata.rs:386,653,701,995,1208,1229,1277` plus the `schema_migrations` row. `metadata.rs` is not in the story's scope list.
     - Eventlog appends on its own SQLite handle (`metadata.rs:1044-1047`), so a bump cannot be atomic with the first `er.record/5` write.
     - The guard already works for any level above 9 (`metadata.rs:1625-1640`). A later release can bump to 10 and also cover stores this pin wrote.
   - Accepted change: replace that section with "no raise; 0.74.2 answers `metadata_unavailable` (er.rs:2435, 3004)". Keep acceptance 5 as the run that confirms the code and stage. If a named refusal is wanted, it becomes its own story, put to host-operator as lettered options with default A (no raise).

4. story:entity-runtime-0330-records-name-definition-by-digest, an unrecorded precedent. `decision-blocker:store-checkpoints-one-way` (cleared 2026-10-07) refused auto-applied one-way store changes, because they "change an existing user's store irreversibly without an act of its owner". It chose an explicit command and a CHANGELOG entry under Breaking or Migration. This pin does the same thing on the first write, with no opt-in (0.33.0 CHANGELOG: "A new record is framed `er.record/5`"). The pin itself is settled and D3 covers it; the gap is the record. Accepted change:
   - Add `informed_by decision-blocker:store-checkpoints-one-way`.
   - Say in the body that opt-in is impossible here because the record format is the library's.
   - In acceptance 5 (line 108) name the CHANGELOG section as Breaking, with the rotate-the-state-directory path as the rollback.

Entity Runtime 0.33.0 against `er.rs`: no call site must change. I diffed the 0.32.0 and 0.33.0 sources of `entity-core`, `entity-eventlog`, `entity-executor` and `entity-store`.
- Everything `er.rs` imports is unchanged or additive:
  - `RecordedProviderFacade`, `EventlogRecordedStoreOwner`, `EventlogRecordedStoreProvisioner` and `ProvisionAuthority`
  - `BridgeConfig`, `CallWait`, `Authority`, `OpenVerification` and `CapturePolicy`
  - `RecordedEntry`, `StoredRecord`, `HistoryOrigin`, `CompleteStoreSnapshot` and `WriteFailure`
  - `BatchAction`, `CreateRequest` and `ExecuteRequest`
  - `entity-store`'s `asynchronous/types.rs` has no diff.
- The in-memory `DecisionRecord.definition` stays `Option<EntityDefinition>` (`entity-core runtime.rs:143`), so the `remove("definition")` in the test at `er.rs:6651` still holds.
- Nothing in connectors calls `record_comparison_bytes`, `batch_comparison_bytes` or `record_framing`, whose output changed.
- Behaviour changes that could still refuse at open or registration:
  - `Registry::validate_all` now checks a `ref` in a `service/1` create argument. I found none in the 28 definitions of `entity-runtime-definitions.json`.
  - Map-key text is stricter. All connectors map keys are `string`.
  - Replay now recomputes a creation at event 0. Only the old-store fixtures guard this, which is acceptance 3 plus the 0.26.0 fixture.
- I did not run `cargo check` or `cargo build`. The no-change conclusion comes from reading source, not compiling.

What I read: 4 stories, `relations`, and `graph`. `validate` printed only the stale review-result notices (existing, not findings). I walked the 9 declared edges of the four stories; the only `depends_on` is settled-read to pin, and there is no cycle. I traced 4 acceptance reads to the test and fixture surfaces; 1 of those has an edge.

Not established or out of lane:
- The inputs.json and fixture-directory collisions are with stories outside the set (`story:test-binary-per-package`, the ESS-pin units). That is parallel-safety's lane.
- story:connections-list-answers-through-the-owner has a stale out-of-scope line about D3 still being pending, and its acceptance 1 reads "the size the read-cost test uses". Neither changes my verdict.

```findings
- file: .engineering/planning/story/read-cost-test-reports-bytes-per-read.md
  line: 26
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "both this story and the pin story (entity-runtime-0330-records-name-definition-by-digest.md:102) add the bytes-per-read print and own the bound on read_invoke_cost_by_store_size with no edge; the pin story owns the print, this story gets depends_on the pin story and keeps only the bound"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 44
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance reads the test function the read-cost story reshapes ('or its successor') and declares an edge only to the pin story; add depends_on story:read-cost-test-reports-bytes-per-read"
- file: .engineering/planning/story/entity-runtime-0330-records-name-definition-by-digest.md
  line: 123
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the LEVEL question is deferred to the run of acceptance 5, but the code already answers metadata_unavailable for er.record/5 (er.rs:2435, 3004; admit_writer at metadata.rs:1084 runs after replay), which fires the story's own raise condition; decide no raise now (precedent 6995a513909 kept LEVEL 9; a bump is a 9 to 10 migration in metadata.rs outside scope) and move a named refusal to its own story"
- file: .engineering/planning/story/entity-runtime-0330-records-name-definition-by-digest.md
  line: 108
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the body does not cite decision-blocker:store-checkpoints-one-way, which refused auto-applied one-way store changes; add informed_by to it, state why opt-in is impossible here, and name the CHANGELOG section as Breaking with the rotate-the-state-directory path as the rollback"
```
