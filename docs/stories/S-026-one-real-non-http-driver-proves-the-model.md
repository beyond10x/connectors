---
id: S-026
title: "One real non-HTTP driver proves the five-axis model"
pillar: Platform
status: in-progress
priority:
design: docs/design/03-beyond-http.md
epic: beyond-http
areas: [catalog, domain, service, server]
note: "Asterisk sip.dial is source-grounded and development-proven; S-032 retains the stable-support matrix"
---

# One real non-HTTP driver proves the five-axis model

## Goal

Prove the abstraction against one real built-in protocol: SIP through the bounded `sip_v1` driver,
without a generic framing language or external runtime artifact. S-032 owns the concrete slice;
this story retains the abstraction-level acceptance.

## Acceptance

- [ ] One SIP provider declares session establishment, closed call events, credentials,
      risk/effects, driver, placement-independent shape, and capability requirements as reviewed
      data.
- [ ] The built-in driver consumes the common zero-IO plan and shared egress/audit composition.
- [ ] Authentication failure, protocol refusal, reconnect, event provenance, bounded buffering,
      call cancellation, and unsupported capability cases have fixtures.
- [x] No caller chooses an executable, arbitrary protocol string, credential destination, or
      placement.
- [x] The proof records which abstraction pressure is real before any second driver is planned.

## Progress

- SIP is selected by architecture ADR 0024. Asterisk now contributes the one source-grounded
  `sip-dial`/`sip_v1` member, and the exact sipx driver has completed both loopback SIP/RTVBP
  composition and an operator-authorized dev-cluster SIP/RTP echo call. The operation accepts only
  a Connection-owned alias; route, protocol, placement, credentials, and socket apertures remain
  deployment facts.
- The proof exposed the remaining pressure explicitly: the pinned sipx media runtime may transmit
  to a symmetric-RTP peer before the outer adapter can inspect it, and both the complete lifecycle
  matrix and shared production audit/serving path remain open. S-032 therefore remains the stable
  support gate rather than treating the successful development call as completion.
