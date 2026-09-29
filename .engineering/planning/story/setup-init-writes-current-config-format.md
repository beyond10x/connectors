---
format: aep.planning-md/3
id: story:setup-init-writes-current-config-format
kind: story
status: implemented
title: setup init writes the configuration format its adapter entries need
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: apps/connectors/tests/local_cli.rs
- confidence: cited
  path: apps/connectors/tests/owner_build_security.rs
- confidence: cited
  path: apps/connectors/tests/setup_init_format_adversary.rs
- confidence: cited
  path: contracts/cli/v1alpha1/private-mutations.md
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-host/src/local/approval_keys/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/config.rs
- confidence: cited
  path: crates/connectors-host/src/local/keyring/custody/tests.rs
- confidence: cited
  path: crates/connectors-host/tests/local_foundation.rs
- confidence: cited
  path: docs/local-kubernetes-cli.md
- confidence: cited
  path: docs/local-postgres-cli.md
- confidence: cited
  path: docs/local-runtime-foundation.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T15:46:16Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-09-29T15:46:16Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-09-29T20:39:42Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Observed

- Reproduced here on 0.15.1 (2026-09-29): `connectors --config <new> --state-dir <new> setup init`
  exits 0 and writes `format = "connectors-local/1"` (`crates/connectors-host/src/local/config.rs:270`).
- Reported by the knowledge-ingest consumer, same day, and with a 2026-09-25 build: after adding an
  adapter entry, `setup check` refuses the file as `invalid_configuration`; changing the format to
  `connectors-local/2` fixes it.
- Cause, read in code: `config.rs:224` requires `format == "connectors-local/2"` exactly when an entry
  carries `private_protocol`, and the refusal names neither the rule nor the field.

## Acceptance

- `setup init` writes `connectors-local/2`; a test runs `setup init`, adds a catalog adapter entry with
  `private_protocol`, and `setup check` succeeds.
- Existing `connectors-local/1` files without `private_protocol` keep loading unchanged.

## Decided for the wave (coordinator, 2026-09-29)

- The refusal that names the format and the entry's instance id is moved to
  story:configuration-refusal-names-the-entry: `Failure` carries no data (`local/mod.rs:26`) and the
  CLI error envelope admits no free text (`ess/domains/cli.yaml:267`), so it is a contract change of
  its own. Here the refusal stays `invalid_configuration`, unchanged.
- The tests that run `setup init` and then append an entry without `private_protocol`
  (`local_foundation.rs:39`, `local_cli.rs` ×5, `owner_build_security.rs:36`, `custody/tests.rs:414`)
  are updated to the `/2` form.
