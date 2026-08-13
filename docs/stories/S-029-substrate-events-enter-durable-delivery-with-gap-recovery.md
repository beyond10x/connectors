---
id: S-029
title: "Substrate events enter durable delivery with gap recovery"
pillar: Platform
status: blocked
priority:
design: docs/design/02-architecture.md
epic: substrate-integration
areas: [domain, protocol, service, server]
note: "architecture closed by ADR 0017; blocked only on released substrate event/reconciliation bundle"
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

- Architecture is accepted: persisted generation/sequence, transactional connector high-water,
  explicit history-gap boundary, and barriered resource/operation reconciliation are fixed.
- Blocked only on the released substrate phase-3 event/reconciliation bundle and implementation
  fixtures, not on an architectural owner or recovery decision.
