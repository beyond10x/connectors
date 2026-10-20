---
format: aep.planning-md/3
id: story:openrouter-inference-key-profile
kind: story
status: draft
title: OpenRouter's inference key is a second auth profile, and both OpenRouter instances migrate
relations:
- decomposes: epic:catalog-connectors-only-in-definitions
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: apps/connectors/tests/setup_migrate.rs
- confidence: inferred
  path: crates/connectors-catalog/src/definition.rs
- confidence: inferred
  path: crates/connectors-catalog/tests/fixtures/definitions/connectors/openrouter.yaml
revision: 3
---
## Outcome

OpenRouter's definition carries the inference key as a second auth profile with its own label, the
lowerer writes a per-profile label into the declaration, and both path-configured OpenRouter
instances (the management key and the inference key) migrate to the `openrouter` connector through
`setup migrate`.

## Why now

The operator decided "openrouter A" on 2026-10-20 (about 11:27), relayed to the connectors session by
the b10x-chef-orchestrator session: take option A whatever the probe test shows. The options put were:
A a second auth profile for the inference key in connectors-definitions' `openrouter.yaml` with a
per-profile label in the lowerer; B take `label` out of the configuration revision; C leave the
inference instance unmigrated and have the guide reconnect it.

The OpenRouter guide configures a second instance labelled "OpenRouter inference key"
(`docs/catalog-openrouter.md:203`); the definition carries only the management label, and `label` is
native configuration (`adapters/catalog/src/local.rs:137`). `setup migrate` keeps an entry whose
`configuration_revision` would change (`apps/connectors/src/local/migrate.rs:43-53,184`), so without
this the inference instance would stay pointing at a path story:catalog-provider-data-removed deletes
(inferred; a probe test is being run, see below).

## Acceptance

- A test in `apps/connectors/tests/setup_migrate.rs` migrates two path-configured OpenRouter entries,
  one labelled with the management label and one labelled "OpenRouter inference key", and both read
  back moved to `openrouter`, each with its own profile and label.
- connectors-definitions `connectors/openrouter.yaml` at the pinned tag declares both profiles, and the
  lowered declaration carries each profile's label.

## Scope

- **Files:** connectors-definitions `connectors/openrouter.yaml` — inferred (the other repository)
- **Files:** `crates/connectors-catalog/src/definition.rs` (per-profile label in the lowered declaration) — inferred
- **Files:** `apps/connectors/tests/setup_migrate.rs` — cited (the decision names the migration test)
- **Files:** `crates/connectors-catalog/tests/fixtures/definitions/connectors/openrouter.yaml` — inferred, while the fixture copy exists
- **Confidence:** medium; the definition format's profile shape is not read here

## Ordering

story:instances-resolve-through-locked-index depends on this story: its OpenRouter migration
acceptance needs both profiles.
