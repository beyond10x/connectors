---
format: aep.planning-md/1
id: story:deployment-declared-destination-aperture
kind: story
status: draft
title: Deployment-declared destination aperture
refs:
- provider: legacy
  reference: S-011
relations:
- derived_from: epic:carried-constraints
revision: 1
---
## Acceptance

Verbatim from `docs/stories/S-011-deployment-declared-destination-aperture.md:24`. **read**

- [ ] A deployment-declared destination list (the `[egress]` section of `platform.toml`, plus the
      integration's destination policy for private-host providers) admits a named private destination
      for connections whose integration requires it; everything undeclared stays refused, proven by
      fixtures that include **a public hostname resolving to a private address** — matching happens
      post-resolution, so DNS rebinding cannot smuggle a destination past the check.
- [ ] The aperture is posture-scoped and each posture's behaviour is proven by a refusal test: **org**
      admits per its declaration; **saas** refuses unconditionally; **personal** keeps the owner's
      local rule. A posture never inherits another's aperture by default.
- [ ] **No request field, catalog entry, connection config value or grant can name or widen a
      destination.** The allowlist is deployment configuration only. Audit records never contain
      credentials or raw secret-bearing URLs, but retain a stable policy-rule id and a normalized
      destination hash so incident response can determine which aperture admitted a call.
- [ ] Any operator-only raw proxy also requires a separately granted method/path aperture and is
      evaluated by this same post-resolution destination policy; an arbitrary path cannot convert a
      destination grant into arbitrary provider authority.
- [ ] Every composition point consumes **one shared policy**: the `server` egress module is the only
      place a vendor socket is opened (architecture §5), the invocation path and the channel
      supervisor both go through it, and a census/fence test refuses a **third** composition point
      appearing — the predecessor's failure mode was two dialers composing the same policy
      independently and drifting.
- [ ] A malformed or unparsable egress declaration admits **nothing** and refuses startup by name —
      fail closed, consistent with the operator policy rule.

## Context

Open exactly one narrow aperture in the egress rule: a deployment whose own infrastructure
legitimately reaches an in-cluster service declares which destinations are admitted, as configuration,
value-free — and everything undeclared stays refused **after resolution**. Refusing every private
destination unconditionally is correct for the open internet and exactly wrong for an org-posture
deployment whose value is reaching its own Grafana; admitting one on a request field would hand
destination selection to a model.

Source frontmatter: pillar Platform · areas [service, server, domain]. **read**

Source `note:` field, quoted: “ported from flux-exchange X-143 and restated here as B10x authority. A deployment declares the aperture; a member, model input, Identity service principal, connector Grant, or raw-proxy request cannot widen it; matching is post-resolution”

## Status

`backlog` in the source. Quoted from `docs/stories/S-011-deployment-declared-destination-aperture.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-011-deployment-declared-destination-aperture.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-09-02 · 7 revision(s)
- Legacy id `S-011`, recorded as the reference `legacy:S-011`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
