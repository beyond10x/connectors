---
id: S-031
title: "Connector contract bundles are versioned, signed, and pinned"
pillar: Platform
status: backlog
priority:
design: docs/design/02-architecture.md
epic: contract-release
areas: [catalog, protocol, ci, docs]
note: "architecture closed by ADR 0019; bundle/release implementation remains"
---

# Connector contract bundles are versioned, signed, and pinned

## Goal

Publish schemas and conformance vectors as reproducible owner-issued releases so substrate, agent,
cloud, Flux, and autodev can pin a contract without copying from `main` or a sibling checkout.

## Acceptance

- [x] Architecture RFC 0005 is accepted by ADR 0019 and connectors owns explicit catalog and platform bundles.
- [ ] Each manifest records protocol version, source commit, generator version, hashes, and signing
      identity; consumers pin version and digest.
- [ ] Request, response, event, and channel-frame unknown-field/evolution rules are distinct and
      conformance-tested.
- [ ] A clean-room consumer passes the same vectors without repository source access.
- [ ] Release CI uses immutable action pins and emits signed evidence.

## Progress

- Architecture is accepted: deterministic private OCI bundles, digest pins, dedicated release
  signing, origin manifests, clean-room vectors, and current/previous-major overlap are fixed.
- Remaining work is bundle generation, signing infrastructure, and consumer conformance in CI.
