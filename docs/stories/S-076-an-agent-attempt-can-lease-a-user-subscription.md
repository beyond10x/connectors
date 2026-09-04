---
id: S-076
title: "An agent attempt can lease a user's subscription"
pillar: Platform
status: done
priority:
design: ../design/17-attempt-bounded-subscription-credential-leases.md
epic: subscription-custody
areas: [subscription-custody, server, connectors-client, connectors-runtime, identity-http]
note: "A least-privilege Identity exchange creates an expiring finite-use capability for one exact Harness attempt; Connectors keeps durable custody and the provider value reaches only the bearer boundary."
---

# An agent attempt can lease a user's subscription

## Goal

Let a user-bound agent attempt run through the user's Claude Code subscription without copying the
durable credential into Agent Platform, Devcenter, configuration, logs, or an agent record.

## Acceptance

- [x] Connector-owned custody stores one credential per verified tenant/subject under the permanent
      Claude Code authority and exposes presence, replacement and idempotent disconnect.
- [x] Lease creation requires `connectors.credentials.lease` and binds a cryptorandom capability to
      an exact attempt id, at most one hour, 1–1,024 uses and an in-process 10,000-lease cap.
- [x] Redemption accepts the lease bearer rather than Identity, compares it in constant time, and
      refuses wrong attempt, expiry, exhaustion, unknown id and wrong bearer without distinction.
- [x] Restart, disconnect and credential replacement revoke live leases. Replacement and redemption
      serialize, so an old capability cannot reach the new credential generation.
- [x] Hosted routes are opt-in, require Vault custody, bound every body, and return `no-store` plus
      `no-cache` on successful confidential responses.
- [x] The official client keeps lease and credential values private, redacts `Debug`, zeroizes on
      drop, enforces HTTPS, and refuses a cacheable confidential response.
- [x] The embedded OpenAPI document covers all five operations and marks returned secrets as
      sensitive without misclassifying response-only fields as write-only.
- [x] Tests prove connect → lease → wrong-attempt refusal → exact redemption → spent refusal,
      rotation revocation, disconnect revocation, redaction, and cache-policy refusal.

## Progress

- 2026-09-01 — custody, attempt leases, hosted composition, official client, OpenAPI and refusal
  tests landed together for the 0.3.0 breaking release.

## Notes

- This resolves design 16's last open boundary. It does not make the custody-only provider
  callable: catalog discovery and invocation still see zero services and zero operations.
- Setup tokens do not refresh. A user replaces the connected value explicitly; every existing
  lease is revoked before the new generation can be redeemed. This describes the 0.3.0 seam;
  [S-077](S-077-claude-subscriptions-connect-with-pkce-and-refresh.md) adds refresh-capable OAuth
  custody without changing the attempt-lease contract.

## Superseded by

`story:an-agent-attempt-can-lease-a-user-subscription` in the AEP planning store, at
`.engineering/planning/story/an-agent-attempt-can-lease-a-user-subscription.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
