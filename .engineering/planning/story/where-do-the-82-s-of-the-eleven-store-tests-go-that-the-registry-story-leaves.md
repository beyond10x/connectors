---
format: aep.planning-md/3
id: story:where-do-the-82-s-of-the-eleven-store-tests-go-that-the-registry-story-leaves
kind: story
status: draft
title: Where do the 81.7 s of the eleven store tests go that the registry story leaves?
summary: 'A question, not a change: thirteen items the measurement of 2026-11-25 and its critics left open, each with whose code it is and what must be measured first; items 9 and 10 are about the SQL test, item 13 about four tests whose race is not shown to happen'
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- depends_on: story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process
revision: 6
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

Three items are not about the 81.7 s. Items 9 and 10 concern the SQL test
`every_page_is_one_the_model_admits`, which is not one of the eleven. Item 13 is not a cost: it
asks whether four of the eleven tests still meet in their race, before and after the registry
change.

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
   - Known by reading: the batch answers with a receipt (`outcome.receipt()`, `metadata/er.rs`
     after line 4454), not with the records. The read after it does two things (lines 4479 to
     4487): it checks that each member of the receipt is in the store, and it reads what other
     handles wrote since the baseline. So it is a check of this repository, and dropping it is
     a change of what a persist verifies, not a saving that keeps behaviour.
   - To measure first: per call the number of entries read back, split into the batch's own
     members and the entries of other handles.
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
   PARSE2, which the registry draft takes; 2.46 s (2.1 %) stay (arithmetic).
   - Whose code: this repository's product code.
   - Not established: whether a return before the persist keeps the behaviour. The persist can
     write the writer record (`writer_action`, `metadata/er.rs` line 4447).
   - To establish first by reading, no run: which tests hold what a migrate without a step
     must still do.
   - To measure first: the split of the 2.46 s between the rest of the persist, the
     projection, the sync (0.17 s in 754 calls over all migrates) and the validation. The
     2.46 s overlap with items 4 and 5.
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
   - This item owns the three timers of a fresh open for all eleven tests: its answer also
     covers the 4.01 s of them in the other ten tests (item 12).
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
11. Variance from run to run. `bounded_owner…` took 30.3, 24.1, 19.7 and 17.6 s in four runs on
    the build host; `concurrent_revoke…` took 30.45 s once (run 3, 154 ms per batch call) and
    7.309 s in run 5.
    - Whose code: not established: the load of the shared build host or the disk under the
      temporary directory.
    - To measure first: one test several times with the host's load recorded beside each run,
      and once with its temporary directory in memory. That comparison was not run.
12. The rest. 27.47 s (23.3 %) of the 117.685 s are in none of the seven named costs of the
    measurement (BATCH, REG, READBACK, PARSE2, CATCHUP, PROJ, VALID).
    - Closed: 5.55 s of it are the 101 calls of `Metadata::initialize` (the paragraph "Closed by
      the measurement" below). 21.92 s remain (27.47 − 5.55, arithmetic).
    - Timers of run 5 that no item above holds, summed over the eleven tests: `host.feed`
      3,467.8 ms, `host.capture_sql` 1,694.2 ms, `m.open_connection` 1,463.9 ms,
      `m.floor_record` 909.9 ms, `er.write_checkpoint` plus `er.enable_checkpoints` 698.0 ms.
      Together 8.23 s (arithmetic).
    - `er.start`, `resynchronize` and `complete_snapshot` sum to 12.33 s over the eleven tests:
      8.32 thread-seconds in the one test of item 8 and 4.01 s in the other ten (arithmetic).
      Item 8 answers for all of it; this item gets no entry for these three timers.
    - Not established: how the timers nest in each other and in the seven named costs. So the
      figures of this item are not added to each other, and 21.92 s is the most that has no
      owner.
    - Whose code: not established per timer.
    - To measure first: the timers of run 5 as a tree, each with its own time without the time
      of the timers inside it, on a head that holds the registry change; then whose code each
      timer over 1.0 s of own time is. The answer has one entry per such timer and one entry
      for what is left, with its sum.
13. Four tests whose race is not shown to happen. Read on 2026-11-25, not measured. Each
    starts several threads or tasks and asserts something that also holds when they run one
    after another. A persist that is 13.67 ms shorter on the build host (10.63 + 3.04,
    arithmetic) can make them meet less often.
    - `adversary_concurrent_issues_within_the_lead_send_one_exchange`
      (`owner/refresh/tests/issue.rs` lines 299 to 345): 16 callers in each of 8 trials start at
      a `Barrier` (line 311) and then sleep 3 ms times their index (line 319), so they arrive 0
      to 45 ms apart. The test asserts one request at the token endpoint per trial (lines 329 to
      343).
    - `concurrent_revoke_repair_publication_and_read_dispatch_share_one_fence`
      (`registry/tests.rs` lines 786 to 856): three threads in each of 8 rounds start at a
      `Barrier` (line 805), each with the contended wait of the lifecycle lock (line 809). The
      test accepts success, `Conflict` or `Revoked` for the publication and for the dispatch
      (lines 838 to 850) and asserts the state `Revoked` at the end.
    - `abort_and_gate_compete_and_recovery_never_reconstructs_a_send_receipt`
      (`mutations/tests.rs` lines 490 to 522): two threads in each of 24 rounds start at a
      `Barrier` (line 497). The test asserts that one of the two succeeds and the other does not
      (line 509).
    - `adversary_a_concurrent_duplicate_under_a_cap_of_one_is_a_replay_not_rate_limited`
      (`crates/connectors-host/tests/http_standing_grant_adversary.rs` lines 258 to 324): 8
      tasks in each of 6 rounds are spawned in a loop (lines 272 to 287) with no common start.
      The test asserts one provider request per round and accepts an answer of 200 or a
      replayed 502 (lines 298 to 323). It never requires the 502.
    - A `Barrier` gives a common start. It does not show that two threads were in the same
      section at the same time.
    - Not in this item: the fifth test with several threads among the eleven
      (`replays_waiting_on_an_original_hold_no_slot_and_at_most_four_wait_per_key`,
      `crates/connectors-host/tests/http_standing_grant.rs` lines 948 to 1030). It holds the
      provider's answer with a lock until it has counted the waiters (lines 957 to 984), so its
      overlap is shown.
    - Whose code: this repository's test code.
    - Not established: how often the threads meet today, and whether they meet less often
      after the registry change. For the third test also where its two threads meet:
      `open_dispatch` and `abort` were not read. That is read before its counter is placed.
    - The count, one figure per test and run, taken with a temporary counter:
      - first test: per trial the number of callers, the leader not counted, that read the
        expiring generation before the leader published the new one (0 to 15); the figure is
        the sum over the 8 trials (0 to 120). The leader is the caller whose request reaches
        the token endpoint.
      - second and third test: per round 1 when at least one acquisition of the lifecycle lock
        (`lock_within`, `metadata.rs` line 1465) between the `Barrier` and the joins needed
        more than one try, else 0; the figure is the sum over the rounds (0 to 8, 0 to 24). If
        the reading shows that the two threads of the third test meet elsewhere, the entry
        names that place and counts there.
      - fourth test: per round the number of tasks, the original not counted, that arrived
        while the original held its key (0 to 7); the figure is the sum over the 6 rounds (0 to
        42). The original is the task whose request reaches the provider.
    - To measure first: each test 3 times on the registry story's base commit and 3 times on
      the head of its pull request, on the build host. Per test and commit the median of the
      three figures is the result. The counts need the head of the pull request, not its
      merge: the merge does not wait for this item.

Closed by the measurement and not a question: the store built from nothing per test. It costs
5.55 s in 101 calls of `Metadata::initialize` over the eleven tests; a template would save 33 to
38 ms per store, 3.3 to 3.8 s for the 99 stores that unit tests can reach. The section "Measured
(2026-11-25): not built as written" of
story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run holds the figures.

Not an item here: the share of REG and PARSE2 in the tests under 5 s.
story:where-do-the-1298-s-of-the-tests-under-5-s-go owns it and reads it from the gate job of
the registry story's pull request.

## Acceptance

1. Each of the thirteen items gets exactly one entry under a heading `## Answer` in this story,
   with two exceptions: item 12 gets one entry per timer over 1.0 s of own time and one for
   what is left, and item 13 gets one entry per test. Each figure in an entry names its
   source: the command of the run on the build host and its output, or the job of the forge
   run. An entry is one of five kinds:
   - a story: the cost is in this repository's code and a change that keeps behaviour removes
     at least 1.0 s of it on the build host. The 1.0 s is summed over the eleven tests for
     items 1 to 8 and 12, summed over the 16 tests of `adapters/sql/tests/ess_model.rs` that
     call `schema` for item 9, and taken from the duration of the one test
     `every_page_is_one_the_model_admits` for item 10. The entry names the draft story that
     claims it, created as a draft that decomposes
     story:the-gates-test-step-ends-within-120-s-on-the-forge. For item 13 the 1.0 s does not
     apply. A story is due for a test when its median on the head of the registry story's
     pull request is 0, or is less than half of its median on the base commit. A median of 0
     on the base commit means the test does not meet in its race today, whatever the registry
     change does: a story is due for it too, and the entry says that the registry change did
     not cause it. That draft gives the test an assertion that the overlap happened, or a
     start that makes it happen;
   - the dependency's: the entry names the function and its share and no change in this
     repository. Whether a story is asked of the dependency's repository is the operator's
     decision; the entry lists it for that decision;
   - dropped: the cost is in this repository's code and the removable part is measured under
     1.0 s, or removing it changes what the product verifies. The entry states the figure or
     the check that would be lost. For a test of item 13: the three figures and the median of
     both commits, when no story is due by the rule above;
   - no code: the cost follows the load of the host or the disk (item 11) or is in neither the
     fixture, the adapter nor the client library (item 10). The entry names the run that shows
     it and what it means for how the time statements of the set are read;
   - not taken up: only item 10, when the fixture story is confirmed. The entry says so and
     names that story's pull request.
2. The 1.0 s of statement 1 is 0.85 % of the 117.685 s of run 5 (arithmetic). For items 1 to 8
   and 12 it is read from a run on a head that holds the registry change, by the same timers.
   For items 9 and 10 it is read from the durations nextest prints, before and with a probe of
   the change.
3. Order. Items 1 to 8 and 12 are measured after
   story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process has
   merged, on a head that holds it: that change removes REG and PARSE2 from every figure above.
   Item 13 is counted as soon as that story's pull request has a head, on that head and on its
   base commit. It does not wait for the merge, and the merge does not wait for it; its
   entries may be written first. This story has a `depends_on` edge to the registry story for
   both. Item 10 waits for the outcome of the fixture story and records no edge, because a
   confirmed fixture story ends the item. Items 9 and 11 wait for nothing; their runs on the
   build host are not started while the six runs of the fixture story are made.

## Scope

No file of the repository is written but this story's own file in the planning store, which
gets the section `## Answer`. Recorded scope: none. The files read are
`crates/connectors-host/src/local/metadata.rs`, `crates/connectors-host/src/local/metadata/er.rs`,
the test files of the eleven tests, `adapters/sql/tests/ess_model.rs` and
`adapters/sql/tests/mysql/fixture.rs`. A measurement may add temporary timers or a `cfg(test)`
counter only in a worktree made for that measurement, never in the tree of a story that writes
`er.rs`, `metadata.rs` or a test file; they are taken back, no branch is pushed with them and
no pull request carries them. The line numbers in the items are those of head f9f263a2. They
are read again on the head that holds the registry change before a timer is placed, because
that change and story:four-host-tests-no-longer-wait-out-the-30-s-lock-wait move lines of
`er.rs` and `metadata.rs`.

## Not in this story

- Any change of a test, a fixture or product code.
- REG and PARSE2:
  story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process.
- The hold in the MySQL test fixture:
  story:the-mysql-test-fixture-sends-its-packets-without-the-nagle-hold.
- The tests under 5 s: story:where-do-the-1298-s-of-the-tests-under-5-s-go.
- A change in the dependency's repository.
- The runs follow the folder rules: on the build host, no run of the whole workspace, no timing,
  soak or stress run on the workstation, each test bound to 60 s by the runner, the test step
  bound to 120 s, a pool job to 10 minutes unless it names its time (30 minutes at most), a
  script that only measures starts without the `slot` prefix, no new Python.

## Answer

Entries are added as items are measured. An item without an entry is open.

### Item 9: a story

`ess generate` per test costs 2.18 s on the build host, summed over the 16 tests that call
`schema`: more than the 1.0 s of statement 1.

- Runs, 2026-11-25, build host, a head that holds
  story:the-mysql-test-fixture-sends-its-packets-without-the-nagle-hold, the binary alone:
  `cargo nextest run --locked -p connectors-sql --test ess_model --status-level pass --final-status-level pass`.
  The 17 durations nextest printed, summed per run, with the jobs running in the pool
  (`slot status`) before each run.
- As the code is: 3.295 s (5 jobs), 4.543 s (7 jobs), 3.424 s (6 jobs). Median 3.424 s.
- With a probe of the change (`schema` reads one directory under the target directory, filled
  by one test before the three runs): 0.909 s (5 jobs), 1.242 s (4 jobs), 1.349 s (5 jobs).
  Median 1.242 s. The probe was taken back; no branch holds it.
- The difference of the medians is 2.18 s (arithmetic). The three base runs were made first
  and the three probe runs after them, not in turns.
- The forge, read first as the item asks: in the test step of four "repository gate" jobs (pull
  requests 302, 303, 304 and 307) five of the 16 tests ended in 0.061 to 0.088 s each, the call
  included. A call there takes at most 0.088 s; 16 calls at most 1.41 s (arithmetic).
- The draft that claims it:
  story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run.
