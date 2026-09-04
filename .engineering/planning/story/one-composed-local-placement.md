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
revision: 1
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
