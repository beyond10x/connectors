---
id: S-061
title: "A logged-in person needs no configuration"
pillar: Platform
status: done
design: ../design/15-a-zero-configuration-endpoint-plane.md
epic: endpoint-plane
areas: [config, integrations]
---

# A logged-in person needs no configuration

## Goal

Today a person needs curl and token plumbing to use the hosted plane. Give the local Connectors
CLI a zero-configuration hosted mode: `connectors login <connectors-base>` reads that deployment's
public bootstrap document, drives the neutral Identity loopback flow, and places the opaque login
session in the OS keyring. Subsequent hosted CLI requests and the stdio MCP bridge exchange that
session for short-lived, exact-scope tokens transparently. Identity remains relying-party neutral:
Connectors publishes the Identity origin it trusts, never the other way around.

## Acceptance

- On a machine with no prior state: `connectors login <connectors-base>` completes the browser
  flow, and a following catalog/search/invoke command against the selected hosted deployment
  succeeds with no endpoint or token flags; tokens refresh across the
  300-second expiry without re-login, proven by an integration test with a fake identity.
- The thin-CLI architecture fence stays green — the behaviour lands behind connectors-client
  per its prescription.

## Progress

- 2026-08-24 — filed from design 15 (Timo: "users, when logged in with Google SSO via us,
  don't have to configure any of these things — they can just use it").
- 2026-09-02 — implemented the Connectors-owned discovery document, browser Authorization Code +
  S256 PKCE loopback, OS-keyring session custody, non-secret XDG selection state, minimal-scope
  token caching and refresh, one authenticated retry, and automatic hosted selection for
  Operation, Connection and Event commands. A fake Identity/Connectors integration proves login,
  separate exact-scope tokens, cache reuse and refresh without re-login.

## Superseded by

`story:a-logged-in-person-needs-no-configuration` in the AEP planning store, at
`.engineering/planning/story/a-logged-in-person-needs-no-configuration.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
