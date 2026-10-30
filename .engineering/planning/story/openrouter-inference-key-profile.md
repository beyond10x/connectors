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
- confidence: inferred
  path: apps/connectors/src/local/declarations.rs
- confidence: inferred
  path: apps/connectors/src/local/migrate.rs
- confidence: cited
  path: apps/connectors/tests/setup_migrate.rs
- confidence: inferred
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-host/src/local/config.rs
revision: 7
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

Derived 2026-10-29 by `story-scoper` on origin/main 65c928d72 (connectors) and 3f9dd19
(connectors-definitions). Every line is **cited** (read from the story or the tree) or **inferred**
(a reading that could be wrong).

- **Primary surface:** `apps/connectors` `setup migrate` (`apps/connectors/src/local/migrate.rs`) and
  its test — cited (Acceptance names `apps/connectors/tests/setup_migrate.rs`; `connector_of`,
  `migrate.rs:194-231`)
- **Files:** `apps/connectors/tests/setup_migrate.rs` — cited (Acceptance)
- **Files:** `apps/connectors/src/local/migrate.rs` — inferred: "each with its own profile" needs
  migrate to pick a declaration profile per entry, presumably by `auth.label`; today the migrated
  instance gets `profile: None` (`crates/connectors-host/src/local/config.rs:1043`)
- **Files:** `apps/connectors/src/local/declarations.rs` — inferred: OpenRouter is not among the 14
  compiled-in declarations (`declarations.rs:9-24`), so `connector_of` keeps both entries as
  `connector_undeclared` until a declaration is added there
- **Files (connectors-definitions):** `connectors/openrouter.yaml` — cited (Acceptance): a second
  profile, such as `openrouter.inference-key`, labelled "OpenRouter inference key", with
  `default: true` on `openrouter.api-key` (two profiles need exactly one default,
  `crates/connectors-catalog/src/definition.rs:1844-1856`; ids start with `openrouter.`, `:890`)
- **Files (connectors-definitions):** `CHANGELOG.md` — cited (connectors-definitions `AGENTS.md:69-80`)
- **Also likely:** `crates/connectors-host/src/local/config.rs` (`with_instances`, `:1017`) — inferred
- **Also likely:** `contracts/cli/v1alpha1/semantics.md` §3 — inferred, if migrate starts choosing a profile
- **Not touched:** `crates/connectors-catalog/src/definition.rs` — cited: the lowerer already emits one
  block per profile with its own `label` (`:1951`, `:2013`, loop at `:2196-2199`); zendesk's two
  profiles lower to two labels (`adapters/catalog/providers/zendesk/connector.json:14,25`)
- **Not touched:** `crates/connectors-catalog/tests/fixtures/definitions/connectors/openrouter.yaml` —
  cited: SM-D207 keeps definitions only in connectors-definitions, and
  `story:definitions-fixture-copy-removed` deletes the copy
- **Confidence:** medium — the definition and lowerer side is read; where the migration test gets an
  OpenRouter declaration, and how migrate picks a profile, are not decided by the story
- **Would collide with:** any unit touching `apps/connectors/src/local/{migrate,declarations}.rs` or
  `apps/connectors/tests/setup_migrate.rs` (the D218 units change `apps/connectors`); any unit editing
  connectors-definitions `connectors/openrouter.yaml` or its `CHANGELOG.md`
- **Not ready (2026-10-29):** the acceptance test needs an OpenRouter declaration, which
  `story:instances-resolve-through-locked-index` adds, and that story depends on this one; the
  "pinned tag" the acceptance names comes from `story:definitions-pin-covers-catalog`, which this story
  does not depend on

## Ordering

story:instances-resolve-through-locked-index depends on this story: its OpenRouter migration
acceptance needs both profiles.

## Probe result, 2026-10-20

A probe test settled the inferred risk above, and it was wrong: the inference-key instance migrates
without a second profile. `probe_openrouter_label_management_and_inference`, appended to
`apps/connectors/tests/setup_migrate.rs` in a throwaway worktree (archived, not merged), with an
OpenRouter `connector.json` lowered from the fixture definition (label "OpenRouter management key")
added to the compiled-in declarations. Run through `b10x-build-slot cargo test --locked -p connectors
--test setup_migrate probe_openrouter_label`: `test result: ok. 1 passed; 0 failed`, exit 0. Two
path-configured entries differing only in `auth.label` both read back
`disposition="migrated" reason=null differing_field=null connector="openrouter"`: `orm` (the
management label) and `ori` ("OpenRouter inference key"). Read from the code, as a hypothesis:
`migrate.rs` compares its nine FIELDS and the instance's alias, description, endpoint and profile,
never `auth.label`.

The operator's "openrouter A" stands as decided ("regardless of the test outcome"); the acceptance
test above still holds it. The b10x-chef-orchestrator session has told the operator the result; if
the operator drops A, this story is archived.
