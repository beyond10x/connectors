---
format: aep.planning-md/1
id: story:substrate-events-enter-durable-delivery-with-gap-recovery
kind: story
status: active
title: Substrate events enter durable delivery with gap recovery
refs:
- provider: legacy
  reference: S-029
relations:
- derived_from: epic:substrate-integration
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

Verbatim from `docs/stories/S-029-substrate-events-enter-durable-delivery-with-gap-recovery.md:20`. **read**

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

## Context

Own the bridge from a bounded substrate cursor to connectors' durable delivery and replay contract
without pretending those guarantees are identical.

Source frontmatter: pillar Platform · areas [domain, protocol, service, server] · design `docs/design/02-architecture.md`. **read**

Source `note:` field, quoted: “architecture closed by ADR 0017 and substrate phase 3 is green; implementation remains blocked on an owner-signed substrate bundle”

## Status

`blocked` in the source. Quoted from `docs/stories/S-029-substrate-events-enter-durable-delivery-with-gap-recovery.md:5`: `status: blocked`. **read**

## Provenance

Migrated from `docs/stories/S-029-substrate-events-enter-durable-delivery-with-gap-recovery.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-15 · 4 revision(s)
- Legacy id `S-029`, recorded as the reference `legacy:S-029`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
