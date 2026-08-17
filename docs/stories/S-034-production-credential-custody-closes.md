---
id: S-034
title: "Production credential custody closes with owner evidence"
pillar: Platform
status: backlog
priority:
design: docs/design/07-credential-custody-topologies.md
epic: credential-production
areas: [connector-secrets, service, protocol, ci, docs]
note: "ADR 0032 accepts only the central managed-store development subset; every production exit remains gated"
---

# Production credential custody closes with owner evidence

## Goal

Complete the Connector-owned portion of architecture's phase-8 production credential and secret
closure without turning a development FileStore, one-node Vault, or design fixture into release
evidence.

## Acceptance

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

## Current bounded evidence

The central Vault KV v2 adapter, Kubernetes workload login, tenant-scoped layout, bounded sensitive
transport, and value-free Connection contract provide development evidence only. They close no
unchecked item above by themselves.

The current Connector history baseline is documented in
[Historical secret-scan baseline](../security/secret-scan-baseline.md). It contains no broad rule
or path exemption and is not production rotation evidence.
