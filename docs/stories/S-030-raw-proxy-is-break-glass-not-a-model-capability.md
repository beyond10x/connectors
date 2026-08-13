---
id: S-030
title: "Raw proxy is break-glass authority, not a model capability"
pillar: Platform
status: backlog
priority:
design: docs/design/01-domain-model.md
epic: carried-constraints
areas: [domain, protocol, service, server]
note: "closes the review finding that risk=high/network+write was not the worst case for arbitrary credentialed vendor access"
---

# Raw proxy is break-glass authority, not a model capability

## Goal

Ensure arbitrary credential-bearing provider requests cannot bypass reviewed operation facts or be
mistaken for ordinary granted catalog execution.

## Acceptance

- [ ] Generic v1 omits raw proxy, or an operator-only build/config gate exposes it as a distinct
      destructive/max-effects capability unavailable to models and ordinary Service Accounts.
- [ ] A separate method/path aperture and S-011 destination policy both admit the request; neither
      request data nor a catalog grant can widen them.
- [ ] Audit identifies break-glass authority, actor, policy rule, normalized destination hash,
      method/path class, and result without recording credential or secret-bearing values.
- [ ] Undeclared credential-bearing model calls are refused and direct users to add a reviewed
      catalog operation.
- [ ] Delete and money-movement refusal fixtures prove the old `risk=high` classification cannot
      reappear.

## Progress

- (not started)
