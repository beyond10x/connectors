---
format: aep.planning-md/1
id: story:connect-session-oauth-custody-in-personal-posture
kind: story
status: proposed
title: Decide the connect-session ↔ OAuth-callback custody chain in personal posture
tags:
- ready
- wave-cli
refs:
- provider: legacy
  reference: S-013
relations:
- derived_from: epic:carried-constraints
scope:
- confidence: cited
  path: crates/catalog-build/src/document.rs
- confidence: cited
  path: crates/catalog/src/lib.rs
- confidence: cited
  path: crates/catalog/src/table.rs
- confidence: inferred
  path: crates/catalog/tests/main/consumer_api.rs
- confidence: inferred
  path: crates/connector-spec/schema/provider-toml.schema.json
- confidence: cited
  path: crates/connector-spec/src/auth.rs
- confidence: inferred
  path: crates/connector-spec/src/provider/auth_validation.rs
- confidence: inferred
  path: crates/connector-spec/tests/main/oauth2_acquisition.rs
- confidence: cited
  path: docs/design/07-credential-custody-topologies.md
- confidence: inferred
  path: docs/design/21-personal-oauth-callback-custody.md
- confidence: inferred
  path: providers/gitlab.toml
- confidence: inferred
  path: providers/jira.toml
- confidence: inferred
  path: providers/slack.toml
revision: 33
---
## Acceptance

Verbatim from `docs/stories/S-013-connect-session-oauth-custody-in-personal-posture.md:22`. **read**

- [ ] A numbered design document (`docs/design/NN-*.md`, per AGENTS.md's series rule) names the
      custody chain for each flow it admits: **loopback redirect** (`http://127.0.0.1:<port>/…`
      registered as the deployment's redirect URI) and **device-code-style** flows, with the rule for
      choosing between them. It states, measured against real vendor registration rules rather than
      assumed, which providers refuse loopback, which refuse plain `http` on loopback, and which
      offer a device flow — naming the vendors checked and the date.
- [ ] The choice is **declared in the catalog** (the redirect-URI shapes and grant flows a provider
      admits) rather than decided at runtime by attempting one and falling back. A fallback would let
      the same connection succeed by different means on two machines, and the pair would be
      unreviewable.
- [ ] The connect-session invariants hold unchanged in personal posture: single-purpose, short-lived,
      never returns credential material to its creator, terminal event names the connection id and
      nothing else. The loopback listener is bound to **one** session, admits exactly one callback,
      binds the port for no longer than the session lives, validates `state`, PKCE, exact Host and
      expected Origin where present, and refuses CSRF, DNS-rebinding, state, or PKCE mismatch by
      name.
- [ ] The headless case is covered explicitly: an agent obtains the URL and a human completes it
      ([S-014](../../../docs/stories/S-014-auth-as-tool-result.md)'s consumer), **including** the case where the human's
      browser is not on the machine running the listener. That case is either supported by a named
      mechanism (device code, or a paste-back of the authorization response) or refused with a message
      that says what to do instead — never a silent hang until the session expires.
- [ ] BYO registration stays deployment configuration: no `client_id` and no secret appears in the
      catalog, the redirect URI a user must register is derivable from their configuration alone, and
      `connectors doctor` can print it.
- [ ] Whatever is decided is portable: a connection authorized in personal posture and one authorized
      in org posture are the same connection shape, because the catalog and the auth templates are
      the same text everywhere (vision, three postures).

## Context

Answer the domain model's open question 3 — how a connect session takes custody of an OAuth callback
on a machine with no public callback origin — and make the answer **declared per provider** rather
than discovered at runtime, so a personal-posture user can connect a real vendor through the same
connect-session contract every posture uses.

Source frontmatter: pillar Platform · areas [domain, service, catalog]. **read**

Source `note:` field, quoted: “domain model open question 3: personal posture has no public callback origin, so the custody chain is loopback redirect vs device-code-style flows — per provider, and per what each vendor's registration rules actually admit. A decision story: the deliverable is a numbered design document plus the schema consequence”

## Status

`backlog` in the source. Quoted from `docs/stories/S-013-connect-session-oauth-custody-in-personal-posture.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-013-connect-session-oauth-custody-in-personal-posture.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-13 · 2 revision(s)
- Legacy id `S-013`, recorded as the reference `legacy:S-013`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill

## Readiness

Selected for implementation on 2026-09-06 at the operator's request. wave-cli: 3 of 10. The ready tag records selection; proposed is the pre-implementation lifecycle state, and existing active work stays active. Existing dependencies and implementation evidence requirements still apply.

## Scope

Derived 2026-09-06 by aep-drive story-scoper; coordinator records the returned surfaces.

- `docs/design/21-personal-oauth-callback-custody.md` — inferred.
- `crates/connector-spec/src/auth.rs` — cited.
- `crates/connector-spec/src/provider/auth_validation.rs` — inferred.
- `crates/connector-spec/schema/provider-toml.schema.json` — inferred.
- `crates/connector-spec/tests/main/oauth2_acquisition.rs` — inferred.
- `crates/catalog/src/lib.rs` — cited.
- `crates/catalog/src/table.rs` — cited.
- `crates/catalog-build/src/document.rs` — cited.
- `crates/catalog/tests/main/consumer_api.rs` — inferred.
- `docs/design/07-credential-custody-topologies.md` — cited.
- `providers/gitlab.toml` — inferred.
- `providers/slack.toml` — inferred.
- `providers/jira.toml` — inferred.

Medium confidence. Existing PendingStates/PKCE/custody are reusable. Current raw-credential loopback is not an OAuth callback. A decision and schema alone cannot satisfy runtime acceptance; confirm provider callback facts from official sources, settle callback/device transport and type any new values before dispatch. Reserve design 21 for this decision.

Would collide with any unit editing these files; directory entries require an additional containment review because AEP compares scope strings exactly.

## Execution queue

CLI execution queue 2026-09-06: 9 of 10. The urgent Slack delivery follow-up leads the queue. Original wave-cli readiness ordering remains historical context. Dependencies and measured scope govern dispatch order; priority is not a claim that prerequisites have landed.
