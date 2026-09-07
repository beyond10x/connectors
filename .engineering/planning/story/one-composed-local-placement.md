---
format: aep.planning-md/1
id: story:one-composed-local-placement
kind: story
status: draft
title: One composed local placement, called by both the CLI and Zwirn
refs:
- provider: legacy
  reference: S-042
relations:
- derived_from: epic:local-product
scope:
- confidence: inferred
  path: crates/connectors-cli/src/lib.rs
- confidence: cited
  path: crates/connectors-runtime/src/composition.rs
- confidence: inferred
  path: crates/connectors-runtime/src/lib.rs
- confidence: cited
  path: crates/connectors-runtime/src/one_shot.rs
- confidence: cited
  path: crates/connectors-runtime/tests/one_shot_runtime.rs
- confidence: cited
  path: docs/design/12-one-owner-for-every-outside-connection.md
revision: 4
---
## Acceptance

Verbatim from `docs/stories/S-042-one-composed-local-placement.md:61`. **read**

- [ ] `compose` and `bind` are separate, and `connectors serve` is the only caller of `bind`.
- [ ] A one-shot CLI command answers with no daemon running.
- [ ] Zwirn's local placement is a call to `compose`, not a spawned process plus reimplemented flows.
- [ ] `attach_managed_slack`, `attach_managed_kubernetes` and `submit_connect_credential` are deleted
      from `products/zwirn/crates/agent-app/src/connectors.rs`.
- [ ] A declared instance still materialises at open without a human typing a token, so a restart
      costs no hands. This property came from the concurrent session's Slack work and must survive.
- [ ] An ADR records the build-dependency edge.

## Context

One function that composes a local Connector placement from configuration, called by everything that
needs one — a one-shot CLI command, `connectors serve`, and Zwirn's local placement — so there is
exactly one composition path and exactly one owner of auth, secrets, token state and dispatch.

Timo, 2026-08-20: *"since connectors-cli AND zwirn both need a 'local' variant of those seams, we
should put the main composed thing in a shared place inside connectors. Then cli just calls the
constructor with config, and zwirn does the same thing."*

Source frontmatter: pillar Platform · areas [runtime, cli, zwirn] · priority 3 · design `../design/12-one-owner-for-every-outside-connection.md`. **read**

Source `note:` field, quoted: “depends on S-041; splits compose from bind so the CLI one-shot, `connectors serve` and Zwirn's local placement share one entry point”

## Status

`backlog` in the source. Quoted from `docs/stories/S-042-one-composed-local-placement.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-042-one-composed-local-placement.md`, which is not deleted and now names this artifact.

- First written 2026-08-20 · last touched 2026-08-20 · 1 revision(s)
- Legacy id `S-042`, recorded as the reference `legacy:S-042`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill

## Scope

Derived 2026-09-07 by `story-scoper` through read-only inspection — cited.

- **Primary surface:** `crates/connectors-runtime/src/composition.rs:186` — cited; `PersonalComposition` already retains registry/state ownership, and `PersonalRuntime::compose` exists at line 249 but is crate-private.
- **Public API:** `crates/connectors-runtime/src/lib.rs:13` — inferred; expose an embedding-safe composition interface through existing runtime exports.
- **Existing consumer:** `crates/connectors-runtime/src/one_shot.rs:54` — cited; daemon-free operations already call the shared composition function.
- **Runtime verification:** `crates/connectors-runtime/tests/one_shot_runtime.rs:56` — cited; preserve ownership-before-side-effects and bounded execution.
- **CLI:** `crates/connectors-cli/src/lib.rs:1070` — inferred; adapt serve/one-shot call sites if the public composition/binding API changes.
- **Documents:** `docs/design/12-one-owner-for-every-outside-connection.md:85` — cited; records the shared composition API and consumer dependency.
- **Cross-repository:** Zwirn embedding migration and build-dependency ADR — inferred; current authoritative paths were not established in this checkout.
- **Symbols:** `PersonalRuntime::compose`, `PersonalComposition`, `PersonalCredentialStores`, `LocalStateOwnership`, `BackendRegistry` — cited.
- **Confidence:** medium — inferred; local ownership is explicit, while consumer migration and ADR acceptance remain unverified.
- **Would collide with:** personal runtime composition/public exports, one-shot lifetime/state ownership, CLI runtime dispatch, and Zwirn embedding changes — inferred.
