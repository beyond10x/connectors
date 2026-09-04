---
format: aep.planning-md/1
id: story:raw-proxy-is-break-glass-not-a-model-capability
kind: story
status: draft
title: Raw proxy is break-glass authority, not a model capability
refs:
- provider: legacy
  reference: S-030
relations:
- derived_from: epic:carried-constraints
revision: 1
---
## Acceptance

Verbatim from `docs/stories/S-030-raw-proxy-is-break-glass-not-a-model-capability.md:20`. **read**

- [x] Generic v1 omits raw proxy. Any later operator-only build/config gate must expose it as a
      distinct destructive/max-effects capability unavailable to models, ordinary Identity service
      principals, and ordinary connector Grants.
- [ ] A separate method/path aperture and S-011 destination policy both admit the request; neither
      request data nor a catalog grant can widen them.
- [ ] Audit identifies break-glass authority, actor, policy rule, normalized destination hash,
      method/path class, and result without recording credential or secret-bearing values.
- [ ] Undeclared credential-bearing model calls are refused and direct users to add a reviewed
      catalog operation.
- [ ] Delete and money-movement refusal fixtures prove the old `risk=high` classification cannot
      reappear.

## Context

Ensure arbitrary credential-bearing provider requests cannot bypass reviewed operation facts or be
mistaken for ordinary granted catalog execution.

Source frontmatter: pillar Platform · areas [domain, protocol, service, server] · design `docs/design/01-domain-model.md`. **read**

Source `note:` field, quoted: “architecture closed by omitting raw proxy from generic v1; implementation fences remain”

## Status

`backlog` in the source. Quoted from `docs/stories/S-030-raw-proxy-is-break-glass-not-a-model-capability.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-030-raw-proxy-is-break-glass-not-a-model-capability.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-14 · 3 revision(s)
- Legacy id `S-030`, recorded as the reference `legacy:S-030`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
