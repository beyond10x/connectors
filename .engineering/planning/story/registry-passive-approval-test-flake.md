---
format: aep.planning-md/3
id: story:registry-passive-approval-test-flake
kind: story
status: draft
title: Approval-target passivity test fails once in a full host run
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

`local::registry::tests::approval_target_is_passive_but_requires_exact_retained_admission`
passes in every full `cargo test -p connectors-host` run, not only alone.

## Today

It failed once in a full package run on 2026-10-14 at
`crates/connectors-host/src/local/registry/tests.rs:60` (`left: 3, right: 2`): SQLite's
`PRAGMA data_version` counted one more write than the test expects during a call that should be
passive. It passed 5 of 5 alone and on a full rerun of the same binary, and the change under test
did not touch `src/local`. `data_version` counts commits made through other connections to the same
database file, so a second connection writing the same store during the call would explain it;
that is a hypothesis, not observed.

## Acceptance

- The cause is observed (which connection wrote, from where), and the test or the code is fixed so
  that 100 of 100 full package runs pass.
