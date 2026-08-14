---
id: S-027
title: "Direct-byte session establishment is operation-scoped"
pillar: Platform
status: in-progress
priority:
design: docs/design/03-beyond-http.md
epic: beyond-http
areas: [domain, protocol, service, server]
note: "ADR 0016 fixes authority; ADR 0024 selects the SIP-to-RTVBP voice journey"
---

# Direct-byte session establishment is operation-scoped

## Goal

Govern the SIP-to-RTVBP media establishment while the selected voice and application endpoints
exchange continuous bytes directly under a short-lived bounded authority.

## Acceptance

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

## Progress

- Architecture is accepted: governed connectors issuer, local substrate issuer, proof-bound
  60-second one-shot authority, fresh reconnect grant, explicit lease, TLS, and `unserved` when no
  independent route exists.
- Architecture ADR 0024 selects the concrete neutral RTVBP media journey and extends verification
  to the selected voice/application endpoint without widening issuer or replay semantics.
- `server::authority` now issues an Ed25519 authority to the connecting voice endpoint, binds its
  ephemeral DPoP key and exact `GET wss://…` upgrade, and gives a distinct proof type only to the
  serving endpoint after atomic replay redemption. Audience, expiry, replay, revocation, exact URI,
  and debug-redaction fixtures pass.
- Satellite reachability, generation drain, durable-state inspection, and production serving
  enforcement remain open; the repository bundle is alpha and unsigned.
