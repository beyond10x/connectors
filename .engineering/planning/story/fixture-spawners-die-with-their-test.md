---
format: aep.planning-md/3
id: story:fixture-spawners-die-with-their-test
kind: story
status: draft
title: Keyring, custody and CLI-started owners die with the test that started them
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Finding

story:test-children-die-with-their-parent bound the host's printed provider and host test children
to their parent and tied the usual owner fixtures in apps/connectors/tests to their test. Still
unbound or untied (grepped by its implementor, not each read in full):

- owners the CLI starts by itself during apps/connectors tests (the CLI passes
  CONNECTORS_TEST_OWNER_PARENT only when its own environment has it)
- adapters/catalog/tests/local_runtime/owner_guard.rs
- crates/connectors-custody/src/secret_service/fixture.rs (dbus-daemon, gnome-keyring-daemon),
  crates/connectors-custody/src/mounted/fixture.rs, crates/connectors-custody/src/tests.rs:618
- spawners in crates/connectors-build (live_read_loop.rs, gate.rs, ignored.rs, tests),
  crates/connectors-catalog/tests/discovery.rs, crates/connectors-spec/src/toolchain.rs

## Outcome

A SIGKILLed test leaves none of these running: each spawner binds its child with
`parent_death::bind` (Linux) or ties an owner with CONNECTORS_TEST_OWNER_PARENT.

## Acceptance

- One kill-the-harness test per spawner family (owner via the CLI, keyring fixture, catalog
  owner_guard) finds nothing of it left within 2 s, red without the change.
