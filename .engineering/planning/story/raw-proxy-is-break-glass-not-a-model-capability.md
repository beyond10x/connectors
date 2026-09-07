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
scope:
- confidence: inferred
  path: crates/domain
- confidence: inferred
  path: crates/protocol
- confidence: cited
  path: crates/server
- confidence: cited
  path: crates/service/src/dispatch.rs
- confidence: inferred
  path: ess/system/domains/runtime.yaml
revision: 4
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

## Scope

Derived 2026-09-07 by `story-scoper` through read-only inspection — cited.

- **Primary surface:** `crates/server` — cited; hosted admission/MCP refusal boundaries and connection-bound destination enforcement already live here.
- **Dispatch and audit:** `crates/service/src/dispatch.rs:47` — cited; `DispatchPolicy` and `AuditSink` own pre-dispatch enforcement and audit.
- **Authority facts:** `crates/domain` — inferred; any implemented break-glass aperture requires authority distinct from ordinary grants and destructive/max-effect classification.
- **Protocol fences:** `crates/protocol` — inferred; negative fixtures should prove arbitrary credential-bearing requests cannot enter the ordinary operation protocol.
- **Specification:** `ess/system/domains/runtime.yaml:191` — inferred; an operational break-glass feature would need to resolve the existing deferred Proxy model.
- **Documents:** none required for omission-only regression fences — inferred.
- **Symbols:** `OperationRequest`, `DispatchPolicy`, `AuditSink`, `DestinationRule` — cited.
- **Confidence:** low — inferred; the draft mixes protecting an omitted capability with acceptance requiring that capability to execute.
- **Would collide with:** hosted admission/MCP boundaries, destination policy, dispatch audit, grant evaluation, operation protocol fences and runtime-domain specification — inferred.
