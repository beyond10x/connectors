---
id: S-011
title: "Deployment-declared destination aperture"
pillar: Platform
status: backlog
priority:
design:
epic: carried-constraints
areas: [service, server, domain]
note: "ported from flux-exchange X-143 and restated here as B10x authority. A deployment declares the aperture; a member, model input, Service Account, or raw-proxy request cannot widen it; matching is post-resolution"
---

# Deployment-declared destination aperture

## Goal

Open exactly one narrow aperture in the egress rule: a deployment whose own infrastructure
legitimately reaches an in-cluster service declares which destinations are admitted, as configuration,
value-free — and everything undeclared stays refused **after resolution**. Refusing every private
destination unconditionally is correct for the open internet and exactly wrong for an org-posture
deployment whose value is reaching its own Grafana; admitting one on a request field would hand
destination selection to a model.

## Acceptance

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

## Progress
- (not started)

## Notes

- Predecessor evidence: [`X-143 — deployment-declared destination aperture`](https://github.com/codewandler/flux-exchange/blob/main/docs/stories/X-143-deployment-declared-destination-aperture.md).
  Private roadmap records explain its lineage, but the complete normative rule is restated in this
  story and does not depend on those records being reachable.
- Domain model, Integration: the destination policy is deployment-operator authority and a
  **value-free** allowlist; open question 2 of the domain model asks whether it belongs on
  Integration or on a deployment-global document with per-integration references — this story is
  where that question gets answered, so record the answer in the design series.
- Consumed by [S-008](S-008-m3-connect-a-provider-and-invoke-it.md) (invoke), S-030 (any later raw
  proxy), and [S-009](S-009-m4-events-reach-a-client-by-push-and-by-pull.md) (channel supervisor).
  Private-endpoint provider work cannot start without it.
