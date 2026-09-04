---
id: S-029
title: "Substrate events enter durable delivery with gap recovery"
pillar: Platform
status: blocked
priority:
design: docs/design/02-architecture.md
epic: substrate-integration
areas: [domain, protocol, service, server]
note: "architecture closed by ADR 0017 and substrate phase 3 is green; implementation remains blocked on an owner-signed substrate bundle"
---

# Substrate events enter durable delivery with gap recovery

## Goal

Own the bridge from a bounded substrate cursor to connectors' durable delivery and replay contract
without pretending those guarantees are identical.

## Acceptance

- [ ] One supervised Channel per `(Connection, source_scope)` partition persists a
      source-scope/generation-scoped high-water mark and deduplicates on the exact native identity
      `(deployment, source_scope, generation, seq)`. `source_scope` is opaque and never derived from
      tenant data.
- [ ] The tuple's `deployment` comes only from the Connection's authenticated, out-of-band substrate
      peer binding. A frame/page/snapshot assertion that differs refuses before deduplication,
      delivery, or high-water mutation, degrades the Channel, and requires an authenticated
      operator rebind; payloads never select deployment and mismatch is not treated as a history
      gap.
- [ ] Native resource, transition, operation, actor, time, and source sequence survive normalization;
      connectors adds but never rewrites platform attribution.
- [ ] Initial durable bootstrap creates and completely consumes a stable snapshot before resuming
      from its opaque inclusive-barrier cursor. No-cursor pull remains diagnostic only.
- [ ] A cursor outside retention or from another source scope/generation produces one non-oracular
      typed gap, snapshot reconciliation, and an explicit reconciliation boundary before normal
      delivery resumes.
- [ ] Snapshot ingestion accepts empty complete current workspace and exec sets, distinguishes
      complete current state from bounded/truncated event provenance, and never projects operation
      ledger rows or deletion-tombstone tables.
- [ ] Duplicate, reconnect, retention-gap, state-loss generation, and replay fixtures are shared
      with the released substrate contract.
- [ ] Substrate remains independent of the connectors runtime.

## Progress

- Architecture is accepted: opaque source scope, generation/sequence, transactional connector
  high-water, snapshot-first bootstrap, explicit history-gap boundary, complete bounded current
  state, and honest bounded provenance are fixed.
- Substrate phase 3 now provides the independently reviewed source-scoped, snapshot-first 0.2
  development behavior and shared vector authority. No S-029 implementation exists yet: the
  required platform crates now exist, but stable adoption still requires an owner-signed Substrate
  bundle before the bridge is implemented. It is not blocked on an architectural owner or recovery
  decision.

## Superseded by

`story:substrate-events-enter-durable-delivery-with-gap-recovery` in the AEP planning store, at
`.engineering/planning/story/substrate-events-enter-durable-delivery-with-gap-recovery.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
