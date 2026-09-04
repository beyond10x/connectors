---
format: aep.planning-md/1
id: story:production-credential-custody-closes
kind: story
status: draft
title: Production credential custody closes with owner evidence
refs:
- provider: legacy
  reference: S-034
relations:
- derived_from: epic:credential-production
revision: 1
---
## Acceptance

Verbatim from `docs/stories/S-034-production-credential-custody-closes.md:21`. **read**

- [ ] Personal release custody uses an accepted OS-keychain set and explicitly refuses headless
      operation when none is available; no file or environment fallback is shipped.
- [ ] External-provider binding fixes session lifetime, user presence, revocation, and unavailable
      behavior without a generic value-reading Connector operation.
- [ ] The management contract implements value-free acquire/status/rotate/revoke/delete, atomic
      credential-generation and Connection metadata changes, and no read/export/list-value method.
- [ ] Target-sealed satellite completion fixes the envelope algorithm, authenticated target-key
      binding, one-use replay/expiry behavior, partition handling, and interrupted-commit recovery.
- [ ] Production managed storage proves tenant isolation, KMS/seal custody, HA, backup/restore,
      root-token elimination, break-glass recovery, rotation/revocation SLOs, and incident response.
- [ ] End-to-end sentinels prove credential and derived forms absent from responses, logs, audit,
      argv, environment, relational state, manifests, crashes, backups, and central relay plaintext.
- [ ] Every maintained repository runs the checksum-pinned full-history secret scan; any historical
      baseline is exact-fingerprint-only, provenance-reviewed, and paired with rotation evidence
      whenever material may have been live.
- [ ] Owner-signed bundles, a clean-room consumer, and cross-foundation Identity/Cloud/Substrate
      conformance satisfy ADR 0019 before a stable or production claim.

## Context

Complete the Connector-owned portion of architecture's phase-8 production credential and secret
closure without turning a development FileStore, one-node Vault, or design fixture into release
evidence.

Source frontmatter: pillar Platform · areas [connector-secrets, service, protocol, ci, docs] · design `docs/design/07-credential-custody-topologies.md`. **read**

Source `note:` field, quoted: “ADR 0032 accepts only the central managed-store development subset; every production exit remains gated”

## Status

`backlog` in the source. Quoted from `docs/stories/S-034-production-credential-custody-closes.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-034-production-credential-custody-closes.md`, which is not deleted and now names this artifact.

- First written 2026-08-15 · last touched 2026-08-15 · 1 revision(s)
- Legacy id `S-034`, recorded as the reference `legacy:S-034`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
