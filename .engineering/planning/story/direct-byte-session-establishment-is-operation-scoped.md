---
format: aep.planning-md/1
id: story:direct-byte-session-establishment-is-operation-scoped
kind: story
status: active
title: Direct-byte session establishment is operation-scoped
refs:
- provider: legacy
  reference: S-027
relations:
- derived_from: epic:beyond-http
scope:
- confidence: cited
  path: crates/domain
- confidence: cited
  path: crates/protocol
- confidence: cited
  path: crates/server
- confidence: cited
  path: crates/service
revision: 4
---
## Acceptance

Verbatim from `docs/stories/S-027-direct-byte-session-establishment-is-operation-scoped.md:20`. **read**

- [x] Architecture RFC 0002 is accepted by ADR 0016 with issuer/verifier, audience/resource/principal binding,
      TTL, replay, reconnect, revocation, proof-of-possession, TLS, and NAT semantics.
- [ ] Connectors audits grant admission and authority issuance without receiving continuous bytes.
- [ ] The serving endpoint independently enforces local ownership, limits, and capability facts.
- [ ] A private satellite initiates RTVBP outward; an unavailable direct route is `unserved`, and
      the federation control path cannot relay bytes.
- [ ] One call remains on one endpoint generation until hangup or bounded drain; it is not migrated
      mid-dialog.
- [ ] Authorities are absent from logs, ordinary events, and client-visible long-lived state.
- [ ] Wrong audience/resource/principal, expiry, replay, and revoked deployment all fail fixtures.

## Context

Govern the SIP-to-RTVBP media establishment while the selected voice and application endpoints
exchange continuous bytes directly under a short-lived bounded authority.

Source frontmatter: pillar Platform · areas [domain, protocol, service, server] · design `docs/design/03-beyond-http.md`. **read**

Source `note:` field, quoted: “ADR 0016 fixes authority; ADR 0024 selects the SIP-to-RTVBP voice journey”

## Status

`in-progress` in the source. Quoted from `docs/stories/S-027-direct-byte-session-establishment-is-operation-scoped.md:5`: `status: in-progress`. **read**

## Provenance

Migrated from `docs/stories/S-027-direct-byte-session-establishment-is-operation-scoped.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-15 · 5 revision(s)
- Legacy id `S-027`, recorded as the reference `legacy:S-027`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
