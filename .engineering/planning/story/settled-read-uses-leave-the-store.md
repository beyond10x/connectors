---
format: aep.planning-md/3
id: story:settled-read-uses-leave-the-store
kind: story
status: implemented
title: A read use is held in the owner process and no longer recorded (D5 way A)
relations:
- decomposes: epic:review-20261123-findings
- serves: vision:independent-contract-adapters
- depends_on: story:entity-runtime-0330-records-name-definition-by-digest
- depends_on: story:read-cost-test-reports-bytes-per-read
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: contracts/cli/v1alpha1/owner.md
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-host/src/local/metadata.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/er.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/metamorphic_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/expiry_batch_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/store_cost_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/tests.rs
- confidence: cited
  path: docs/local-connection-registry.md
- confidence: cited
  path: docs/local-er-metadata.md
- confidence: cited
  path: website/docs/reference/contracts/cli.md
revision: 46
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T10:40:57Z", actor: "human:timo", revision: 44, decided_on: {"recorded":{"review_outcome":8}}}
- {from: "proposed", to: "active", at: "2026-11-23T10:40:58Z", actor: "human:timo", revision: 45, decided_on: {"recorded":{"review_outcome":8}}}
- {from: "active", to: "implemented", at: "2026-11-23T19:56:34Z", actor: "human:timo", revision: 46, decided_on: {"recorded":{"test_result":1,"review_outcome":9}}}
---
## Outcome

A read invoke records nothing per read in the local metadata store. A read use (the
reservation a read, a reuse or a revalidation takes between capture and release) that a
process captures is held in the memory of that process and is no longer recorded.

The clock-floor event stays: a registry transaction more than 60,000 ms past the recorded
floor records one (`crates/connectors-host/src/local/registry.rs:675-681`,
`contracts/service/clock.md`), so an instance that reads all day adds at most one event per
60 s of use.

Stores written by earlier releases do not shrink. Each read use such a release left open in a
store is recorded Expired once, by the first capture after its deadline; after that a read
writes nothing there either.

## Source

Issue #267, wanted item 1. Timo's pick D3 of 2026-11-23 (about 04:57 CET, relayed by
host-operator): "D: A then B (Recommended)"; A was the Entity Runtime 0.33.0 pin
(story:entity-runtime-0330-records-name-definition-by-digest, connectors 0.76.0), B is this
story. Timo's pick D5 of 2026-11-23 (about 10:55 CET, relayed by host-operator) for how B is
done: "A: stop recording reads", with host-operator's words "Read uses are no longer recorded;
open ones live in the owner process; connectors only."

The three ways of question D5: A, connectors stops recording read uses (this story); B, Entity
Runtime gains a removal of settled subjects and connectors calls it; C, connectors rewrites a
store without its settled read uses. Timo picked A. B and C are not built.

Measured: on connectors 0.74.0 103 reads left 387 events and 11.0 MB of blobs
(https://github.com/beyond10x/connectors/issues/267#issuecomment-7091114772); on 0.73.0 2,710 of
3,159 recorded entries were `connectors.credential_evidence.ReadUse`; with the 0.33.0 pin one
read invoke still appends 24,500 blob bytes
(https://github.com/beyond10x/connectors/issues/267#issuecomment-7095756424).

## What the tree holds today

Read on 2026-11-23 at 3d38f77b99f (branch `story/er-0330-pin`, connectors 0.76.0 before its
release) by two scopers and four critics. `L/` is `crates/connectors-host/src/local/`.

- One settled read is one `ReadUse` subject with 4 events (Captured, Dispatched, Released,
  Expired). Every statement on the SQL table `registry_uses` is in five files:
  `L/registry/use_and_retirement.rs`, `reuse.rs`, `revalidation.rs`, `lifecycle.rs`,
  `refresh.rs`. No file outside those five and `L/metadata/er.rs` names `registry_uses` except
  tests.
- The SQL is an in-memory projection. `Metadata` builds it on every open
  (`L/metadata.rs:1096`, `er.rs:3049`) from the recorded baseline of the handle; the pool
  (`IDLE`, `er.rs:630`) keeps the handle (`ErAuthority`), not the SQL connection.
- A persist (`persist_batch`, `er.rs:3904`) reads the SQL rows back (`capture`, `:3942`),
  compares them with the recorded baseline and records each difference as a command
  (`:3997-4060`). A new `registry_uses` row becomes a recorded `ReadUse`
  (`creation_stages`, `:4764`; admitted only from `transaction_admission`,
  `admitted_registry_use`, `:4765-4772`).
- With no difference a persist returns at `er.rs:4102`, before the writer marker (`:4107`) and
  the batch: no event and no blob. The clock row changes only when `now` is more than
  60,000 ms past the recorded floor (`L/registry.rs:59`, `:675-681`).
- One registry transaction (`L/registry.rs:627-710`): the lifecycle lock is taken at
  `metadata.rs:694` before the projection is built and held through the persist. A closure
  that returns `Err` is rolled back in SQL (`ROLLBACK TO action`, `registry.rs:686-688`) and
  the persist still runs for the clock (`:690-702`). A failed commit answers `OutcomeUnknown`
  at `:690` and skips the persist. The test hook `lose_next_commit` fires after a successful
  persist (`:703-710`).
- An observation opens without the lock held through its build: it releases the lock at
  `metadata.rs:1060-1068`, builds, and takes it again at `registry.rs:634`. No observation
  closure has a statement on `registry_uses`.
- Two more places refuse a persist whose SQL holds a key the baseline does not: the
  `RuntimeStateOnly` check (`er.rs:3947-3965`) and the `PreparedObservation` check
  (`:3970-3986`). `postcommit_projection_matches` (`:4199-4211`) compares the SQL rows with the
  baseline after a batch.
- One owner process serves a state directory: it holds the lifetime lock `owner.lock` until it
  exits (`L/owner/transport.rs:1499`, `:2080-2084`), a starting owner waits on that lock and
  starts under `lock(&directory)` (`:333`), and `stop_owner` stops an owner of any build
  (`:1479`).
- Every capture, dispatch, release, cancel, refuse and invalidate of a use runs in the owner
  process: all callers are in `L/owner.rs`, `owner/supervisor.rs`, `owner/transport.rs` and
  `owner/mutation/execution.rs` (`execute` is `pub(in crate::local::owner)`, called at
  `supervisor.rs:1315`). The CLI process calls `admit_read`, which reserves no use
  (`apps/connectors/src/local/definition.rs:1115`), and builds projections for
  `connections list`, `describe`, the acquisition status, `revoke` and `Metadata::inspect`.
- The one statement on `registry_uses` that another process runs: `connections revoke` runs in
  the CLI (`apps/connectors/src/local/connections.rs:168-182`) and releases the connection's
  undispatched uses at `L/registry/lifecycle.rs:469`. A dispatch of a released use answers
  `Failure::Conflict` (`use_and_retirement.rs:294`), which is checked before the revoked state
  (`:295`); an undispatched revalidation likewise (`revalidation.rs:166-167` before `:170-171`).
- `retirements` and `delete_retired` (the callers of the retirement guard `deletion_allowed`,
  `use_and_retirement.rs:447`) have no production caller.
- `ReadUse` and its commands are declared in `ess/domains/credential_evidence.yaml:60-201`,
  `ess/components.yaml:65-68`, `crates/connectors-build/src/metadata_entities.rs:27` and
  `L/metadata/entity-runtime-definitions.json`. The legacy import creates `ReadUse` subjects
  (`L/metadata.rs:2455-2582`, `er.rs:2806`). No other subject's command depends on a recorded
  use (`er.rs:4427-4441`, `:4803-5060`).
- `process()` (`er.rs:634-640`) is 0 outside tests and a thread-local in tests
  (`simulate_process`, `:2927`); `exit_process` (`:2934`) closes that process's handles.
- A captured operation is not resumed after an owner restart today
  (`docs/local-connection-registry.md:220`). A use lasts at most 120 s (`USE_MS`,
  `L/registry.rs:56`), a revalidation capture at most 30 s.

Not established: whether the checkpoint write on a handle's release (`er.rs:709`) writes
nothing for an unchanged store (its comment says so); acceptance 1 measures it.

## Design

The change is in `L/metadata/er.rs`. No statement on `registry_uses` changes, and no
production file under `L/registry/`, `L/owner/` or `apps/` changes.

- A static in `er.rs` holds the row images of the `ReadUse` rows this process captured and did
  not record. Its key is the store's authority id (`facade.authority()`), the store's path
  and `process()`. A store recreated at the same path has another authority id and starts
  empty; the file identity (`er.rs:473`) is not part of the key, so a handle without one works
  as any other. Outside tests `process()` is 0; in tests a
  spawned thread acts as process 0 unless it calls `simulate_process`.
- `build_projection` inserts the static's rows of this store into `registry_uses` beside the
  recorded ones, each only if its connection, generation and version are rows of the
  projection being built. The handle keeps the set of use ids it inserted (the build
  snapshot) and whether the lifecycle lock was held when the projection was built
  (`activate_er(!release_physical)`, `metadata.rs:694`, `:1060-1096`). Every statement of the
  five registry files reads and writes the rows as today: the bound, dispatch, release,
  cancel, refuse, invalidate, the release by connection or by version (`lifecycle.rs:226`,
  `:469`, `refresh.rs:422`, `revalidation.rs:254`, `use_and_retirement.rs:365`), the sweep at
  the next capture and `deletion_allowed`.
- `persist_batch` takes every `ReadUse` key that the SQL holds and `reference`
  (`er.rs:3939-3941`) does not out of `desired`, between `:3941` and `:3943`, and out of the
  rows `postcommit_projection_matches` compares. It returns those rows.
- `persist` (`er.rs:3890`) replaces the static with those rows after `persist_batch` returned
  `Ok`, which covers the return at `:4102` and the return after a batch, and only for a handle
  whose projection was built under the lifecycle lock. In that case a key that is not in the
  build snapshot is a new use: it is admitted only where a recorded creation is admitted today
  (`admitted_registry_use`, a row that is Captured, not dispatched and not released,
  `creation_stages`, `:4764-4772`); otherwise the persist answers as today.
- A handle whose projection was built without the lock (an observation, an inspection, a
  runtime-state persist after a relock) never replaces the static, and its persist drops the
  keys without the admission check: another thread may have swept or released such a use
  after this handle's build, and that is no creation.
- A closure that returned `Err` was rolled back in SQL (the sweep included), so its persist
  puts back what the static held. A failed commit skips the persist, and a failed expiry batch
  returns `Err`: the static keeps what it held. The test hook `lose_next_commit` fires after
  a successful persist, so the static holds the new rows, as the recorded rows do today.
- A `ReadUse` that the recorded baseline holds (left by an earlier release, or created by the
  legacy import) is compared and recorded as today. The sweep of the next capture after its
  deadline records it Expired once.
- `exit_process` drops the static's rows of the simulated process it ends.
- A `#[cfg(test)]` thread-local switch `RECORD_READ_USES` in `er.rs`, off by default and set
  by `record_read_uses(bool)`, makes a persist record a new use as releases up to 0.76.0 did.
  Tests that need a store with recorded uses turn it on; `L/metadata.rs` passes
  `record_read_uses` on as it does `simulate_process` (`metadata.rs:124-139`).
- `ReadUse` stays declared in ESS, in `metadata_entities.rs`, in the definitions file and in
  `registry.sql`, and its mapping in `er.rs` stays. No ESS file and no generated file changes,
  so no generation job runs.

What differs from today, each stated in the CHANGELOG entry:

1. The store holds no record that a read was captured, dispatched or released. The
   clock-floor event stays (Outcome).
2. A use that is open when the owner process ends is gone with it. The read is not resumed
   today either. A dispatch of its id by the next owner answers `Failure::Conflict`, the
   answer for an unknown id.
3. A use is visible only to the process that captured it. One owner serves a state directory,
   so the bound of 1,000 open uses counts as today. The open-use check of `deletion_allowed`
   (`use_and_retirement.rs:447`) holds in no process today: a deletion needs the retirement
   time plus 86,400,000 ms (`RETENTION_MS`, `L/registry.rs:55`, `lifecycle.rs:788`), a use ends
   within 120,000 ms, and an undispatched use is released at retirement (`refresh.rs:422-425`).
   Nothing changes there, and no test of it is added.
4. A `connections revoke` run by the CLI process no longer releases the owner's undispatched
   uses of that connection: `lifecycle.rs:469` matches no row in the CLI's projection. Such a
   use stays in the owner's bound until its deadline (at most 120 s). The owner's dispatch of
   it answers `Failure::Revoked` (`revoked`) where it answers `Failure::Conflict` today, and an
   undispatched revalidation of it likewise. After the deadline both answer `Expired` as
   today.
5. A store written by an earlier release gets one Expired event for each use left in it, and
   keeps the records it has.
6. The static is not written in one step with a recorded batch. When a batch commits and the
   persist then answers `Err` (the post-commit check at `er.rs:4199`, or the caller's batch
   after expiry batches), the static keeps its rows, so a release by connection or by version
   that the same transaction made (`lifecycle.rs:226`, `refresh.rs:422`, `revalidation.rs:254`)
   does not reach them. The fence check (`use_and_retirement.rs:296`) still refuses their
   dispatch with `Failure::Conflict`, and they count in the bound until their deadline.

## Acceptance

Tests are `connectors-host` lib tests; new tests of lines 2, 5 and 6 are in
`L/registry/tests.rs`. The branch starts at 3d38f77b99f and is rebased onto a main that holds
0.76.0 (`git rebase --onto origin/main 3d38f77b99f`) before the checks that name
`origin/main...HEAD`. Every `cargo` command that may run over 10 minutes in the build pool is
started with `SLOT_TIMEOUT=30m`.

1. A new test in `L/registry/store_cost_tests.rs`, not ignored: on a new store, 20
   `measured_invoke` calls at `now` values within 60,000 ms of the recorded registry floor add
   0 to `recorded_events` and 0 to `blob_bytes`. The implementor's report shows the same test
   failing before the change (3 events per invoke). It covers the registry sequence, not the
   owner's transport.
2. A new helper `recorded_read_uses(root) -> i64` beside `recorded_clock_floor_events`
   (`L/registry/tests.rs:1853`) counts the recorded events of `ReadUse` subjects. A new test on
   a new store runs seven uses, one per path, and then `recorded_read_uses` is 0:
   `capture_read`, `dispatch_read`, `release_read`; `capture_read`, `cancel_read`;
   `capture_read`, `dispatch_read`, `refuse_read`; `capture_read`, `invalidate_read`;
   `capture_reuse` and its release; `capture_revalidation`, `dispatch_revalidation`,
   `finish_revalidation`; `capture_revalidation`, `cancel_revalidation`. The same test then
   turns `record_read_uses` on, runs the first path once more, and `recorded_read_uses` is 3.
3. `git diff --stat origin/main...HEAD -- crates/connectors-host/src/local/registry/use_and_retirement.rs
   crates/connectors-host/src/local/registry/reuse.rs
   crates/connectors-host/src/local/registry/revalidation.rs
   crates/connectors-host/src/local/registry/lifecycle.rs
   crates/connectors-host/src/local/registry/refresh.rs crates/connectors-host/src/local/registry.rs
   crates/connectors-host/src/local/owner apps/connectors/src` prints nothing, and
   `git diff --stat origin/main...HEAD -G registry_uses -- crates apps` lists only
   `crates/connectors-host/src/local/metadata/er.rs` and files whose name ends in `tests.rs`.
4. These tests pass in the run of line 17 with their bodies unchanged:
   `a_bound_of_open_read_uses_still_refuses_the_next_capture_as_capacity`,
   `released_read_uses_inside_one_window_do_not_fill_the_concurrent_bound`,
   `a_cancelled_capture_is_released_and_cannot_be_dispatched` (`L/registry/reuse_tests.rs`), the
   tests that count `registry_uses` rows through `Metadata::inspect` (`reuse_tests.rs:420-430`,
   `registry/recipient_tests.rs:110-113`, `owner/governed/tests/final_fence.rs:277-281`,
   `owner/approval_issuance/tests.rs:765-771`), and in `L/registry/tests.rs`
   `terminal_revoke_invalidates_repair_and_dispatch_without_custody`,
   `competing_repairs_have_one_winner_and_old_dispatch_is_fenced`,
   `lost_publication_acknowledgement_is_resolved_without_repeating_capture` and
   `abandoned_candidates_require_acknowledged_retirement_and_full_retention`.
   `git diff --stat origin/main...HEAD -- crates/connectors-host/src/local/registry/reuse_tests.rs
   crates/connectors-host/src/local/registry/recipient_tests.rs` prints nothing.
5. A new test: process 1 captures a read use (`simulate_process(1)`, `capture_read`), process 2
   revokes the connection, process 1 calls `dispatch_read`: the answer is `Failure::Revoked`.
   A second case: process 1 calls `capture_revalidation`, process 2 revokes, process 1 calls
   `dispatch_revalidation`: the answer is `Failure::Revoked`. The report states that both
   answer `Failure::Conflict` before the change, from a run.
6. A new test: process 1 captures one use and captures and dispatches a second, then
   `exit_process`. Process 1 again and process 2 each count 0 rows in `registry_uses`
   (`Metadata::inspect`), and `dispatch_read` of the first use by process 1 answers
   `Failure::Conflict`.
7. `a_store_the_previous_pin_wrote_a_read_into_takes_a_read_invoke` (the 0.32.0 fixture, which
   holds one use): with `now` past that use's deadline and within 60,000 ms of the recorded
   floor, the first read invoke adds 1 to `recorded_read_uses` and a second read invoke adds 0
   to `recorded_events`.
8. `cargo nextest run -p connectors-host -E
   'test(revoked_legacy_connection_retains_both_read_use_histories_without_invented_admission)'`
   exits 0, and the old-side ranges of `git diff -U0 origin/main...HEAD --
   crates/connectors-host/src/local/metadata.rs` lie outside lines 2455-2582.
9. The tests that need recorded uses turn `record_read_uses` on and pass: the four tests of
   `L/registry/expiry_batch_tests.rs` (through `store_with_due_read_uses`, `:36`), `mr1` and
   `mr2` of `L/metadata/metamorphic_tests.rs` (through `populated`, `:456`), and
   `observation_after_a_recorded_read_use_expiry_is_admitted` (`registry/tests.rs:1670`).
   `git grep -n -B1 "RECORD_READ_USES\|fn record_read_uses" --
   crates/connectors-host/src/local/metadata/er.rs crates/connectors-host/src/local/metadata.rs`
   shows `#[cfg(test)]` or a `thread_local!` inside a `#[cfg(test)]` item above each
   definition.
10. `grown_stores` (`store_cost_tests.rs:144`) grows its stores with `record_read_uses` on and
    turns it off before it returns. `CONNECTORS_STORE_COST_EVENTS=30
    CONNECTORS_STORE_COST_INVOKES=3 cargo test -p connectors-host --lib
    read_invoke_cost_by_store_size -- --ignored --nocapture` prints one line with `events=` at
    30 or more and `appended_per_invoke=0`.
11. A report on the pull request and on issue #267, from two release builds on one saved
    store: `CONNECTORS_STORE_COST_STORES=<dir> CONNECTORS_STORE_COST_EVENTS=50
    CONNECTORS_STORE_COST_INVOKES=100 cargo test --release -p connectors-host --lib
    read_invoke_cost_by_store_size -- --ignored --nocapture`, first in a checkout of
    3d38f77b99f (it saves the store in `<dir>`), then on this branch reading the same `<dir>`.
    The report gives `blob_bytes_per_invoke` (24,500 before, 0 after), `median_ms` and
    `peak_rss_mb` of both runs and the output of the shell command `uptime` taken before each
    run. On issue #267 it says that item 1 is fixed for reads from this release on and that
    stores written by earlier releases keep their records. The pull request's title and body
    hold no closing keyword for #267. When the report is posted, the question whether stores
    written by earlier releases get a way to shrink is sent to host-operator with the default
    "leave them; a new state directory is the way to a small store", and the report names it.
12. `git diff --stat origin/main...HEAD -- ess crates/connectors-build/generated
    crates/connectors-core/generated
    crates/connectors-host/src/local/metadata/entity-runtime-definitions.json
    crates/connectors-host/src/local/metadata/registry.sql` prints nothing, and `cargo test -p
    connectors-build --lib the_committed_metadata_emissions_are_current` passes.
13. `contracts/cli/v1alpha1/semantics.md:2094` reads "the owner dispatched it (its read use
    dispatched) is the provider's", on the same line. `cargo nextest run -p connectors-build -E
    'test(the_corrected_semantics_citations_land_on_their_rules)'` exits 0
    (`cli_spec_mapping_adversary.rs:202`; `ess/domains/cli.yaml` cites `semantics.md:1508` and
    `:1512`). The mirror `website/docs/reference/contracts/cli.md:2109` is regenerated:
    `slot cargo run --locked -p connectors-docs -- generate --check` exits 0.
14. `contracts/cli/v1alpha1/owner.md`: the sentence ending at `:210` reads "the source
    connection's record is not written by any step: its read use is held in the owner process
    and is not recorded." The paragraph gains: "A `connections revoke` run by another process
    does not release that use; its dispatch then answers `revoked`."
15. `docs/local-er-metadata.md` holds these two sentences, the first in the paragraph at
    `:79-82`, the second in the paragraph at `:287`: "From connectors 0.77.0 on a captured use
    is a row only in the projection of the process that captured it and is not recorded;
    `connectors.credential_evidence.ReadUse` subjects come from releases up to 0.76.0 and from
    the legacy import." and "A due `ExpireReadUse` comes only from a use that a release up to
    0.76.0 or the legacy import recorded." `docs/local-connection-registry.md`: `:220` gains
    "The read uses of an ended owner are gone with it."; `:233` reads "1,000 open read uses of
    the owner process"; the sentence at `:282` ends "are written by nothing: its read use is
    not recorded." `git grep -n "read use recorded\|read-use records\|but its read use" --
    contracts docs website/docs` prints nothing.
16. `CHANGELOG.md` has a `### Changed` entry under `## Unreleased` above `## 0.76.0` (under the
    existing `## Unreleased` / `### Changed` when
    story:connections-list-answers-through-the-owner merged first). The entry holds each of
    these phrases: "no longer recorded", "clock-floor event", "gone with the owner", "only to
    the process that captured it", "answers `revoked`", "one Expired event", "do not shrink",
    "until their deadline".
17. `cargo nextest run -p connectors-host -E 'test(registry::) | test(owner::) |
    test(keyring::custody) | test(metadata::)'` exits 0, or every test it fails is one of the
    five that story:store-tests-end-within-60s-under-load names and is killed at 60 s on the
    base too. `cargo nextest run -p connectors -E 'binary(local_cli)'` exits 0 (it holds the
    CLI revoke, `apps/connectors/tests/local_cli.rs:846`). The report gives each new test's
    time, the wall time of both runs and the base's wall time for the first filter; a run over
    120 s on the base too is carried by story:store-tests-end-within-60s-under-load.

## Shares files with

- story:store-tests-end-within-60s-under-load (draft, `depends_on` this story, scope
  `L/registry/tests.rs`, `L/registry/store_cost_tests.rs`,
  `crates/connectors-host/tests/metadata_reopen_adversary.rs`). This story edits
  `store_cost_tests.rs:144-166` (`grown_stores`), `:470` (the fixture test), adds one test
  there, and adds the helper `recorded_read_uses` (near `:1853`) and the new tests of
  acceptance 2, 5 and 6 to `registry/tests.rs`, and edits its test at `:1670`. That
  story's five tests (`tests.rs:787`, `:1463`, `:2061`, `store_cost_tests.rs:529`,
  `metadata_reopen_adversary.rs:175`) are not edited here. It runs after this story: a read
  invoke that records nothing changes the times it was written for.
- story:connections-list-answers-through-the-owner (in implementation, on main-based branch
  `story/connections-through-owner`). Files both edit:
  - `CHANGELOG.md`: both add under `## Unreleased` / `### Changed` at line 3. The second to
    merge puts its entry under the heading the first created.
  - `website/docs/reference/contracts/cli.md` (generated): that story changes the mirror of
    `semantics.md:1378-1380` (hunk at 1393), this one the mirror of `:2094` (`:2109`). The
    second to merge rebases and runs `connectors-docs generate` again.
  - `contracts/cli/v1alpha1/semantics.md`: `:1378-1380` there, `:2094` here; neither adds a
    line.
  - `contracts/cli/v1alpha1/owner.md`: `:224-261` there, `:207-211` here.
  - `docs/local-er-metadata.md`: `:269-271` there (2 lines added), `:79-82` and `:287` here;
    the line numbers after `:270` in this story move by 2 when that story merges first.
- No production file under `L/owner/`, `L/registry/` or `apps/` (acceptance 3).

## Out of scope

- Stores that already hold read uses. Timo's D5 pick A chose "stop recording reads" over B (a
  removal of settled subjects in Entity Runtime) and C (a rewrite of the store). Such a store
  keeps its size; issue #267 item 1 is fixed for reads from this release on and stays open for
  those stores. The report of acceptance 11 says so on the issue, and with it the question
  whether such stores get a way to shrink is sent to host-operator, with the default "leave
  them; a new state directory is the way to a small store" (acceptance 11).
- The clock-floor event: at most one per 60 s of use, under `contracts/service/clock.md`.
- Removing `ReadUse` from ESS, the definitions and `er.rs`.
- A new owner request that tells the owner of a CLI revoke.
- An audit record of reads: no requirement in code, documents or contracts.
- The root decision "model concrete ReadUse retention" in
  story:ess-evolution-er-metadata-adoption is superseded by D5 for uses written from this
  release on; that story's text is not edited here.
