---
format: aep.planning-md/3
id: story:the-entity-runtime-registry-and-the-definition-bundle-are-built-once-per-process
kind: story
status: draft
title: 'The Entity Runtime registry is built once per process: 108.0 s become an estimated 76 s'
summary: Every persist and fresh open of the metadata store parses the compiled-in bundle and builds the registry again; one process-wide value replaces 2,729 builds and 2,293 second parses in eleven tests
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- supersedes: story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run
scope:
- confidence: cited
  path: crates/connectors-host/src/local/metadata/er.rs
revision: 4
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
5. The build runs inside the existing test seam `timed("er.definitions_build", …)` (lines 3187
   to 3204). Outside `cfg(test)` `timed` only calls its closure, so the product gets no counter.

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
  definition.
- (c) Who relies on a fresh registry per call: nobody found. `registry()` has five callers, all
  in `er.rs`: three move the value into a facade or a provisioning call, two borrow it; none
  changes it. `Registry::new()` and `register` occur in the host crate only in `registry()`
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

Not measured: the memory the shared value holds for the life of the process.

## Acceptance

1. A new unit test in the test module of `er.rs`,
   `persists_and_fresh_opens_build_the_definitions_at_most_once`, fails before point 1 and passes
   after it. On one thread it initializes a store in a temporary directory, empties the counts
   with `take_costs()`, makes 5 persists that each change a record, then 3 opens that each act
   as a new process (`simulate_process`, line 3000). It asserts that `fresh_opens()` rose by 3
   and that `take_costs()` counts at most 1 call of `er.definitions_build`. The red run is the
   test with the seam of point 5 on today's uncached function: at least 8 calls. "At most 1" and
   not "1": under plain `cargo test` the tests of the binary are threads of one process, and
   another thread may have built the value. The test holds no time window. Run:
   `cargo nextest run -p connectors-host -E 'test(<name of the test>)'`.
2. Behaviour: the tests of `connectors-host` that touch the store pass unchanged. The pull
   request changes no test file and no assertion; its only test change is the test of
   statement 1. The run that shows it: the test step of the forge's "repository gate" job of the
   pull request, and before the push the eleven tests above and the tests of `local::metadata`
   on the build host, each under the 60 s bound.
3. Kept runs: the pull request changes no loop count, round count or trial count in the eleven
   tests (43 cases in the first, counted in the source: 1 + 10 + 4 + 1 + 5 + 20 + 2).
4. Time: the durations of the eleven tests in the test step of the forge's "repository gate" job
   of the pull request sum to at most 92 s. Before: 108.0 s. Estimate: 76 s. The derivation and
   the handling of variance follow below.

Derivation of statement 4:

- Estimate: each test's forge duration times its R+P share on the build host gives a saving of
  32.0 s (arithmetic over the table: 6.20 + 6.42 + 1.55 + 2.84 + 2.34 + 2.15 + 0.67 + 2.30 +
  2.81 + 2.77 + 1.92). 108.0 − 32.0 = 76.0 s. What stays is one build per test process (11 ×
  10.63 ms = 0.12 s, arithmetic) and the clones (not measured).
- The old rule of the set, estimate times 1.5, gives 114 s. That is above "before" and holds
  nothing.
- Proposed rule: "before" minus half the estimated saving: 108.0 − 16.0 = 92 s.
- Variance: one test ran 17.6, 19.7, 24.1 and 30.3 s on the build host in four runs, so the
  bound is on the sum of eleven, not on one test. The sum of the eleven on the build host was
  117.7 s in run 5 and 127.6 s in the earlier solo run (136.889 − 9.293, arithmetic): 9.9 s
  apart. The margin of 16.0 s is wider than that. That the forge varies as little is a reading:
  the forge's "before" is one job.
- So "before" is read again: the pull request names the durations of the eleven tests in the
  newest gate job of its base commit. When their sum is under 108.0 s, the bound is that sum
  minus 16.0 s.
- A job over the bound is not run again until it passes. The pull request then lists the eleven
  durations of both jobs, the story stays open, and the numbers go back to the operator.
  Statement 1 does not depend on time and holds the change on its own.

No test of the eleven is over 60 s before or after (largest: 17.5 s on the forge).

## Scope

- `crates/connectors-host/src/local/metadata/er.rs`: the shared value and its build (point 1),
  `registry()`, `persist_batch`, `record_writer_for_tests`, the seam of point 5, and the new
  test in the test module.

No other file is written. Stories of the set (those that decompose
`the-gates-test-step-ends-within-120-s-on-the-forge`) that name this file: only
`eighteen-tests-take-a-template-store-or-jail-built-once-per-run`, which is not built. Stories
outside the set that name it and are not finished: `connections-list-answers-through-the-owner`
(active), `post-commit-metadata-failure-is-outcome-unknown` (draft),
`spec-models-operation-admission` (draft).
`four-host-tests-no-longer-wait-out-the-30-s-lock-wait` writes `metadata.rs`, not this file.

`aep plan artifact waves --kind story --status draft` (2026-11-25) puts this story in wave 1. It
reports the file as shared with two drafts outside the set, both with inferred scope:
`mcp-outbound-stdio-runtime` (wave 4) and `spec-models-operation-admission` (wave 5). No edge
records an order. This story goes first; a later story that writes the file starts from a head
that holds this change.

## Not in this story

An "open question" with a number is that item of
story:where-do-the-82-s-of-the-eleven-store-tests-go-that-the-registry-story-leaves.

- The write of the Entity Runtime batch (30.29 s of the eleven tests): open question 1.
- The read after the batch, the catch-up of a pooled handle, the projection and the validation:
  open questions 2 to 5.
- The migrate that has nothing to migrate and still persists: open question 6. This story takes
  5.61 s of its 8.07 s away without touching it; 2.45 s stay.
- The poll of the lifecycle lock and the fresh opens of `adversary_concurrent_issues…`: open
  questions 7 and 8.
- The template store and the shared schemas of the old story: not built, see its section
  "Measured (2026-11-25): not built as written".
- The tests under 5 s: whether they carry the same share is not measured.
  `where-do-the-1298-s-of-the-tests-under-5-s-go` holds that question; open question 11 names
  the measurement.
- An interface of Entity Runtime that takes a shared registry, so that a fresh open needs no
  clone: the dependency's code, not asked for here.
- The third parse of the bundle in `model_digest()` (line 1040): it is already cached per
  process.
- The parse sites of the tests (`registry_of`, `adversary_w3_tests.rs`): unchanged.
- `every_page_is_one_the_model_admits`: the story of the MySQL test fixture.
