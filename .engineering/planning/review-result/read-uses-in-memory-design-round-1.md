---
format: aep.planning-md/3
id: review-result:read-uses-in-memory-design-round-1
kind: review-result
status: active
title: Design critic, round 1, story:settled-read-uses-leave-the-store
relations:
- reviews: story:settled-read-uses-leave-the-store
revision: 1
---
verdict: needs-revision

Round 1 on `story:settled-read-uses-leave-the-store`, read at `3d38f77b99f`. The six choices hold in principle. Choice 4 is misdescribed, and the Design leaves the table's other touch points, its commit behaviour and the test-harness hooks unsaid.

All paths below are under `crates/connectors-host/src/local/` unless they start with `apps/` or `contracts/`.

## Findings

1. **Choice 4 describes today's behaviour wrongly, and the answered code changes.**
   - Today a CLI revoke runs `lifecycle.rs:469` and sets `released=1` on the connection's undispatched uses.
   - A later `dispatch_read` then stops at `use_and_retirement.rs:294` (`dispatched || released` gives `Conflict`). That line comes before the `revoked` check at `:295`.
   - Without `:469`, nothing marks the table entry released. Dispatch passes `:294` and answers `Revoked` at `:295`. The fence and generation checks at `:296` also refuse it.
   - Revalidation behaves the same way. `revalidation.rs:164` returns `Conflict` on `released`, and `:168` returns `Revoked`.
   - So the refusal is real, but it is `revoked` where today it is `lifecycle_conflict`. The sentence "as the registry does today for a use the revoke did not release" describes a use that was already dispatched, which is a different case.
   - Change to make: the Design names `revoked`, with `use_and_retirement.rs:295` and `revalidation.rs:168` as the lines. It says `lifecycle.rs:469` stops releasing uses. It lists the changed code in the CHANGELOG entry and in `semantics.md`/`owner.md`. Acceptance 5 asserts `revoked`.
   - The expiry check at `:288` still comes first, so a revoke after the deadline answers `Expired`.

2. **Table effects at four more sites are not named.**
   - Choice 5 lists release, refuse, cancel, invalidate and deadline. Four other sites also set `released=1`:
     - publication at `lifecycle.rs:226`;
     - refresh publication at `refresh.rs:422`;
     - `revalidation.rs:241`, the revalidation `release`;
     - `revalidation.rs:254`, the revalidation `invalidate`.
   - Two of them release by connection or by version, not by use id: `use_and_retirement.rs:365` and `revalidation.rs:254` release every undispatched use of a version.
   - A table keyed only by use id cannot do that.
   - Change to make: the Design says the table can be searched by connection and by version. It also says, for each of these sites, whether it removes entries or leaves them until the deadline.
   - Leaving them is safe for dispatch, because the fence check at `:296` still refuses the old use with `Conflict`, the same code as today.
   - Leaving them has costs the Design must state. The entries count against the 1,000 bound until their deadline, which is at most `USE_MS` = 120 s (`registry.rs:56`). `deletion_allowed` also sees them. That check is nearly redundant, since `RETENTION_MS` is 24 h (`registry.rs:55`).

3. **The Design is silent on commit semantics.**
   - A table change made inside the transaction closure is not rolled back in these cases:
     - when `transaction_inner` returns `Err` after the closure succeeded, from `tx.commit()` → `OutcomeUnknown`, from a failed `persist`, or from the `lose_next_commit` hook (`registry.rs:~700-715`);
     - when `finish_revalidation` fails at `native.validate(...)?` and rolls back its SQL `release`.
   - Change to make: state that table inserts, flag changes and removals take effect only after the `transaction_*` call returns `Ok`, or are undone on `Err`.
   - State that the capacity check and the insert happen under one table lock. The lifecycle file lock already serialises captures.
   - State the lock order: store lifecycle lock first, then the table mutex. No code holds the mutex while it waits for the lifecycle lock.
   - `deletion_allowed` runs inside the lifecycle-locked closure, and `release`/`cancel` take the mutex alone, so this order holds.

4. **Release, cancel and the clock coupling are not stated.**
   - `release_read` (`:304`), `cancel_read` (`:341`) and `cancel_revalidation` (`revalidation.rs:~356`) open the store, take the lifecycle lock, sample the clock, and record the floor file and `registry_clock` (`registry.rs:636-680`). They fail with `MetadataUnavailable` on a clock regression. Every caller uses `?` (`supervisor.rs:1903-1905`, `transport.rs:3465`, `:3620`). The `Use` guard in `execution.rs:46-57` calls them from `Drop`.
   - Change to make: say these three no longer open the store and return `Ok(())`. Signatures stay as they are, so no caller changes.
   - Say they no longer sample the clock or the floor, and no longer wait for the lifecycle lock. Releasing only lowers protection, so it needs no ordering against revoke or publication.
   - Say `dispatch_read`, `refuse_read` (it still marks the connection Stale), `invalidate_read` and the revalidation `current`/`finish`/`missing` keep their transaction. They read the table inside the closure with the closure's `now`, which is the clock sampled after the lock when `system_clock` is set.
   - Tests that count store opens or floor writes for release and cancel (`store_cost_tests.rs`) change.

5. **"At its deadline" must mean the lazy sweep.**
   - `dispatch_read` (`:288-293`) and `current` (`revalidation.rs:168`) answer `Expired`, or `EvidenceExpired`, from a use that is past its deadline but not yet swept.
   - Removing an entry eagerly at its deadline turns that answer into `Conflict` (a missing id).
   - Change to make: the Design says entries leave at the next capture's sweep, with `expires <= now` using the transaction's `now`. This matches today's SQL sweep.
   - It also says a released or cancelled entry is absent, so a dispatch after its deadline answers `Conflict` where today's present-but-released row answers `Expired`.

6. **"Keyed by the simulated process" is not buildable from the registry module alone, and the harness hooks are missing from the scope.**
   - `er::process()` is private and `#[cfg(test)]` (`er.rs:~634`). Outside tests it returns 0.
   - `simulate_process` sets a thread-local (`er.rs:2927`), so a spawned thread acts as process 0.
   - For ER handles a mismatch only costs a reopen. For the table it changes outcomes to `Conflict`.
   - `exit_process` (`er.rs:2936`, `metadata.rs:129`) closes handles only. Tests reuse an id after an exit (`store_cost_tests.rs:38-39`, `:322-330`) and would see stale entries.
   - Many tests build a second `Registry::new(root.path())` in the same thread and call it a restart (`registry/tests.rs:1308`, `:1967`, `:2005`). Those share the table.
   - Change to make: the scope adds `metadata.rs` and `metadata/er.rs`. The Design names:
     - a cfg(test) accessor for the process id;
     - an `exit_process` hook that drops that process's entries;
     - that acceptance 5 and 6 call `simulate_process` and `exit_process` explicitly;
     - that the production key is (state directory, 0);
     - a `pub(in crate::local)` cfg(test) snapshot accessor for the tests outside `registry/` (`final_fence.rs:281`, `approval_issuance/tests.rs:766`).
   - Key the table on the `Registry.path` as given. Every owner site builds it from `paths.state`.

7. **Two things the scoper left "not established" are settled by the code, and the Design should say so.**
   - One owner serves a state directory.
     - It holds the lifetime lock `owner.lock` until exit (`owner/transport.rs:1499`, `:2080-2084`).
     - A starting owner waits on that lock and starts under `lock(&directory)` (`:333`).
     - Retirement requires `quiet` (`:2045`): no clients, no recovery pass, and the pool quiet.
     - `stop_owner` stops an owner "of any build" (`:1479`).
     - An owner of another build therefore cannot run beside a new one. So the 1,000 bound per owner is the bound per state directory.
   - Every capture and settle is in the owner.
     - All callers of the use API are in `owner.rs`, `owner/supervisor.rs`, `owner/transport.rs` and `owner/mutation/execution.rs`. `execute` is `pub(in crate::local::owner)`.
     - The CLI calls only `admit_read` (`apps/connectors/src/local/definition.rs:1115`). It has no capture, dispatch, release or cancel.
     - Publication and refresh publication are in the owner too: `transport.rs:3070` and `:3484`, `mutation/custody.rs:165`, `owner/refresh.rs:503`.
     - The only CLI touch is `connections revoke` (`apps/connectors/src/local/connections.rs:168-182`).
   - Change to make: replace "Not established by the scoper" with these facts and their lines.

8. **`deletion_allowed` protects only its own process, and the Design should record the constraint.**
   - `retirements` and `delete_retired` have no production caller. A later caller in a CLI or maintenance process would silently see an empty table.
   - Change to make: the Design records that any caller of the deletion guard must be the owner process, and `docs/local-connection-registry.md` says so.

## Checked and found sound

- **Choice 1.** `Registry` holds only a path and is rebuilt at each call site with `Registry::with_system_clock(&paths.state)`, for example `execution.rs:~197`, `transport.rs:3580`, `owner.rs:1016`. A table inside `Registry` would be lost between calls. A static keyed by state directory is the only form that works without changes under `owner/`.
- **Choice 2.** The read, reuse and revalidation captures share the bound through `admit_read_use_capacity` at `use_and_retirement.rs:266`, `reuse.rs:187` and `revalidation.rs:374`. `capture_recollection` goes through `capture` at `revalidation.rs:~337`.
- **Choice 3.**
  - The sweep `DELETE` at `use_and_retirement.rs:263`, `reuse.rs:173` and `revalidation.rs:373` removes leftover rows.
  - `removal_command` (`er.rs:4749-4762`) expires `Captured`, `Dispatched` and `Released` rows. `expired_transient` (`:3200`) and `projects` (`:3216`) accept the Expired result.
  - With no row difference, `persist_batch` returns early at `actions.is_empty()` (`er.rs:~4096`). No batch, event or blob is written and no writer marker is added.
  - Keeping the sweep is consistent with how `er.rs` records row differences. A capture on a new store writes nothing besides the clock row, and only past `FLOOR_RECORD_MS`.
- **Capture lease and idempotency.**
  - A revalidation capture lasts at most 30 s (`revalidation.rs:~345`).
  - No process other than the owner reads a use. Nothing outside `registry/` and `er.rs` names a use id; `standing.rs` `grant_uses` is a different table.
  - After an owner end the old id is missing and answers `Conflict`, which is the replay refusal as today.
- **`deletion_allowed`.** With one owner and a 24 h retention, the open uses in the table are the only live ones.
- **Leftover rows from older stores.** They no longer count towards capacity or `deletion_allowed`, but their owner is gone because only one runs, so this is safe.
- **Existing acceptance tests.** Acceptance 4 holds as written. `terminal_revoke_invalidates_repair_and_dispatch_without_custody` asserts only `is_err` on dispatch (`tests.rs:1307`). `competing_repairs_have_one_winner_and_old_dispatch_is_fenced` expects `Conflict`, which the fence check at `:296` still gives (`tests.rs:1273-1274`).

Walked: no graph edges (I did not run the graph commands). I read the one story, the code above, and no other artifacts. Acceptance reads traced to a producer in the set: none.

Out of my lane: whether acceptance 1 holds as written is for the acceptance critic. I traced the read path (`admitted_read`, `observation::readiness`) and found no other per-read write, so the 0-event claim rests on `registry_clock`, which acceptance 1 already limits to one clock interval.

```findings
- file: crates/connectors-host/src/local/registry/use_and_retirement.rs
  line: 294
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Choice 4 says the owner's refusal after a CLI revoke is today's behaviour, but today lifecycle.rs:469 releases the use and dispatch answers Conflict at :294; without that release it answers Revoked at :295 (revalidation.rs:164-168 likewise), so the Design must name `revoked` as a changed code, drop :469's release, and the acceptance and CHANGELOG must state it"
- file: crates/connectors-host/src/local/registry/lifecycle.rs
  line: 226
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Choice 5 omits the other table-effect sites: publication (lifecycle.rs:226), refresh publication (refresh.rs:422), revalidation release and invalidate (revalidation.rs:241, :254) and invalidate_read (use_and_retirement.rs:365) release by connection or by version, not by use id, so the Design must say the table is searchable by both and, per site, whether it removes entries or leaves them to the deadline with the capacity effect"
- file: crates/connectors-host/src/local/registry.rs
  line: 700
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "The Design does not say when table changes take effect: a transaction can return Err after its closure succeeded (commit OutcomeUnknown, persist failure, lose_next_commit) and finish_revalidation rolls back its release, so table changes must apply only after transaction_* returns Ok or be undone, the capacity check and insert must share one table lock, and the lock order is lifecycle lock then table mutex"
- file: crates/connectors-host/src/local/registry/use_and_retirement.rs
  line: 304
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "release_read, cancel_read and cancel_revalidation open the store, take the lifecycle lock and sample the clock floor today (registry.rs:636-680) and callers propagate their errors with `?`; the Design must say these three no longer open the store, keep their Result signature and return Ok, no longer record the floor or wait for the lifecycle lock, while dispatch, refuse, invalidate and the revalidation current/finish/missing keep their transaction and use the closure's now"
- file: crates/connectors-host/src/local/registry/use_and_retirement.rs
  line: 288
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Choice 5's removal 'at its deadline' must be the lazy sweep at the next capture: dispatch (:288-293) and revalidation current (:168) answer Expired or EvidenceExpired from an unswept past-deadline use, and eager removal would turn those contract codes into Conflict; the Design must say so and that a released entry past its deadline answers Conflict where today it answers Expired"
- file: crates/connectors-host/src/local/metadata/er.rs
  line: 2927
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "The process key in choice 1 needs er::process() (private, cfg(test), 0 outside tests, thread-local) and an exit_process hook that drops that process's entries, neither of which is in the scope (metadata.rs and er.rs are absent); without the hook acceptance 6 passes only by distinct ids and tests that reuse an id after exit (store_cost_tests.rs:38-39, :322-330) see stale uses, and a spawned thread acts as process 0 so its table misses change outcomes to Conflict; the Design names the accessor, the hook, a cfg(test) snapshot accessor for tests outside registry/, and that same-thread 'restarted' Registry values share the table"
- file: crates/connectors-host/src/local/owner/transport.rs
  line: 2080
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The story lists 'two owners on one state directory' and 'only the owner runs the mutation executor' as not established, but owner.lock is held until exit (transport.rs:1499, :2080-2084), starts run under lock(&directory) (:333), retirement requires quiet (:2045), and every use-API caller is in owner.rs, owner/supervisor.rs, owner/transport.rs and owner/mutation/execution.rs with the CLI calling only admit_read (apps/connectors/src/local/definition.rs:1115); the Design records these facts and lines in place of 'not established'"
- file: crates/connectors-host/src/local/registry/use_and_retirement.rs
  line: 447
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "deletion_allowed will see only its own process's table and retirements/delete_retired have no production caller, so a later caller outside the owner would silently see no open uses; the Design and docs/local-connection-registry.md record that any caller of the deletion guard must be the owner process"
```
