---
id: S-027
title: "Direct-byte session establishment is operation-scoped"
pillar: Platform
status: blocked
priority:
design: docs/design/03-beyond-http.md
epic: beyond-http
areas: [domain, protocol, service, server]
note: "blocked on b10x/architecture RFC 0002; continuous bytes never enter ordinary invoke"
---

# Direct-byte session establishment is operation-scoped

## Goal

Govern one concrete terminal, tunnel, or media establishment while the client and serving endpoint
exchange continuous bytes directly under a short-lived bounded authority.

## Acceptance

- [ ] Architecture RFC 0002 is accepted with issuer/verifier, audience/resource/principal binding,
      TTL, replay, reconnect, revocation, proof-of-possession, TLS, and NAT semantics.
- [ ] Connectors audits grant admission and authority issuance without receiving continuous bytes.
- [ ] The serving endpoint independently enforces local ownership, limits, and capability facts.
- [ ] Authorities are absent from logs, ordinary events, and client-visible long-lived state.
- [ ] Wrong audience/resource/principal, expiry, replay, and revoked deployment all fail fixtures.

## Progress

- (blocked on architecture RFC 0002)
