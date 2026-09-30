---
format: aep.planning-md/3
id: story:grown-store-replay-test-flake
kind: story
status: implemented
title: The grown-store replay count holds under parallel test load
relations:
- serves: vision:independent-contract-adapters
- supersedes: story:registry-replay-test-flaky-under-load
scope:
- confidence: cited
  path: crates/connectors-host/src/local/metadata/er.rs
- confidence: cited
  path: crates/connectors-host/tests/service.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T15:07:27Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-09-30T15:07:28Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-09-30T16:53:27Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":2}}}
---
## Observed

2026-09-30, adversary run on wave0930b-absent (`cargo test --no-fail-fast -p connectors -p connectors-host`, several
test binaries in parallel): `local::registry::tests::a_read_invoke_against_a_grown_store_replays_it_at_most_once`
failed with "the next read invoke replayed the whole store 2 times" (registry/tests.rs:1645); alone with `--exact` it
passed (49.7 s). The unit under test does not touch the registry. Whether the second replay is a timing-dependent
catch-up fallback (a real cost under load) or a test artifact is not established.

## Acceptance

- Reproduce under load (for example the full host suite with `--test-threads` at the machine's core count, 20 runs)
  and name the path that falls back to a full replay; fix it or bound the test to what the design promises, stating
  which in the story.

## Also flaky (2026-09-30)

`crates/connectors-host/tests/service.rs` `invalid_identifiers_cannot_inject_log_lines_or_terminal_controls` failed at
:377 (`log.contains("operation completed")`) once in a full host run and 1 of 10 isolated reruns on
wave0930b-absent, and passed 5 of 5 isolated reruns afterwards; it also failed once in the 0.18.0 wave. The absent-
operation unit does not touch `service.rs`, `server.rs` or `http.rs`. Suspected: a shared log buffer read before the
line is flushed.
