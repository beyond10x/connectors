---
format: aep.planning-md/3
id: review-result:read-uses-in-memory-design-round-2
kind: review-result
status: active
title: Design critic, round 2, story:settled-read-uses-leave-the-store
relations:
- reviews: story:settled-read-uses-leave-the-store
revision: 1
---
verdict: needs-revision

Design 2 holds for the main traces, but it has one blocker (a spurious observation refusal) and a replace condition that is not the property you need. Paths below are under `crates/connectors-host/src/local/`.

## Findings

1. **"New to the static" is undefined and races.** Observation projections are built with the lock released (`metadata.rs:1043-1096`, `Hold::Relocks`).
   - Thread B builds with static S0 holding use u. Thread A then sweeps or releases u under the lock and replaces the static with S1.
   - B's SQL still holds u, which is not in the baseline. Measured against the live static it is "new", and a `PreparedObservation` persist does not admit creation (`er.rs:4765`). B's persist answers `MetadataUnavailable`.
   - The observation fails under ordinary load, with a capture running beside an `admit_read`.
   - Change: compare against the snapshot taken at `build_projection`, stored with the handle. In modes that do not replace the static, drop non-baseline `ReadUse` keys from `desired` and `rebuilt` without admission. Only the replacing mode applies the `creation_stages` checks (Captured, undispatched, unreleased) to keys it did not have at build.

2. **The replace condition names the wrong property.** `relock_observation` sets `concurrent_observation=false` (`metadata.rs:684`), and `persist_with_admission` only needs the lock held (`:851`).
   - A handle opened unlocked and relocked that then calls `persist()` is `Standard` without `concurrent_observation`, but its projection was built before the lock.
   - Today no production caller does this. `transaction_inner` sends every observation to `persist_prepared_observation` (`registry.rs:626`).
   - The `concurrent_observation` clause is dead, because that flag already makes `persist_with_admission` return an error.
   - Change: record at build time whether the lock was held (`activate_er(!release_physical)`). Replace the static only when it was.

3. **The key does not bind to a store, and an orphaned row would make the store unusable.**
   - The key is path, file identity and process. An inode number can be reused after a store is deleted and recreated at the same path.
   - A resurrected row then fails `pragma_foreign_key_check` (`er.rs:3110-3116`) and every open returns `MetadataUnavailable` until the owner exits.
   - Parents are never deleted, because `persist_batch` refuses any non-transient deletion (`:4056`). So the sound case is fine. The foreign-key check only fails for a wrong-store hit.
   - Change: add the authority id (`facade.authority()`) to the key. At build, insert a row only if its connection, generation and version exist in the projection being built. Say what happens when `file` is `None`: no rows read and none stored.

4. **Both `Ok` returns must replace.**
   - The hot read path returns at `er.rs:4102` (`actions.is_empty()`), before the batch.
   - Change: do the replacement in `persist` (`:3890`) after `persist_batch(...)?`, so both returns are covered. Have `persist_batch` return the next rows.

5. **Add a sixth difference: the static is not atomic with the recorded batch.**
   - If a batch commits (for example, publication) and the post-commit check at `:4199` then fails, or `record_expiries` succeeds and the caller's batch fails, the persist returns `Err`. The static is not replaced.
   - So release-by-connection and release-by-version effects (`lifecycle.rs:226`, `refresh.rs:422`, `revalidation.rs:254`) are lost for those rows. Today they commit atomically with the publication.
   - This is safe for dispatch, because the fence check (`use_and_retirement.rs:296`) still refuses. The rows count against the bound until their deadline.
   - `lose_next_commit` fires after a successful persist (`registry.rs:703-710`), so the static keeps the new content. That matches the recorded behaviour today and should be stated.

6. **The Outcome overstates.** "A store whose size follows its connections and its writes" is not true.
   - Under steady reads the floor record is written once per 60,000 ms (`registry.rs:675-681`). The 60,000 ms window in acceptance 1 hides this.
   - Change: state the residual in the Outcome and the CHANGELOG entry.

## Checked and sound

- A locked open takes the flock before building (`metadata.rs:694`, `lifecycle_lock` opens a new descriptor each time). The replace happens inside `persist`, before the lock is released. Locked transactions are therefore serial against the static.
- Handles hold no copy of the rows. The `Metadata` SQL connection copies the global static at each build, so `take_or_wait` (`er.rs:571-612`) is irrelevant to it. Only unlocked builds (`inspect`, the first observation attempt) can be stale. They never replace the static, and no observation closure touches `registry_uses`.
- A closure `Err` is rolled back by `ROLLBACK TO action` (`registry.rs:686`), including the sweep `DELETE`. SQL then equals the build snapshot, so the static ends as it was.
- A failed commit skips the persist. A failed expiry batch returns `Err`, so the static is untouched.
- Stripping the keys from `desired` and `rebuilt` covers the `RuntimeStateOnly` and `PreparedObservation` checks (`:3943-3986`). It also covers the creation loop and `postcommit_projection_matches` (`:4315`).
- The strip must use `reference` (`:3939-3941`), so it goes between `:3941` and `:3943`, as written.
- `adopt_er`'s digest (`metadata.rs:965`) reads a freshly provisioned projection, which cannot have static rows. `snapshot_facts` reads the baseline only. The checkpoint write (`er.rs:709`) is unaffected.
- No `DELETE` of connections, generations or materials exists outside tests, and the table is level 2.
- Difference 4 is right. The only code changed is `Conflict` to `Revoked` at `use_and_retirement.rs:294-295` and `revalidation.rs:160-171`.
  - `release_read`, `refuse_read`, `cancel_read` and `invalidate_read` use `dispatched=1`/`dispatched=0` filters or id-only lookups. A CLI revoke changes none of their answers.
  - A dispatched revalidation was already `Revoked`.
- Round-1 findings:
  - 1 is answered (`revoked` named, lines cited, acceptance 5).
  - 7 and 8 are answered (difference 3, acceptance 7, docs in item 16).
  - 6 is mostly answered (accessor, hook, key). A spawned thread under `simulate_process(n)` still acts as process 0, which the Design does not mention.
  - 2 to 5 are moot, because the SQL statements stay.

## Could not establish

- Whether any test combines `simulate_process(n)` with a spawned thread that captures or dispatches. I found none in `registry/tests.rs`, but `store_cost_tests.rs:893` (threads running `simulate_process`) was not read.
- Static growth under a high read rate. Released rows do not count toward the bound but live until their deadline (120 s). That is the same row count as today, and not worse.

```findings
- file: crates/connectors-host/src/local/metadata/er.rs
  line: 3926
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "'A key new to the static' is compared with the live static, but an observation builds unlocked (metadata.rs:1043-1096) and another thread can sweep or release the use before the observation persists; the use then looks new and the PreparedObservation persist answers MetadataUnavailable. Compare against the snapshot taken at build_projection; in modes that do not replace the static drop non-baseline ReadUse keys without admission; apply creation_stages only in the replacing mode."
- file: crates/connectors-host/src/local/metadata.rs
  line: 684
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Standard without concurrent_observation does not mean the projection was built under the lock: relock_observation clears the flag, and persist_with_admission (:851) only needs the lock held. The clause is dead because that flag already refuses the persist. Record at build whether the lock was held (activate_er release_physical false) and replace the static only then."
- file: crates/connectors-host/src/local/metadata/er.rs
  line: 3110
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The static's key (path, file identity, process) can match a recreated store after inode reuse; its rows then fail pragma_foreign_key_check and every open returns MetadataUnavailable until the owner exits. Add the authority id (facade.authority()) to the key, insert a row at build only if its connection, generation and version are in the projection, and say what a None file identity does."
- file: crates/connectors-host/src/local/metadata/er.rs
  line: 4102
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The common read persist returns Ok at the actions.is_empty() early return before the batch, so a replacement written only at the end of persist_batch never runs on the hot path. Do the replacement in persist (:3890) after persist_batch returns Ok, with persist_batch returning the next rows."
- file: crates/connectors-host/src/local/metadata/er.rs
  line: 4199
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "If the post-commit check fails after the batch committed, or a caller's batch fails after expiry batches, the persist returns Err and the static is not replaced although the recorded effect (publication) committed; release-by-connection and release-by-version effects are lost for those rows until their deadline. Safe through the fence check at use_and_retirement.rs:296, but add it as a sixth difference and state that lose_next_commit leaves the new rows in the static, as the recorded rows today."
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 1
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The Outcome says a store's size follows its connections and its writes, but the registry floor is still recorded once per 60,000 ms of use (registry.rs:675-681), and acceptance 1 stays inside one window. State the residual in the Outcome and the CHANGELOG entry."
```
