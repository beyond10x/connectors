---
format: aep.planning-md/3
id: story:list-tests-copied-cli-busy
kind: story
status: implemented
title: The connectors list tests exec a CLI copy the test process may still hold open
relations:
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-23T20:24:47Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-23T20:24:47Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-23T21:45:51Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Defect

`an_unresolvable_state_directory_does_not_hide_the_configured_instances`
(`apps/connectors/tests/adversary_list_connectors.rs`) failed in the Linux repository gate of pull
request https://github.com/beyond10x/connectors/pull/207, run 40010872724, 2026-10-23, at
`adversary_list_connectors.rs:67:14`: `Os { code: 26, kind: ExecutableFileBusy, message: "Text file
busy" }`. The change under test (the ESS pin move) touches no CLI code.

Cause, read from the code and matching the error: `Installation::new` copies the CLI with `fs::copy`
(`:35`) and writes the stand-in catalog provider with `fs::write` (`private_file`, `:17-20`), both in
the test process. A test thread that forks while one of those files is open for writing hands the
descriptor to its child until the child's exec, and an exec of the file meanwhile fails with
`ETXTBSY`. This is the cause story:ci-timing-flakes-20261018 found for the Kubernetes exec tests.
`apps/connectors/tests/list_connectors.rs:36` has the same `Installation`.

## Outcome

Both list test files put every file they execute in place through a `cp` child, as
`adapters/kubernetes/tests/exec_credential.rs` `install` does, so the test process never holds an
executable open for writing. No retry and no longer deadline.

## Acceptance

- `cargo test -p connectors --test adversary_list_connectors` and `--test list_connectors` pass 200
  of 200 runs in a loop on Linux with every core busy, the count recorded as evidence.
- Neither file calls `fs::copy` or `fs::write` on a path it later executes.
