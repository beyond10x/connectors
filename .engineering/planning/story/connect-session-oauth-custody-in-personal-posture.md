---
format: aep.planning-md/1
id: story:connect-session-oauth-custody-in-personal-posture
kind: story
status: draft
title: Decide the connect-session ↔ OAuth-callback custody chain in personal posture
refs:
- provider: legacy
  reference: S-013
relations:
- derived_from: epic:carried-constraints
revision: 1
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
