---
format: aep.planning-md/3
id: story:setup-init-writes-current-format
kind: story
status: draft
title: connectors setup init writes connectors-local/5
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Context

`Config::initialize` (`crates/connectors-host/src/local/config.rs:1505`) writes a new
configuration with `format = "connectors-local/2"` and an empty `[adapters]` table. The wave of
epic:connector-vocabulary made `connectors-local/5` with `[connectors.<c>.instances.<i>]` the
documented form, so every guide now tells a new user to replace the `format` line before adding
an instance (`apps/connectors/tests/setup_init_format_adversary.rs` pins that step). Found by the
README and site unit on 2026-10-16.

## Change

- `connectors setup init` writes `format = "connectors-local/5"` with no `[adapters]` table.
- The guides that tell the user to replace the `format` line drop that step;
  `setup_init_format_adversary.rs` follows.
- Tests that build a configuration through `Config::initialize` and then add `[adapters.*]`
  entries keep working: they set the format they need, as `owner.rs` tests already do.

## Scope

- `crates/connectors-host/src/local/config.rs`
- `apps/connectors/tests/setup_init_format_adversary.rs`
- `docs/local-*.md`, `docs/catalog-*.md` and `website/docs/getting-started.md`, where they name the format step

## Acceptance

After `connectors setup init` on an empty directory, `connectors setup check` passes, and adding
one `[connectors.gitlab.instances.work]` table with no other edit gives a configuration
`connectors instances list` shows as `gitlab.work`.
