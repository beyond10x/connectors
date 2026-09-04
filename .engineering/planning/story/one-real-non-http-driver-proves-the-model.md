---
format: aep.planning-md/1
id: story:one-real-non-http-driver-proves-the-model
kind: story
status: active
title: One real non-HTTP driver proves the five-axis model
refs:
- provider: legacy
  reference: S-026
relations:
- derived_from: epic:beyond-http
scope:
- confidence: cited
  path: crates/catalog
- confidence: cited
  path: crates/domain
- confidence: cited
  path: crates/server
- confidence: cited
  path: crates/service
revision: 4
---
## Acceptance

Verbatim from `docs/stories/S-026-one-real-non-http-driver-proves-the-model.md:21`. **read**

- [ ] One SIP provider declares session establishment, closed call events, credentials,
      risk/effects, driver, placement-independent shape, and capability requirements as reviewed
      data.
- [ ] The built-in driver consumes the common zero-IO plan and shared egress/audit composition.
- [ ] Authentication failure, protocol refusal, reconnect, event provenance, bounded buffering,
      call cancellation, and unsupported capability cases have fixtures.
- [x] No caller chooses an executable, arbitrary protocol string, credential destination, or
      placement.
- [x] The proof records which abstraction pressure is real before any second driver is planned.

## Context

Prove the abstraction against one real built-in protocol: SIP through the bounded `sip_v1` driver,
without a generic framing language or external runtime artifact. S-032 owns the concrete slice;
this story retains the abstraction-level acceptance.

Source frontmatter: pillar Platform · areas [catalog, domain, service, server] · design `docs/design/03-beyond-http.md`. **read**

Source `note:` field, quoted: “B10x sip.dial is source-grounded and development-proven; S-032 retains the stable-support matrix”

## Status

`in-progress` in the source. Quoted from `docs/stories/S-026-one-real-non-http-driver-proves-the-model.md:5`: `status: in-progress`. **read**

## Provenance

Migrated from `docs/stories/S-026-one-real-non-http-driver-proves-the-model.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-16 · 5 revision(s)
- Legacy id `S-026`, recorded as the reference `legacy:S-026`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
