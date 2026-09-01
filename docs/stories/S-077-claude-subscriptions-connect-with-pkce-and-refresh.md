---
id: S-077
title: "Claude subscriptions connect with PKCE and refresh"
pillar: Platform
status: done
priority:
epic: subscription-custody
areas: [subscription-custody, server, connectors-client, connectors-runtime]
note: "Connectors owns the public-client PKCE verifier, provider exchange, refresh-capable custody, and refresh-on-redemption; Identity remains provider-agnostic."
---

# Claude subscriptions connect with PKCE and refresh

## Goal

Let an authenticated person connect their Claude subscription through the provider's public-client
PKCE flow instead of obtaining and pasting a durable credential themselves, while keeping every
provider token in Connector-owned custody.

## Acceptance

- [x] A bounded, expiring, single-use start operation creates server-held PKCE state and returns
      only an authorization URL plus an opaque flow id.
- [x] Completion is bound to the verified tenant and subject, consumes the pending flow before the
      provider exchange, and stores neither code nor verifier after the attempt.
- [x] The provider exchange uses the exact public-client authorization-code JSON contract and
      validates bounded access, refresh, expiry, and inference-scope fields before custody.
- [x] Custody stores a versioned refresh-capable record, returns presence only, and exports only a
      current access credential through an exact attempt lease redemption.
- [x] Redemption refreshes under serialization before expiry, handles refresh-token rotation, and
      replaces custody without exposing either token in diagnostics or API responses.
- [x] Hosted routes and the official Rust client expose start and complete with non-cacheable
      responses, bounded bodies, exact Identity scope checks, and credential-safe types.
- [x] The embedded OpenAPI artifact and tests cover start, completion, replay refusal, provider
      response validation, refresh, and lease redemption.

## Boundary decision

Identity does not know Claude, Agent Platform, provider endpoints, or either service's scope
vocabulary. Provider acquisition and token lifecycle are Connector responsibilities; Devcenter is
only the authenticated BFF that invokes them.

## Progress

- 2026-09-01 — filed from the first deployed Devcenter journey after the operator clarified that
  `Connect Claude` means provider OAuth2 PKCE, not a manually pasted setup credential.
- 2026-09-01 — implemented in `subscription-custody`, hosted server, `connectors-client`, runtime,
  and embedded OpenAPI. Unit and HTTP tests cover one-use completion, refresh-token rotation,
  current-token lease redemption, route authorization, and route/schema consistency.
