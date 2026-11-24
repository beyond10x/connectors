---
format: aep.planning-md/3
id: review-result:read-uses-in-memory-adversary-1
kind: review-result
status: active
title: Adversary, round 1, story:settled-read-uses-leave-the-store
relations:
- reviews: story:settled-read-uses-leave-the-store
revision: 1
---
verdict: needs-change

```findings
- file: crates/connectors-host/src/local/metadata/er.rs
  line: 3020
  category: concurrency
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "exit_process() drops every held use of its simulated process id across all stores, and read_invoke, read_invoke_with_distinct_step_clocks, measured_invoke and the three tests this change added all act as process 1, so under `cargo test` threads (.github/workflows/macos-host.yml:60, windows-host.yml:68) one test's exit ends another test's use between capture and dispatch (shown: dispatch answers Conflict, control with a recorded use answers Ok). Fix: give each such test its own process id or key the drop by store as well."
- file: crates/connectors-host/src/local/metadata/er.rs
  line: 4133
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: "Held uses leave `desired` before the 'only the registry clock' (:4169) and 'only runtime rows' (:4145) checks, so an observation persist whose SQL added or released a use and a runtime-state persist whose SQL deleted one all answer Ok (0.76.0 refused; control with a recorded use still refuses) and the held use is then released or gone. The state was constructed; no production writer of those two modes has a statement on registry_uses. Fix: compare the SQL's unrecorded uses with the build snapshot's row images in those two modes and refuse any difference."
- file: docs/local-connection-registry.md
  line: 233
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "'1,000 open read uses of the owner process' omits that unreleased uses recorded by an ended 0.76.0 owner count against the bound until their deadlines pass (Capacity at NOW+12 with bound 2)."
- file: docs/local-er-metadata.md
  line: 132
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "'The retained ReadUse is a concrete owner fact' still describes a recorded use as current; ess/domains/cli.yaml:795 likewise still says a read's ReadUse 'took dispatch (DispatchReadUse)'."
```

Unit: commit e08df9e9fd8 on `story/read-uses-in-memory`, 2026-11-23. No blocker: the production
read path held on all five points of the brief.

Tests the adversary added, all in `crates/connectors-host/src/local/registry/tests.rs`, times
alone at host load about 70:

- `adversary_d5_read_paths_leave_every_table_of_the_store_file_unchanged`, 3.6 s, green
- `adversary_d5_the_next_owner_expires_the_recorded_uses_of_an_ended_0760_owner_once`, 9.4 s,
  green
- `adversary_d5_an_observation_built_before_a_capture_neither_revives_nor_drops_a_use`, 2.8 s,
  green
- `adversary_d5_reads_and_observations_of_one_process_at_once_leave_every_use_released`, 4.3 s,
  green
- `adversary_d5_a_revoke_by_another_process_refuses_the_reuse_and_ends_the_read_in_flight`,
  3.2 s, green
- `adversary_d5_a_persist_that_may_write_only_the_clock_or_runtime_rows_refuses_a_changed_use`,
  5.8 s, red (finding 2)
- `adversary_d5_only_an_admission_holds_a_new_use_and_only_a_captured_one`, 2.1 s, green
- `adversary_d5_a_test_that_ends_process_1_does_not_end_the_uses_of_another_test_thread`,
  11.8 s, red (finding 1)

The five points:

1. Two processes on one state directory: held by test for a revoke by another process. Repair
   and publish in a second process, two CLIs and the served host: held by reading, not run.
2. One process, threads: held by test (nine reads on three threads beside six `admit_read`,
   every use dispatched and released, 0 ReadUse events).
3. A process dying mid-read, and a 0.76.0 store with open ReadUse records: held by test (12
   recorded uses expire once in two batches ahead of the capture; nothing recorded afterwards).
4. A persist built without the lifecycle lock: held by test for every path production reaches.
5. `appended_per_invoke=0`: held by test (every table's row count, content hash and
   `PRAGMA data_version` unchanged over reads, a cancelled read, a reuse and a cancelled
   revalidation).

Suite of 21 tests with `--test-threads 2`: 18 passed, 2 failed (the two red cases), 1 killed at
60 s (`a_read_invoke_against_a_grown_store_replays_it_at_most_once`, which the change does not
touch; story:store-tests-end-within-60s-under-load); 198 s, exit 100.

Not established: how often finding 1 hits a real `cargo test` run; no mutant was run; nothing
was run on the base commit; no test with two real OS processes; clippy was not run on the new
tests.
