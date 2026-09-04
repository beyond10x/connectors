---
format: aep.planning-md/1
id: story:claude-subscriptions-connect-with-pkce-and-refresh
kind: story
status: implemented
title: Claude subscriptions connect with PKCE and refresh
refs:
- provider: legacy
  reference: S-077
relations:
- derived_from: epic:subscription-custody
scope:
- confidence: cited
  path: crates/connectors-client
- confidence: cited
  path: crates/connectors-runtime
- confidence: cited
  path: crates/server
- confidence: cited
  path: crates/subscription-custody
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-077-claude-subscriptions-connect-with-pkce-and-refresh.md:20`. **read**

- [x] A bounded, expiring, single-use start operation creates server-held PKCE state and returns
      only an authorization URL plus an opaque flow id.
- [x] Completion is bound to the verified tenant and subject, consumes the pending flow before the
      provider exchange, verifies the state returned in the provider's `code#state` manual result,
      exchanges only the code component, and stores neither code nor verifier after the attempt.
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

## Context

Let an authenticated person connect their Claude subscription through the provider's public-client
PKCE flow instead of obtaining and pasting a durable credential themselves, while keeping every
provider token in Connector-owned custody.

Source frontmatter: pillar Platform · areas [subscription-custody, server, connectors-client, connectors-runtime]. **read**

Source `note:` field, quoted: “Connectors owns the public-client PKCE verifier, provider exchange, refresh-capable custody, and refresh-on-redemption; Identity remains provider-agnostic.”

## Status

`done` in the source. Quoted from `docs/stories/S-077-claude-subscriptions-connect-with-pkce-and-refresh.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-077-claude-subscriptions-connect-with-pkce-and-refresh.md`, which is not deleted and now names this artifact.

- First written 2026-09-01 · last touched 2026-09-01 · 2 revision(s)
- Legacy id `S-077`, recorded as the reference `legacy:S-077`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
