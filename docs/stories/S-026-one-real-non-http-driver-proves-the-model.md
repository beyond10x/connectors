---
id: S-026
title: "One real non-HTTP driver proves the five-axis model"
pillar: Platform
status: backlog
priority:
design: docs/design/03-beyond-http.md
epic: beyond-http
areas: [catalog, domain, service, server]
note: "ADR 0024 selects SIP as the first proof; S-032 carries the implementation slice"
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
- [ ] No caller chooses an executable, arbitrary protocol string, credential destination, or
      placement.
- [ ] The proof records which abstraction pressure is real before any second driver is planned.

## Progress

- SIP is selected by architecture ADR 0024. Implementation now has S-024's platform-family
  foundation but remains blocked on its source fence and S-032's exact dependency/runtime gate.
