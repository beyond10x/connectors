---
format: aep.planning-md/3
id: story:where-do-the-82-s-of-the-eleven-store-tests-go-that-the-registry-story-leaves
kind: story
status: draft
title: Where do the 81.7 s of the eleven store tests go that the registry story leaves?
summary: 'A question, not a change: twelve costs the measurement of 2026-11-25 saw and did not attribute far enough for a story, each with whose code it is and what must be measured first'
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
revision: 1
---
## Defect

The measurement of 2026-11-25 named where the eleven store tests of
story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run spend their time. Two
drafts take what it could attribute to a cause:
story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process takes
35.98 s of the 117.685 s (30.6 %), and
story:the-mysql-test-fixture-sends-its-packets-without-the-nagle-hold takes the SQL test. The
other 81.7 s of the eleven tests (117.685 − 35.979, arithmetic) have no cause that was measured
far enough for a story. This story holds them as questions, so that no change is planned on a
figure nobody read.

Source of the figures unless an item says otherwise: run 5 of that measurement on the build
host, head d5cf22480: the eleven store tests one at a time, with timers around the calls,
117.685 s summed. Shares are of that sum (arithmetic). For the tests with several threads the
figures are thread-seconds, and several items overlap (items 2 and 3, items 4 to 6), so the
items do not sum to 81.7 s. "Dependency" is Entity Runtime 0.33.1. File names are under
`crates/connectors-host/src/local/` unless they start with `adapters/`.

## Change

None in product code and none in a test.

Questions. Each item says whose code the cost is in and what a measurement must show before a
story can be written:

1. BATCH: the write of the batch. 30.29 s, 25.7 %, 1,293 calls of `er.execute_batch`, 23.4 ms
   on average (arithmetic).
   - Whose code: the dependency's (`RecordedProviderFacade::execute_batch`). This repository
     decides how many batches there are and what is in them (`persist_batch`,
     `metadata/er.rs` line 4201).
   - Known: the cost per call differs by a factor of 13 between tests: 15.5 ms (301 calls,
     `bounded_owner…`), 56.9 ms (66 calls, `a_reconnect…`), 202 ms (20 calls,
     `a_read_invoke…`). In `bounded_owner…` the sampler (1,614 samples) puts 22.6 % of the CPU
     in the dependency's `drive_request`, 13.5 % in `sqlite3Prepare` and 5.3 % in
     `definition_comparison_bytes`. `fsync` is ruled out (at most 1.05 s in any test, run 2).
   - Not established: why a call costs 15 ms in one test and 202 ms in another.
   - To measure first: per call the number of actions, the number of events in the store and
     the time, to say whether the cost follows the batch or the store; then the sampler on
     `a_reconnect…` and `a_read_invoke…` for the split inside the dependency. If the cost is
     the dependency's, the story belongs to its repository.
2. READBACK: the read after the batch (`catch_up` in `persist_batch`). 7.00 s, 5.9 %, 1,293
   calls.
   - Whose code: this repository calls it; the read itself is the dependency's. Over the whole
     run `er.lookup_batch` took 6.04 s in 1,192 calls and `er.read_history` 4.58 s in 1,987
     calls; they are parts of items 2 and 3 and are not added to them.
   - To measure first: per call the number of entries read back, and whether the batch's own
     answer already holds them (read the dependency's return type first).
3. CATCHUP: the catch-up of a pooled handle on open. 5.75 s, 4.9 %, 1,972 calls, 2.9 ms each
   (arithmetic).
   - Whose code: this repository's (`open`, `metadata/er.rs` line 2466) calling the
     dependency's read.
   - To measure first: how many of the 1,972 catch-ups find no new entry, and what such a call
     costs.
4. PROJ: building the projection. 5.67 s, 4.8 %, 2,973 calls, 1.9 ms each (arithmetic).
   - Whose code: this repository's (`build_projection`, `metadata/er.rs`).
   - To measure first: how many of the calls follow a persist that changed nothing (item 6 is
     one source).
5. VALID: `Metadata::validate`. 5.53 s, 4.7 %, 5,318 calls, 1.0 ms each (arithmetic).
   - Whose code: this repository's (`metadata.rs`).
   - To measure first: calls per open and per update, and how many validate a state that was
     validated before without a change between.
6. The migrate with nothing to migrate. `update` runs `migrate` (`metadata.rs` line 803), which
   persists (line 834), builds the projection (line 845), syncs the directory and validates
   even when no step applied. 420 calls inside `update`, 8.07 s; 5.61 s of it are REG and
   PARSE2, which the registry draft takes; 2.45 s (2.1 %) stay.
   - Whose code: this repository's product code.
   - Not established: whether a return before the persist keeps the behaviour. The persist can
     write the writer record (`writer_action`, `metadata/er.rs` line 4447).
   - To measure first: the split of the 2.45 s between the rest of the persist, the
     projection, the sync (0.17 s in 754 calls over all migrates) and the validation; and, by
     reading, which tests hold what a migrate without a step must still do. The 2.45 s overlap
     with items 4 and 5.
7. The poll of the lifecycle lock. `lock_within` (`metadata.rs` line 1465) tries `flock` without
   blocking and sleeps 5 ms between tries (lines 1471 and 1483). Waiting in it: 170.55
   thread-seconds in 2,345 acquisitions, of them 93.18 in `adversary_concurrent_issues…` (553
   acquisitions) and 65.72 in `replays_waiting…`.
   - Whose code: this repository's product code.
   - Not established: what the poll costs in wall time. The figure is waiting, most of it for a
     holder that works; only the time between a release and the waiter's next try is lost.
   - To measure first: per acquisition the time from the holder's release to the waiter's
     success. No change is proposed before that.
8. The fresh opens of `adversary_concurrent_issues_within_the_lead_send_one_exchange`. 138 of
   its 545 opens are fresh: `er.start` 2.29, `resynchronize` 2.55 and `complete_snapshot` 3.48
   thread-seconds (274 snapshots).
   - Whose code: this repository's (the pool of idle handles in `metadata/er.rs`), and the
     test's 8 trials.
   - Not established: why these opens find no pooled handle.
   - To measure first: per fresh open its reason, counted with a `cfg(test)` seam beside
     `note_fresh_open` (line 2990): no handle for the path, the pool full, another process id.
9. `ess generate` per test. The helper `schema` (`adapters/sql/tests/ess_model.rs` line 24) runs
   it in 16 of the 17 tests of the file. One call: 0.10, 0.13, 0.54 and 0.69 s in four runs on
   the build host; 0.568 s in run 6.
   - Whose code: this repository's test code.
   - To measure first: the time of the call on the forge, from the durations of the 16 tests in
     a gate job. The accepted forms are a committed generated file with a digest of its sources
     or one product per test run.
10. The SQL test, when the fixture draft is refuted. 8.437 s of 9.229 s are in `epoll_wait`
    (run 6); 23 calls of 56.9 to 59.6 ms explain about 1.35 s of it (arithmetic).
    - Whose code: not established: the test fixture, the adapter or `mysql_async` 0.37.1.
    - To measure first: a trace with time stamps of `sendto`, `recvfrom` and `epoll_wait` per
      descriptor, and the `epoll_wait` durations counted in steps of 10 ms.
11. REG and PARSE2 in the tests under 5 s. Not measured. The plan's rows: 103 unit tests of
    `connectors-host` between 1 s and 5 s sum to 189.5 s. If they carried the 30.6 % of the
    eleven, that would be 58 s summed (arithmetic on an assumption nobody checked).
    - Whose code: this repository's; the registry draft changes it.
    - To measure first: the durations of those tests in the gate job of the registry draft's
      pull request against its base. The story
      `where-do-the-1298-s-of-the-tests-under-5-s-go` holds the question.
12. Variance from run to run. `bounded_owner…` took 30.3, 24.1, 19.7 and 17.6 s in four runs on
    the build host; `concurrent_revoke…` took 30.45 s once (run 3, 154 ms per batch call) and
    7.309 s in run 5.
    - Whose code: not established: the load of the shared build host or the disk under the
      temporary directory.
    - To measure first: one test several times with the host's load recorded beside each run,
      and once with its temporary directory in memory. That comparison was not run.

Closed by the measurement and not a question: the store built from nothing per test. It costs
5.55 s in 101 calls of `Metadata::initialize` over the eleven tests; a template would save 33 to
38 ms per store, 3.3 to 3.8 s for the 99 stores that unit tests can reach. The section "Measured
(2026-11-25): not built as written" of
story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run holds the figures.

## Acceptance

1. Every item gets one of two entries under a heading `## Answer`: the measurement the item
   names, with each figure's source (the command of the run on the build host and its output, or
   the job of the forge run); or one sentence that the item is dropped, with the reason and the
   figure it rests on.
2. For each cost an answer attributes to this repository's code, the answer names the draft
   story that then claims it, created as a draft that decomposes
   story:the-gates-test-step-ends-within-120-s-on-the-forge, or says that no change removes it
   and why.
3. For each cost an answer attributes to the dependency, the answer says so with the function
   and the share, and names no change in this repository. Whether a story is asked of the
   dependency's repository is the operator's decision; the answer lists it for that decision.
4. Items 1 to 8 are measured after
   story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process has
   merged, on a head that holds it, because that change removes REG and PARSE2 from every figure
   above. Item 10 is taken up only when the fixture story is refuted twice. Item 11 is read from
   the gate job of the registry story's pull request and needs no run of its own.

## Scope

No file of the repository is written. Recorded scope: none. The files read are
`crates/connectors-host/src/local/metadata.rs`, `crates/connectors-host/src/local/metadata/er.rs`,
the test files of the eleven tests, `adapters/sql/tests/ess_model.rs` and
`adapters/sql/tests/mysql/fixture.rs`. A measurement may add temporary timers or a `cfg(test)`
counter; they are taken back and no pull request carries them.

## Not in this story

- Any change of a test, a fixture or product code.
- REG and PARSE2:
  story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process.
- The hold in the MySQL test fixture:
  story:the-mysql-test-fixture-sends-its-packets-without-the-nagle-hold.
- The tests under 5 s as a whole: story:where-do-the-1298-s-of-the-tests-under-5-s-go. Item 11
  gives that story one figure and takes nothing from it.
- A change in the dependency's repository.
- The runs follow the folder rules: on the build host, no run of the whole workspace, no timing,
  soak or stress run on the workstation, each test bound to 60 s by the runner, the test step
  bound to 120 s, a pool job to 10 minutes unless it names its time (30 minutes at most), a
  script that only measures starts without the `slot` prefix, no new Python.
