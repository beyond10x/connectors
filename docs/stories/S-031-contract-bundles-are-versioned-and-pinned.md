---
id: S-031
title: "Connector contract bundles are versioned, signed, and pinned"
pillar: Platform
status: blocked
priority:
design: docs/design/02-architecture.md
epic: contract-release
areas: [catalog, protocol, ci, docs]
note: "blocked on b10x/architecture RFC 0005; consumers never depend on an unversioned sibling checkout"
---

# Connector contract bundles are versioned, signed, and pinned

## Goal

Publish schemas and conformance vectors as reproducible owner-issued releases so substrate, agent,
cloud, Flux, and autodev can pin a contract without copying from `main` or a sibling checkout.

## Acceptance

- [ ] Architecture RFC 0005 is accepted and connectors owns explicit catalog and platform bundles.
- [ ] Each manifest records protocol version, source commit, generator version, hashes, and signing
      identity; consumers pin version and digest.
- [ ] Request, response, event, and channel-frame unknown-field/evolution rules are distinct and
      conformance-tested.
- [ ] A clean-room consumer passes the same vectors without repository source access.
- [ ] Release CI uses immutable action pins and emits signed evidence.

## Progress

- (blocked on architecture RFC 0005)
