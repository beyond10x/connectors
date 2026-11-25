---
format: aep.planning-md/3
id: story:eight-timing-tests-have-no-job-and-none-ends-within-60-s
kind: story
status: draft
title: Eight timing tests have no job and none ends within 60 s
relations:
- informed_by: story:the-lane-slow-runs-the-ignored-timing-family
- serves: vision:independent-contract-adapters
- depends_on: story:the-lane-slow-runs-the-ignored-timing-family
revision: 1
---
## Defect

Eight ignored tests measure a time or a cost, no job runs them, and none ends within 60 s. Until
story:the-lane-slow-runs-the-ignored-timing-family they were `Timing` entries without a
prerequisite in the table of `crates/connectors-build/src/ignored.rs`, and no job selected that
family. That story moves them to `Deferred`, so that the lane `slow` can end a test at 60 s. They
stay unrun, as before.

Measured on 2026-11-25 for that story:

- The lane's command with the eight still in the family, on the build host of the shared pool:
  six were ended at 60 s and two failed at once.
- Each of the six alone, with a limit of 600 s, on a workstation with a load average of 14 to 27.

The eight, with the time or the failure of each:

1. `connectors-host`, `local::registry::store_cost_tests::two_revalidations_of_a_grown_store`:
   133.1 s.
2. `connectors-host`, `local::registry::store_cost_tests::batch_cost_by_subject`: 178.7 s.
3. `connectors-host`, `local::registry::store_cost_tests::read_invoke_cost_by_store_size`:
   186.6 s.
4. `connectors-host`, `local::registry::store_cost_tests::fresh_open_passes_of_a_blob_heavy_store`:
   266.9 s.
5. `connectors-host`, `local::registry::tests::read_invoke_metadata_time`: 468.8 s.
6. `connectors`, `cold_owner_start`,
   `a_cold_describe_and_read_of_a_large_store_answer_within_60_seconds`: not ended at 600 s. It is
   the one of the eight that asserts a time (60 s).
7. `connectors-host`, `local::registry::store_cost_tests::first_owner_open_of_a_grown_store`:
   fails at once. It unwraps the variables `CONNECTORS_STORE_COST_STORES` and
   `CONNECTORS_STORE_COST_EVENTS`, which no job sets. Its comment gives the command of a person
   who has a saved store. Its time with such a store was not measured.
8. `connectors-host`, `local::registry::store_cost_tests::owner_start_of_a_grown_store`: fails at
   once, for the same two variables. Its time with a saved store was not measured.

Tests 1 to 5, 7 and 8 print costs. Tests 1 to 5 exited 0 when run alone.

## Change

Not decided. This draft holds the eight under one id. For each of the eight one of these applies:

- It gets a job: a scheduled workflow, outside the lane `slow`, with a time of its own. Which
  runner, and how often, is open.
- It is cut down to end within 60 s and moves into the lane under a `slow_lane_` name. For test 6
  that means a smaller store or a bound other than 60 s.
- It is removed, when nobody reads what it prints.
- For tests 7 and 8: they stay tools a person starts with a saved store, and their entry says so
  in place of `Deferred`. That needs a family or a helper mark the table does not have today.

## Acceptance

Draft. To be written with the choice above. What holds for any choice: no entry of the eight is
`Deferred` with this story's id after it, and the lane `slow` keeps its limit of 60 s per test.

## Scope

- `crates/connectors-build/src/ignored.rs`: the eight entries.
- `crates/connectors-host/src/local/registry/store_cost_tests.rs`, the test
  `read_invoke_metadata_time` of `crates/connectors-host/src/local/registry/`, and
  `apps/connectors/tests/cold_owner_start.rs`: by the choice.
- A workflow file, when a job is chosen.

## Not in this story

- The lane `slow` and its rule: story:the-lane-slow-runs-the-ignored-timing-family.
- The ninth `Timing` entry of the table, of `connectors-catalog-provider::main`, which needs the
  Secret Service and the CLI.
- Not established: the times of the eight on a runner of the forge or on an idle host; whether
  test 6 ends at all; who reads the costs tests 1 to 5 print.
- The next measurement of the eight runs on the forge or on the build host of the shared pool, not
  on a workstation.
