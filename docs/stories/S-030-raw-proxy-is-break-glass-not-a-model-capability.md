---
id: S-030
title: "Raw proxy is break-glass authority, not a model capability"
pillar: Platform
status: backlog
priority:
design: docs/design/01-domain-model.md
epic: carried-constraints
areas: [domain, protocol, service, server]
note: "architecture closed by omitting raw proxy from generic v1; implementation fences remain"
---

# Raw proxy is break-glass authority, not a model capability

## Goal

Ensure arbitrary credential-bearing provider requests cannot bypass reviewed operation facts or be
mistaken for ordinary granted catalog execution.

## Acceptance

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

## Progress

- The architectural branch is closed: generic v1 omits raw authenticated proxying. Design 01 and
  the M3 story require credential-bearing calls to use declared operations.
- Remaining work is negative/fence coverage proving the unimplemented path cannot reappear; any
  later operator break-glass implementation must satisfy the conservative alternative above.
