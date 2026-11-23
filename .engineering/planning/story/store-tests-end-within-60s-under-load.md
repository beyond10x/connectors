---
format: aep.planning-md/3
id: story:store-tests-end-within-60s-under-load
kind: story
status: draft
title: Five store tests end within 60 s at host load
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
- depends_on: story:settled-read-uses-leave-the-store
scope:
- confidence: cited
  path: crates/connectors-host/src/local/registry/store_cost_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/tests.rs
- confidence: cited
  path: crates/connectors-host/tests/metadata_reopen_adversary.rs
revision: 4
---
## Defect

Five tests of `connectors-host` run longer than 60 s on this workstation at host load 68 to 85
(20 CPUs), so cargo-nextest kills them (`.config/nextest.toml`: `slow-timeout` 30 s,
`terminate-after` 2) and a run over their modules exits 100. The budget of
`~/beyond10x/AGENTS.md` ("Tests stay fast") is 60 s per test and 120 s per test step.

Measured 2026-11-23 by the implementor of
story:entity-runtime-0330-records-name-definition-by-digest, each pair run at the same time
straight from the two lib test binaries, without the 60 s kill, all exit 0:

| Test | Entity Runtime 0.32.0 (0.75.0) | Entity Runtime 0.33.0 (pin branch) |
|---|---|---|
| `store_cost_tests::a_reopen_after_its_own_write_refuses_every_blob_altered_since` | 114.67 s | 128.35 s |
| `registry::tests::a_read_invoke_against_a_grown_store_replays_it_at_most_once` | 122.62 s | 64.92 s |
| `registry::tests::a_reconnect_below_the_refresh_retention_bound_is_not_refused_capacity` | 81.66 s | 105.44 s |
| `registry::tests::concurrent_revoke_repair_publication_and_read_dispatch_share_one_fence` | 114.16 s | 130.18 s |

The fifth, `metadata_reopen_adversary a_reopen_refuses_every_altered_blob_a_full_replay_refuses`,
ran alone at the pin in 57.01 s and was killed at 60 s in both runs of the filterset; not
measured on the base.

The filterset of that story's acceptance line 8 (76 tests) took 228 s and 239 s, exit 100, with
these five killed. Slowest passing: `a_reconnect_at_the_retention_bound_is_refused_capacity`
36.2 s, `cold_owner_start a_cold_read_without_a_connection_waits_for_a_slow_owner_start` 31.6 s.

Not known: their times on an idle machine and on the forge's runner; the forge's repository
gate passed with them on 0.75.0 (pull request 283).

Earlier stories of the same class, both implemented: story:owner-path-tests-hold-under-load,
story:grown-store-replay-test-flake.

## Acceptance

- Each of the five tests is timed alone on an idle machine and at host load of at least 60, on
  `main`; the two times are in the report.
- Each ends within 60 s at that load: by a smaller store or fewer iterations that still shows
  what the test shows (stated per test), or by a shared fixture built once per run (a saved
  store under `target/` keyed by the digest of its sources; a `OnceLock` is not that under
  cargo-nextest), not by a longer timeout.
- The filterset of story:entity-runtime-0330-records-name-definition-by-digest, acceptance
  line 8, exits 0 within 120 s at that load.

## Out of scope

What each test asserts.
