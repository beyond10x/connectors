---
id: S-027
title: "Direct-byte session establishment is operation-scoped"
pillar: Platform
status: backlog
priority:
design: docs/design/03-beyond-http.md
epic: beyond-http
areas: [domain, protocol, service, server]
note: "architecture closed by ADR 0016; concrete journey and conformance remain"
---

# Direct-byte session establishment is operation-scoped

## Goal

Govern one concrete terminal, tunnel, or media establishment while the client and serving endpoint
exchange continuous bytes directly under a short-lived bounded authority.

## Acceptance

- [x] Architecture RFC 0002 is accepted by ADR 0016 with issuer/verifier, audience/resource/principal binding,
      TTL, replay, reconnect, revocation, proof-of-possession, TLS, and NAT semantics.
- [ ] Connectors audits grant admission and authority issuance without receiving continuous bytes.
- [ ] The serving endpoint independently enforces local ownership, limits, and capability facts.
- [ ] Authorities are absent from logs, ordinary events, and client-visible long-lived state.
- [ ] Wrong audience/resource/principal, expiry, replay, and revoked deployment all fail fixtures.

## Progress

- Architecture is accepted: governed connectors issuer, local substrate issuer, proof-bound
  60-second one-shot authority, fresh reconnect grant, explicit lease, TLS, and `unserved` when no
  independent route exists.
- Remaining work is a concrete terminal/tunnel/media implementation and shared negative vectors.
