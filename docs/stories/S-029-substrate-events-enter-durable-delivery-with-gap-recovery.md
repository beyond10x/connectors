---
id: S-029
title: "Substrate events enter durable delivery with gap recovery"
pillar: Platform
status: blocked
priority:
design: docs/design/02-architecture.md
epic: substrate-integration
areas: [domain, protocol, service, server]
note: "blocked on b10x/architecture RFC 0003 and a released substrate event/ledger bundle"
---

# Substrate events enter durable delivery with gap recovery

## Goal

Own the bridge from a bounded substrate cursor to connectors' durable delivery and replay contract
without pretending those guarantees are identical.

## Acceptance

- [ ] One supervised Channel per substrate Connection persists a deployment/generation-scoped
      high-water mark and deduplicates on deployment + generation + source sequence.
- [ ] Native resource, transition, operation, actor, time, and source sequence survive normalization;
      connectors adds but never rewrites platform attribution.
- [ ] A cursor outside retention produces a typed gap, ledger/resource reconciliation, and an
      explicit reconciliation boundary before normal delivery resumes.
- [ ] Duplicate, reconnect, retention-gap, state-loss generation, and replay fixtures are shared
      with the released substrate contract.
- [ ] Substrate remains independent of the connectors runtime.

## Progress

- (blocked on architecture RFC 0003 and substrate contract bundle)
