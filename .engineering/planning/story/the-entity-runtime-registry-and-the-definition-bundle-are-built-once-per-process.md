---
format: aep.planning-md/3
id: story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process
kind: story
status: implemented
title: 'The Entity Runtime registry is built once per process: 108.0 s become an estimated 76 s'
summary: Every persist and fresh open of the metadata store parses the compiled-in bundle and builds the registry again; one process-wide value replaces 2,729 builds and 2,293 second parses in eleven tests
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- supersedes: story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run
- depends_on: story:the-specification-cites-er-rs-by-function-name-not-by-line
scope:
- confidence: cited
  path: crates/connectors-host/src/local/metadata/er.rs
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T11:18:22Z", actor: "human:operator", revision: 7}
- {from: "proposed", to: "active", at: "2026-11-25T11:38:54Z", actor: "human:operator", revision: 8}
- {from: "active", to: "implemented", at: "2026-11-26T20:08:48Z", actor: "human:operator", revision: 11, decided_on: {"recorded":{"test_result":1}}}
---
## Defect

Every persist and every fresh open of the metadata store parses the compiled-in definition
bundle and builds the Entity Runtime registry from it again. In the eleven host tests below that
is 30.6 % of their time on the build host.

The eleven tests. "Forge" is the duration nextest printed in the forge's gate job 138270416028 of
pull request 291 (2026-11-24), as the story
`eighteen-tests-take-a-template-store-or-jail-built-once-per-run` lists them. "Host" is the wall
time of run 5 of the measurement of 2026-11-25: one test at a time on the build host, head
d5cf22480, with timers. "R+P" is the time of that run inside the two costs named below. Unit
tests are under `crates/connectors-host/src/local/`, the last two under
`crates/connectors-host/tests/`.

1. `owner/governed/tests/bounded_realization_security.rs`,
   `bounded_owner_serves_exactly_the_realizations_the_projection_serves`
2. `owner/refresh/tests/issue.rs`,
   `adversary_concurrent_issues_within_the_lead_send_one_exchange`
3. `registry/tests.rs`,
   `a_reconnect_below_the_refresh_retention_bound_is_not_refused_capacity`
4. `owner/mutation/tests.rs`, `final_audit_recovery_preserves_every_live_business_result`
5. `registry/expiry_batch_tests.rs`,
   `due_expiries_past_the_former_32_member_bound_join_the_callers_batch`
6. `registry/tests.rs`,
   `concurrent_revoke_repair_publication_and_read_dispatch_share_one_fence`
7. `registry/tests.rs`, `a_read_invoke_against_a_grown_store_replays_it_at_most_once`
8. `mutations/tests.rs`,
   `abort_and_gate_compete_and_recovery_never_reconstructs_a_send_receipt`
9. `registry/store_cost_tests.rs`,
   `a_reopen_after_its_own_write_refuses_every_blob_altered_since`
10. `tests/http_standing_grant_adversary.rs`,
    `adversary_a_concurrent_duplicate_under_a_cap_of_one_is_a_replay_not_rate_limited`
11. `tests/http_standing_grant.rs`,
    `replays_waiting_on_an_original_hold_no_slot_and_at_most_four_wait_per_key`

| Test | Forge s | Host s | R+P s | R+P share |
|---|---|---|---|---|
| 1 | 17.5 | 17.555 | 6.222 | 35.4 % |
| 2 | 16.6 | 21.527 | 8.329 | 38.7 % |
| 3 | 9.6 | 7.691 | 1.235 | 16.1 % |
| 4 | 9.3 | 11.265 | 3.440 | 30.5 % |
| 5 | 9.1 | 10.965 | 2.818 | 25.7 % |
| 6 | 8.9 | 7.309 | 1.768 | 24.2 % |
| 7 | 8.7 | 7.624 | 0.584 | 7.7 % |
| 8 | 7.2 | 7.466 | 2.382 | 31.9 % |
| 9 | 6.9 | 6.655 | 2.711 | 40.7 % |
| 10 | 7.1 | 9.850 | 3.843 | 39.0 % |
| 11 | 7.1 | 9.778 | 2.649 | 27.1 % |
| Sum | 108.0 | 117.685 | 35.979 | 30.6 % |

The rounded rows of the R+P column sum to 35.981 s; 35.979 s is REG plus PARSE2 from the timers
(29,004.2 + 6,974.3 ms, arithmetic).

The cause, in `crates/connectors-host/src/local/metadata/er.rs` at head d5cf22480:

- REG: `registry()` (line 992) calls `definition_bundle()` (line 1012), which parses the 469,676
  bytes of `entity-runtime-definitions.json` (`wc -c`), registers every definition and runs
  `validate_all`. It is called in every `persist_batch` (line 4334), in every fresh open (`open`,
  line 2484), in `provider_open` (line 901) and in `provision_with_lock` (line 2377).
  Run 5: 2,729 calls, 29,004.2 ms, 10.63 ms per call (arithmetic). Inside it: `register`
  19,882.7 ms, the parse 8,154.3 ms (all 5,022 parses took 15,128.6 ms; minus PARSE2,
  arithmetic), `validate_all` 883.3 ms.
- PARSE2: `persist_batch` parses the bundle a second time (line 4328) to build the map of
  commands. Run 5: 2,293 calls, 6,974.3 ms, 3.04 ms per call (arithmetic).
- A persist that changes nothing pays both: `persist_batch` returns on `actions.is_empty()` at
  line 4442, after the two builds.
- Of the 2,729 REG calls, 2,293 are in `persist_batch` and 436 outside it (arithmetic: 2,729
  minus the PARSE2 count).

How it was measured: nested `Instant` timers around the calls, keyed by thread and timer path,
in a temporary instrumentation that was taken back; and a temporary in-process sampler (SIGPROF
at 100 Hz), because `perf` is refused on the build host. The sampler agrees: in test 1
`er::registry` holds 30.7 % of the CPU samples (1,614 samples) and the dependency's
`record_form`, which `register` calls, 21.3 %.

Readings and arithmetic:

- Read from run 5: wall times, call counts, milliseconds per timer.
- Arithmetic: per-call costs, shares, sums, the 436.
- The R+P figures of the tests that run several threads (tests 2, 6, 8, 10 and 11) are
  thread-seconds. In test 2 the process used 24.82 s of CPU in 21.527 s of wall, so its wall
  saving can be smaller than 8.329 s.
- A reading, not measured: the forge has the same shares as the build host.
- The product pays the same per persist: 13.67 ms on the build host (10.63 + 3.04, arithmetic).
  No product path was timed outside the tests.

## Change

Product behaviour does not change: same records, same errors, same order. All points are in
`crates/connectors-host/src/local/metadata/er.rs`.

1. One process-wide value holds the registry and the map of commands. It is a `static`
   `OnceLock` built on first use from one parse of the bundle, with the checks `registry()`
   makes today in the same order: the format string, `entry.name == entry.definition.entity`,
   `register`, `validate_all`. A failed build is stored and answered as
   `Failure::MetadataUnavailable` on every call, the form `model_digest()` already uses
   (lines 1035 to 1045).
2. `persist_batch` (lines 4328 to 4334) borrows the shared registry and the shared map of
   commands. It parses nothing and builds nothing.
3. `open` (line 2484), `provider_open` (line 901) and `provision_with_lock` (line 2377) take a
   clone of the shared registry. `registry()` keeps its signature and returns that clone.
4. `record_writer_for_tests` (line 1169, test support) borrows the shared value as
   `persist_batch` does.
5. Two test seams, both through the existing `timed` (lines 3187 to 3204). Outside `cfg(test)`
   `timed` only calls its closure, so the product gets no counter.
   `timed("er.definitions_build", …)` is called inside the closure handed to the `OnceLock`,
   around the build: a call that finds the value counts nothing.
   `timed("er.bundle_parse", …)` is around the parse in `definition_bundle()`. Today
   `definition_bundle()` is called at eleven places besides `registry()` (`grep -n`): lines
   1170 (`record_writer_for_tests`) and 4328 (`persist_batch`), which points 2 and 4 remove,
   and nine in the test module (lines 6119, 6460, 6539, 6784, 6877, 7078, 7534, 7591 and
   8288), which stay and parse as before. After points 1, 2 and 4 the build is its only caller
   outside the test module. The new test calls none of the nine.

Established by reading:

- (a) Where the bundle comes from: it is compiled in with `include_bytes!` (line 1013). It
  cannot change within a process, so the cached value has no key. `registry()` takes no argument
  and reads no file, environment variable or clock: every call in one process returns the same
  value or the same error.
- (b) Who owns the registry: `persist_batch` only borrows it (`&runtime_registry` at lines 4349,
  4360, 4381, 4397, 4412 and 4447, and `&commands` beside it). A fresh open needs an owned
  value: `RecordedProviderFacade::start_with_read_policy` takes `registry: Registry` by value
  (Entity Runtime 0.33.1, `crates/entity-eventlog/src/facade.rs` line 185) and moves it to its
  worker thread. `Registry` derives `Clone` (`crates/entity-core/src/registry.rs` line 108). The
  cost of a clone is not measured. A clone skips the JSON parse, the validation and the
  serialize-and-parse round trip of `record_form`; it copies two `EntityDefinition` values per
  definition. The clone is forced: the facade takes the value, the field of `Registry` is
  private, and the dependency has no shared form. A reading of its size: the bundle holds 28
  definitions and 73 commands; one parse of it, which allocates each definition once, takes
  3.04 ms. A clone that costs one to two such parses makes the 436 clones of run 5 1.3 to 2.7 s
  (arithmetic), 1.1 to 2.3 % of 117.685 s and inside the 16.0 s margin of statement 4.
- (c) Who relies on a fresh registry per call: nobody found. Outside its test module
  `registry()` has five callers, all in `er.rs`: three move the value into a facade or a
  provisioning call, two borrow it; none changes it. The test module calls it at eight more
  places (lines 6127, 6467, 6547, 6876, 7084, 7562, 7598 and 8299); each borrows the value it
  gets, so the kept signature serves them unchanged. No test expects `registry()` to fail.
  `Failure` is `Copy` and `MetadataUnavailable` carries no message (`local/mod.rs` lines 48 to
  58), so a stored failure is the same value on every call. The build reads only the
  compiled-in bytes, so a failure cannot be one that a later call would not see. `Registry::new()` and `register` occur in the host crate only in `registry()`
  itself (lines 997 and 1003), in the test helper `registry_of` (line 5562) and in
  `adversary_w3_tests.rs` (lines 24 and 26). The tests that need other definitions
  (`newer_registry`, `future_registry`, lines 5519 and 5541) build their own registry from the
  JSON and do not call `registry()`.
- (d) Thread safety: `Registry` is a `BTreeMap` of `ValidatedDefinition`, which holds two
  `EntityDefinition` values. Its only cell (`assignment_evaluations`, `registry.rs` line 24) is
  under the dependency's own `cfg(test)` and is absent in a build of this repository.
  `definition.rs` of `entity-core` has no `Cell`, `RefCell`, `Rc` or raw pointer (`grep`).
  `CommandEntry` holds strings and vectors of strings. The compiler confirms the reading: a
  `static OnceLock<T>` builds only when `T` is `Send` and `Sync`. The registry is already moved
  to another thread today.
- (e) Tests that race, read for what makes their threads overlap. Five of the eleven run
  several threads (tests 2, 6, 8, 10 and 11).
  - Test 11 shows its overlap: it holds the provider's answer with a lock until it has counted
    the waiters (`tests/http_standing_grant.rs` lines 957 to 984). The duration of a persist
    does not change that.
  - Tests 6 and 8 start their threads together at a `Barrier` (`registry/tests.rs` line 805,
    `mutations/tests.rs` line 497). A common start is not a shown overlap: both assert a result
    that also holds when the threads run one after another (test 8:
    `assert_ne!(gate.is_ok(), abort.is_ok())`, line 509; test 6: each outcome is `Ok`,
    `Conflict` or `Revoked`, lines 838 to 854).
  - Test 2 starts its 16 callers at a `Barrier` (`owner/refresh/tests/issue.rs` line 311) and
    then spreads them by a sleep of 3 ms times their index (line 319). Test 10 spawns its 8
    tasks with no common start (`tests/http_standing_grant_adversary.rs` lines 272 to 287).
    Both assert a result that also holds one after another.
  - So no overlap is shown in tests 2, 6, 8 and 10, today or after this change. A shorter
    persist can make fewer callers overlap; tests 2 and 10, whose callers arrive apart, are the
    ones most exposed by reading. How many overlap today is not measured.
  - This story changes none of the four tests and no product behaviour; what can change is how
    often a test reaches the race it was written for. Open question 13 counts the overlaps on
    this story's base commit and on the head of its pull request, and names the story that is
    due when they fall. That count can be made before this story's pull request merges, and
    the merge does not wait for it.

Not measured: the memory the shared value holds for the life of the process.

## Acceptance

1. A new unit test in `mod tests` of `er.rs`,
   `persists_and_fresh_opens_build_the_definitions_once_per_process`, fails before points 1 and 2
   and passes after them. It reads two kinds of the cost counter (`take_costs()`, which counts
   per thread): `er.definitions_build`, one per run of the closure that builds the shared value,
   and `er.bundle_parse`, one per parse in `definition_bundle()` (point 5). The test:
   - takes `own_processes()`, as the tests of fresh opens do;
   - initializes a store in a temporary directory (`Metadata::initialize`) and calls
     `registry()` once, so that the shared value exists whichever thread built it;
   - empties the counts with `take_costs()`;
   - makes 5 persists that each change a record: `runtime::state::State::suppress` on that
     store with alternating values, the call the test at line 5617 makes once;
   - makes 3 opens that each act as a new process: `simulate_process` with a new id, then
     `Metadata::update`;
   - initializes a second store in another temporary directory and makes one persist there;
   - asserts that `fresh_opens()` rose by at least 3;
   - asserts that `take_costs()` holds 0 calls of `er.definitions_build` and 0 calls of
     `er.bundle_parse`;
   - starts one more thread that first enters the test's namespace of process ids
     (`own.namespace().enter()`, `ProcessNamespace::enter` at line 3030; a new thread does not
     inherit it), then makes one persist on the first store and one open of it as a further new
     process, and returns its own `take_costs()`: 0 calls of both kinds there too.

   What the counts exclude. A value kept per handle or per simulated process is built again in
   the 3 opens. A value kept per store is built again for the second store. A value kept per
   thread is built once on the new thread. A `persist_batch` that kept its own parse counts 6
   parses.

   The red run is point 5 alone on today's code: the two `timed` calls around today's build and
   today's parse, nothing else changed. Expected by reading, not run: at least 8 builds (5
   persists and 3 opens) and at least 13 parses (two per persist, one per open) on the test's
   thread before the second store. The pull request's body carries the command of the red run
   and the failing assertion with its counts.

   The test holds no time window. Run:
   `cargo nextest run -p connectors-host -E 'test(persists_and_fresh_opens_build_the_definitions_once_per_process)'`.
2. The diff: `git diff --stat` of the pull request against its base names one file under
   `crates/`, `er.rs`. In that file `mod tests` gains the new test and changes nowhere else,
   and the only test-support code changed outside it is `record_writer_for_tests` (point 4).
   So no assertion and no loop, round or trial count of the eleven tests changes (43 cases in
   the first, counted in the source: 1 + 10 + 4 + 1 + 5 + 20 + 2).
3. Behaviour: the tests of `connectors-host` pass. The run that shows it: the test step of the
   forge's "repository gate" job of the pull request, and before the push the eleven tests above,
   the tests of `local::metadata` and the tests of the module `cli_spec_mapping_adversary` of
   `connectors-build` (one of them reads two functions of `er.rs`) on the build host, each
   under the 60 s bound.
4. Time: the durations of the eleven tests in the test step of the forge's "repository gate" job
   of the pull request sum to at most "base" minus 16.0 s. "Base" is the sum of the same eleven
   durations in the newest "repository gate" job on main that ran the test step, at or before
   the pull request's base commit. The base commit is the commit of main that the job tested
   the branch with: the gate runs on `pull_request`, so it checks out the merge of the branch
   with main, and the first parent of that merge is the base (read from the job's checkout
   step). The pull request's body names that commit, the base job and its commit. When
   main is merged into the branch again, "base" is read again for the new base commit, so a
   pull request that merged in between (the fixture story's, for one) is in "base" and not in
   the saving. For the base of the plan, 108.0 s (job 138270416028), the bound is 92.0 s.
   Estimate: 76 s.

Derivation of statement 4:

- Estimate: each test's forge duration times its R+P share on the build host gives a saving of
  32.0 s (arithmetic over the table: 6.20 + 6.42 + 1.54 + 2.84 + 2.34 + 2.15 + 0.67 + 2.30 +
  2.81 + 2.77 + 1.92 = 31.96). 108.0 − 32.0 = 76.0 s. What stays is one build per test process
  (11 × 10.63 ms = 0.12 s, arithmetic) and the clones (not measured; (b) above gives 1.3 to
  2.7 s as a reading).
- The old rule of the set, estimate times 1.5, gives 114 s. That is above "before" and holds
  nothing.
- The rule here: "base" minus half the estimated saving, 16.0 s. It is one formula for every
  base. A base of 108.0 s gives 92.0 s; a base of 100.0 s gives 84.0 s; a base of 130.0 s gives
  114.0 s.
- Why "base" is read again and not taken as 108.0 s: the forge's 108.0 s is one job of
  2026-11-24, and main has changed since.
- A commit on main that changed only planning files has a gate job without a test step. Then
  the nearest earlier commit on main whose gate job ran the test step gives "base", and the
  pull request names that commit and its job.
- Variance: one test ran 17.6, 19.7, 24.1 and 30.3 s on the build host in four runs, so the
  bound is on the sum of eleven, not on one test. The sum of the eleven on the build host was
  117.7 s in run 5 and 127.6 s in the earlier solo run (136.889 − 9.293, arithmetic): 9.9 s
  apart. The margin of 16.0 s is wider than that. That the forge varies as little is a reading:
  each forge figure is one job.
- When the pull request's job is over the bound, it is not run again as a remedy. The pull
  request then lists the eleven durations of both jobs. It may still merge when statements 1 to
  3 hold: they hold the change without a clock. The story then stays active, is not moved to
  implemented, and the two lists go to the operator.

No test of the eleven is over 60 s before or after (largest: 17.5 s on the forge).

## Scope

- `crates/connectors-host/src/local/metadata/er.rs`: the shared value and its build (point 1),
  `registry()`, `persist_batch`, `record_writer_for_tests`, the two seams of point 5, and the
  new test in the test module.

No other file is written. That holds only behind
story:the-specification-cites-er-rs-by-function-name-not-by-line. At head f9f263a2
`ess/domains/cli.yaml` line 2218 cites `er.rs:5095-5097`, the test
`the_er_rs_citation_lands_on_the_cursor_expiry_path` of `connectors-build` requires the text
`ConnectionListCursor` in those lines, and every point of this change is above line 5095. That
test goes red when line 5095 moves down by 3 lines or more or up by 2 or more; by how many
lines this change moves it is not known before it is written. Without that story this pull
request is then red in the gate, or it writes `cli.yaml` and the record
`crates/connectors-build/generated/metadata-emissions/inputs.json`, which holds a sha256 of
`cli.yaml`. This story has a `depends_on` edge to that story and starts from a head that holds
it. No committed record holds a digest of `er.rs` itself (`git grep` for the path under
`crates/connectors-build/`: none).

`entity-runtime-definitions.json`, the bundle beside `er.rs`, is read and not written. A story
that regenerates it before this one changes the cost of one build, not the counts of statement
1 and not the formula of statement 4, which reads its "base" from main.

Who else names `er.rs`, in two lists:

- By recorded scope (`aep plan artifact waves --kind story --status draft`, 2026-11-25), two
  drafts outside the set, both inferred: `spec-models-operation-admission` records the file and
  the bundle; `mcp-outbound-stdio-runtime` records the directory
  `crates/connectors-host/src/local`.
- By the path in a story's text, two more: `connections-list-answers-through-the-owner`
  (active) cites lines of `er.rs` as what it read; its recorded scope has nine paths and not
  this file, so it does not write it. `post-commit-metadata-failure-is-outcome-unknown` (draft)
  cites lines 4115 to 4207, the region of `persist` and `persist_batch`, and records no scope.
- The lists differ because the tool reads only the `scope` of a draft's frontmatter.
- Stories of the set that write the file: none but this one. The archived
  `eighteen-tests-take-a-template-store-or-jail-built-once-per-run` would have. The parent,
  the two question stories, the citation story and `store-tests-end-within-60s-under-load`
  name it and do not write it.
  `four-host-tests-no-longer-wait-out-the-30-s-lock-wait` writes `metadata.rs`, not this file.

Order with the three drafts: this story goes first, and no edge records that. An edge would
state that their content needs this change, which is not so; their scope is inferred and none
is planned in detail. The rule for them is this sentence: a later story that writes `er.rs`
starts from a head that holds this change, and one that cites lines of `er.rs` reads them
again there.

Order within the set: story:the-specification-cites-er-rs-by-function-name-not-by-line, then
this story, then story:store-tests-end-within-60s-under-load and the two question stories that
read this story's result (each has a `depends_on` edge to this story).

## Not in this story

An "open question" with a number is that item of
story:where-do-the-82-s-of-the-eleven-store-tests-go-that-the-registry-story-leaves.

- The write of the Entity Runtime batch (30.29 s of the eleven tests): open question 1.
- The read after the batch, the catch-up of a pooled handle, the projection and the validation:
  open questions 2 to 5.
- The migrate that has nothing to migrate and still persists: open question 6. This story takes
  5.61 s of its 8.07 s away without touching it; 2.46 s stay (arithmetic).
- The poll of the lifecycle lock and the fresh opens of `adversary_concurrent_issues…`: open
  questions 7 and 8.
- The template store, the shared schemas and the setup step of the old story: not built, see
  its section "Measured (2026-11-25): not built as written".
- A common start or a proof of overlap for tests 2, 6, 8 and 10: open question 13.
- The line citations of `er.rs` in the specification:
  story:the-specification-cites-er-rs-by-function-name-not-by-line, which this story depends on.
- The tests under 5 s: whether they carry the same share is not measured.
  `where-do-the-1298-s-of-the-tests-under-5-s-go` holds that question and reads the answer from
  the gate job of this story's pull request against its base.
- An interface of Entity Runtime that takes a shared registry, so that a fresh open needs no
  clone: the dependency's code, not asked for here.
- The third parse of the bundle in `model_digest()` (line 1040): it is already cached per
  process.
- The parse sites of the tests (`registry_of`, `adversary_w3_tests.rs`): unchanged.
- `every_page_is_one_the_model_admits`: the story of the MySQL test fixture.

## As built

Built on 2026-11-25. One file changed under `crates/`,
`crates/connectors-host/src/local/metadata/er.rs`.

The shared value: `struct Definitions { registry, commands }`, held by
`static DEFINITIONS: OnceLock<Option<Definitions>>` inside `shared_definitions()`. The build
(`build_definitions`) is the old body of `registry()` in its order, then the map of commands. A
failed build is stored as `None` and answered as `Failure::MetadataUnavailable` on every call.
`persist_batch` and `record_writer_for_tests` borrow the value. `registry()` keeps its
signature and returns a clone; `open`, `provider_open` and `provision_with_lock` call it as
before and have no changed line.

Where the code differs from the Change section:

- Point 1, order: the map of commands is built after `validate_all`. Before, `persist_batch`
  built it ahead of `registry()`. Every failure on either path is
  `Failure::MetadataUnavailable`.
- Established (c): `registry()` now has three callers outside the test module, not five; the
  two that borrowed call `shared_definitions()`.
- `model_digest()` stands at lines 1036 to 1046 of the base, not 1035 to 1045.

Statement 1. Every run:
`cargo nextest run -p connectors-host -E 'test(persists_and_fresh_opens_build_the_definitions_once_per_process)'`
on the build host.

- Red, with point 5 alone (the two `timed` seams and the new test): the test failed with
  "(builds, parses) of the definitions over five persists, three fresh opens and a second
  store on this thread", left (17, 30), right (0, 0). The story expected at least 8 builds and
  13 parses before the second store; the test reads its counts once, after the second store.
  By arithmetic over the counts: 13 calls of `persist_batch`, 3 opens and 1 provision make 17
  builds, and 13 × 2 + 3 + 1 make 30 parses. Each `State::suppress` reaches `persist_batch`
  twice, and the second store's `Metadata::initialize` once.
- Green with points 1 to 4: 1 passed, 0.388 s.
- Shown red and taken back: `persist_batch` parsing the bundle itself failed the same
  assertion with (0, 13) (the story said 6 parses; it is 13 for the reason above); the value
  kept in a `thread_local!` failed the assertion of the new thread with (1, 1).
- The test asserts more than the story listed: at least 6 recorded batches on the test's
  thread (`er.execute_batch`), so that the six persists are known to have written; at least
  one fresh open on the new thread; and `fresh_opens()` as a rise from a value taken after the
  first `registry()` call.

Statement 2: under `crates/` the diff names `er.rs` only. Outside `mod tests`: the three new
items `Definitions`, `shared_definitions` and `build_definitions`, `registry()`,
`definition_bundle()` (the seam), `record_writer_for_tests` (three hunks) and `persist_batch`
(seven hunks: the borrow, and the `&` dropped at six call sites). Inside `mod tests`: one hunk,
the new test at the end of the module.

Statement 3, before the push, on the build host, each set once on the final tree:

- the eleven tests in one run side by side: 11 passed. Durations in the order of the list
  above: 12.080, 12.805, 7.547, 6.353, 5.677, 6.470, 8.094, 4.603, 3.911, 4.034 and 5.096 s,
  76.670 s summed (arithmetic). Tests 1 to 7 and 11 are over 5 s. This run is not the figure
  of statement 4: run 5 of the measurement ran one test at a time.
- `test(/^local::metadata::/)` of `connectors-host`: 115 run, 115 passed, none over 5 s.
- `test(/^cli_spec_mapping_adversary::/)` of `connectors-build`: 6 run, 6 passed.
- `cargo fmt --check`, `cargo clippy -p connectors-host --all-targets -- -D warnings` and the
  same lint for `x86_64-pc-windows-gnu` with `-p connectors`: exit status 0 each.

An adversary read of the change, after it was green. No change of behaviour was found: no
caller relies on a fresh registry or changes the one it gets; every failure of the build is
`Failure::MetadataUnavailable` before and after, at the same point of `persist_batch`, `open`
and `provision_with_lock`; the build reads only compiled-in bytes; the crate lints with and
without the feature `test-support`.

One finding, a warning, and what was done about it. The new test stayed green under two
defects its claim excludes: a `persist_batch` that parses the compiled-in bytes with an
expression of its own (as `model_digest()` does), and a `registry()` that registers every
definition again from the shared registry on every call. Its two counts see a parse only in
`definition_bundle()` and a build only in the closure of `shared_definitions()`. Two more tests
close that, so `mod tests` gains three tests where statement 2 said one:

- `adversary_the_bundle_is_read_only_where_the_once_test_counts_it`: one call of
  `definition_bundle()` is one counted parse, and by the text of `er.rs` outside its test
  module and of `er/ledger.rs` the bundle is named at two places, `definition_bundle()` and
  `build_definitions()` are each called at one, a registry is started at one, and each seam
  stands once. It was red under both defects above. It is a check of text: a parse in another
  file, or `Registry::default()`, passes it.
- `adversary_first_use_from_eight_threads_at_once_builds_the_definitions_once`: eight threads
  started together each initialize a store and persist once; among them at most one build,
  and exactly one under nextest. 0.370 s.

Mutants the three tests were run against, each taken back: the value keyed per store path
(the story's test red with (1, 1)), keyed per simulated process (red with (3, 3)), `registry()`
building its own (red with (0, 4)), and the two above. With the two added tests,
`test(/^local::metadata::/)` runs 117 tests and 117 pass.

Read from the forge on 2026-11-26: the forge half of statement 3 and statement 4, from pull
request 313, whose branch merged this story's commit 8df1671f7 with three other stories. The diff
of 8df1671f7 against its first parent names one file outside the planning store, `er.rs`; the
pull request's diff also holds the files of the other three stories.

- The job: "repository gate" job 139714078337 (run 45748988866, attempt 2) of head 606a611d5.
  Attempt 1 (job 139671926182) was cancelled after 90 minutes in `apt-get update`, before the
  gate started; it was not a run again for a time bound.
- Statement 3, the forge half: the job's test step printed `Summary [ 466.033s] 4546 tests run:
  4546 passed (4 slow), 229 skipped`. The three tests of this story passed:
  `persists_and_fresh_opens_build_the_definitions_once_per_process` 0.414 s,
  `adversary_the_bundle_is_read_only_where_the_once_test_counts_it` 0.020 s,
  `adversary_first_use_from_eight_threads_at_once_builds_the_definitions_once` 0.294 s.
- Statement 4, the base: the job's checkout printed `HEAD is now at 826ab34 Merge 606a611d5…
  into e1908013e…`, so the base commit is e1908013e. The newest gate job on main at or before it
  that ran the test step is job 139671125964 (run 45748746486, the push of e1908013e; `gate: test
  step ran 641.4 s (budget 120 s)`).

The eleven durations as nextest printed them (`PASS [ <s>s]`), in the order of the list above:

| Test | Base, job 139671125964, s | Pull request 313, job 139714078337, s |
|---|---|---|
| 1 | 24.690 | 10.755 |
| 2 | 22.677 | 10.403 |
| 3 | 10.987 | 7.089 |
| 4 | 13.692 | 6.062 |
| 5 | 10.959 | 3.934 |
| 6 | 10.517 | 6.035 |
| 7 | 9.851 | 7.626 |
| 8 | 8.610 | 4.128 |
| 9 | 9.034 | 3.675 |
| 10 | 9.218 | 4.098 |
| 11 | 8.810 | 4.420 |
| Sum | 139.045 | 68.225 |

- Bound: 139.045 − 16.0 = 123.0 s (arithmetic). 68.225 s is under it by 54.8 s: statement 4
  holds. Against the plan's base of 108.0 s (bound 92.0 s) it holds too.
- The base job's eleven took 31.0 s more than job 138270416028 of the plan (139.045 − 108.0,
  arithmetic). That the runners differ is a reading of two jobs, not a measurement.
- No test of the eleven is over 60 s in either job (largest: 24.690 s in the base job).

Every statement holds; the story moved to implemented on this record.

Not measured: the cost of a `Registry` clone and the memory the shared value holds.
