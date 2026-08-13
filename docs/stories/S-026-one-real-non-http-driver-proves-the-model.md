---
id: S-026
title: "One real non-HTTP driver proves the five-axis model"
pillar: Platform
status: backlog
priority:
design: docs/design/03-beyond-http.md
epic: beyond-http
areas: [catalog, domain, service, server]
note: "ADR 0010 delivery item 4; AMI is the leading candidate only when concrete demand and destination policy exist"
---

# One real non-HTTP driver proves the five-axis model

## Goal

Prove the abstraction against one real built-in protocol, preferably Asterisk AMI when demand is
concrete, without a generic framing language or external runtime artifact.

## Acceptance

- [ ] One provider declares unary actions, closed events, credentials, risk/effects, driver, shape,
      and capability requirements as reviewed data.
- [ ] The built-in driver consumes the common zero-IO plan and shared egress/audit composition.
- [ ] Authentication failure, protocol refusal, reconnect, event provenance, bounded buffering, and
      unsupported capability cases have fixtures.
- [ ] No caller chooses an executable, arbitrary protocol string, credential destination, or
      placement.
- [ ] The proof records which abstraction pressure is real before any second driver is planned.

## Progress

- (not started)
