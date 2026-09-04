---
format: aep.planning-md/1
id: story:an-agent-attempt-can-lease-a-user-subscription
kind: story
status: implemented
title: An agent attempt can lease a user's subscription
refs:
- provider: legacy
  reference: S-076
relations:
- derived_from: epic:subscription-custody
scope:
- confidence: cited
  path: crates/connectors-client
- confidence: cited
  path: crates/connectors-runtime
- confidence: cited
  path: crates/identity-http
- confidence: cited
  path: crates/server
- confidence: cited
  path: crates/subscription-custody
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-076-an-agent-attempt-can-lease-a-user-subscription.md:20`. **read**

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

## Context

Let a user-bound agent attempt run through the user's Claude Code subscription without copying the
durable credential into Agent Platform, Devcenter, configuration, logs, or an agent record.

Source frontmatter: pillar Platform · areas [subscription-custody, server, connectors-client, connectors-runtime, identity-http] · design `../design/17-attempt-bounded-subscription-credential-leases.md`. **read**

Source `note:` field, quoted: “A least-privilege Identity exchange creates an expiring finite-use capability for one exact Harness attempt; Connectors keeps durable custody and the provider value reaches only the bearer boundary.”

## Status

`done` in the source. Quoted from `docs/stories/S-076-an-agent-attempt-can-lease-a-user-subscription.md:5`: `status: done`. **read**

This artifact reached `implemented` with `aep artifact move --evidence test_result=1`. The journal
records that move as resting on an **assertion**, not on a run this migration observed. The flag is
what the CLI provides for evidence that lives outside the store.

What was asserted, and where it came from:

- The source records `status: done` at the line quoted above. **read**
- `bash scripts/gate.sh` was green at commit `a48030b` on 2026-09-04 — exit 0, 136 `test result: ok`
  lines across 11 workspaces. **read**, from `~/.cache/connectors-gate/gate2.log`

No per-story run was attributed to this story. The gate is a repository-wide fact, and reading it as
proof of one story's acceptance would be an inference this record does not make.

## Provenance

Migrated from `docs/stories/S-076-an-agent-attempt-can-lease-a-user-subscription.md`, which is not deleted and now names this artifact.

- First written 2026-09-01 · last touched 2026-09-01 · 2 revision(s)
- Legacy id `S-076`, recorded as the reference `legacy:S-076`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
