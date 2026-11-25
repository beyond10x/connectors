---
format: aep.planning-md/3
id: story:where-do-the-82-s-of-the-eleven-store-tests-go-that-the-registry-story-leaves
kind: story
status: draft
title: Where do the 81.7 s of the eleven store tests go that the registry story leaves?
summary: 'A question, not a change: thirteen items the measurement of 2026-11-25 and its critics left open, each with whose code it is and what must be measured first; items 9 and 10 are about the SQL test, item 13 about two tests'' races'
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- depends_on: story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process
revision: 3
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
asks what the registry change takes from the races of two tests.

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
    - `er.start`, `resynchronize` and `complete_snapshot` sum to 12.33 s over the eleven tests.
      Item 8 holds the 8.32 thread-seconds of one test; 4.01 s are in the other ten
      (arithmetic).
    - Not established: how the timers nest in each other and in the seven named costs. So the
      figures of this item are not added to each other, and 21.92 s is the most that has no
      owner.
    - Whose code: not established per timer.
    - To measure first: the timers of run 5 as a tree, each with its own time without the time
      of the timers inside it, on a head that holds the registry change; then whose code each
      timer over 1.0 s of own time is.
13. Two tests whose race depends on how long a persist takes. Read on 2026-11-25, not measured.
    - `adversary_concurrent_issues_within_the_lead_send_one_exchange`
      (`owner/refresh/tests/issue.rs` lines 299 to 345): 16 callers in each of 8 trials start at
      a `Barrier` (line 311) and then sleep 3 ms times their index (line 319), so they arrive 0
      to 45 ms apart. The test asserts one request at the token endpoint per trial (lines 329 to
      343). That also holds when the callers run one after another, and nothing in the test
      shows that two of them overlapped.
    - `adversary_a_concurrent_duplicate_under_a_cap_of_one_is_a_replay_not_rate_limited`
      (`crates/connectors-host/tests/http_standing_grant_adversary.rs` lines 258 to 324): 8
      tasks in each of 6 rounds are spawned in a loop (lines 272 to 287) with no common start.
      The test asserts one provider request per round and accepts an answer of 200 or a
      replayed 502 (lines 298 to 323). It never requires the 502.
    - The other three tests with several threads among the eleven do not depend on it. Two
      start their threads at a `Barrier` (`registry/tests.rs` line 805, `mutations/tests.rs`
      line 497). One holds the provider's answer with a lock until it has counted the waiters
      (`crates/connectors-host/tests/http_standing_grant.rs` lines 957 to 984).
    - Whose code: this repository's test code.
    - Not established: how many callers overlap today, and whether fewer do when each persist
      is 13.67 ms shorter on the build host (10.63 + 3.04, arithmetic).
    - To measure first: per trial of the first test the number of callers that read the
      expiring generation before the leader published, and per round of the second the number
      of duplicates that arrived while the original held its key; counted with a temporary
      counter on the registry story's base commit and on its merge commit, each test run 3
      times at each.

Closed by the measurement and not a question: the store built from nothing per test. It costs
5.55 s in 101 calls of `Metadata::initialize` over the eleven tests; a template would save 33 to
38 ms per store, 3.3 to 3.8 s for the 99 stores that unit tests can reach. The section "Measured
(2026-11-25): not built as written" of
story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run holds the figures.

Not an item here: the share of REG and PARSE2 in the tests under 5 s.
story:where-do-the-1298-s-of-the-tests-under-5-s-go owns it and reads it from the gate job of
the registry story's pull request.

## Acceptance

1. Each of the thirteen items gets exactly one entry under a heading `## Answer` in this story.
   Each figure in an entry names its source: the command of the run on the build host and its
   output, or the job of the forge run. An entry is one of five kinds:
   - a story: the cost is in this repository's code and a change that keeps behaviour removes
     at least 1.0 s of it, summed over the eleven tests on the build host. The entry names the
     draft story that claims it, created as a draft that decomposes
     story:the-gates-test-step-ends-within-120-s-on-the-forge. For item 13 the 1.0 s does not
     apply: a story is due when, in any of the 3 runs on the merge commit, a test has no trial
     or round with an overlap, or when the count of overlaps fell to less than half of the base
     commit's. That draft gives the test a common start or an assertion that the overlap
     happened;
   - the dependency's: the entry names the function and its share and no change in this
     repository. Whether a story is asked of the dependency's repository is the operator's
     decision; the entry lists it for that decision;
   - dropped: the cost is in this repository's code and the removable part is measured under
     1.0 s, or removing it changes what the product verifies. The entry states the figure or
     the check that would be lost. For item 13: the counts of both commits, when no story is
     due by the rule above;
   - no code: the cost follows the load of the host or the disk (item 11) or is in neither the
     fixture, the adapter nor the client library (item 10). The entry names the run that shows
     it and what it means for how the time statements of the set are read;
   - not taken up: only item 10, when the fixture story is confirmed. The entry says so and
     names that story's pull request.
2. The 1.0 s of statement 1 is 0.85 % of the 117.685 s of run 5 (arithmetic). It is read from a
   run on a head that holds the registry change, by the same timers.
3. Order. Items 1 to 8, 12 and 13 are measured after
   story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process has
   merged, on a head that holds it: that change removes REG and PARSE2 from every figure above,
   and item 13 compares its merge commit with its base commit. This story has a `depends_on`
   edge to it. Item 10 waits for the outcome of the fixture story and records no edge, because
   a confirmed fixture story ends the item. Items 9 and 11 wait for nothing.

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
