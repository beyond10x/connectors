---
id: S-011
title: "Deployment-declared destination aperture"
pillar: Platform
status: backlog
priority:
design:
epic: carried-constraints
areas: [service, server, domain]
note: "ported from flux-exchange X-143 (one of decision 0026's named migration set). The question is authority, not reachability: the deployment declares an explicit, value-free allowlist; a member, a model input or a Service Account can never select, widen or substitute a destination; matching is post-resolution, so DNS rebinding cannot smuggle one in"
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
- [ ] **No request field, catalog entry, connection config value or grant can name a destination.**
      The allowlist is deployment configuration only; audit records and receipts stay value-free
      (they may record that a destination policy admitted, never which address).
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

- Predecessor: [`X-143 — deployment-declared destination aperture`](https://github.com/codewandler/flux-exchange/blob/main/docs/stories/X-143-deployment-declared-destination-aperture.md)
  — ported per decision 0026's named set (X-143, X-156, C-540/C-541, C-552), not re-derived. Its
  lineage: flux-roadmap decision 0019 rule 3 (the deployment declares admitted egress destinations)
  and 0008 rule 4 (identical post-resolution matching by request construction and permission subject).
- Domain model, Integration: the destination policy is deployment-operator authority and a
  **value-free** allowlist; open question 2 of the domain model asks whether it belongs on
  Integration or on a deployment-global document with per-integration references — this story is
  where that question gets answered, so record the answer in the design series.
- Consumed by [S-008](S-008-m3-connect-a-provider-and-invoke-it.md) (invoke and proxy) and
  [S-009](S-009-m4-events-reach-a-client-by-push-and-by-pull.md) (channel supervisor); decision 0024
  wave 3 (grafana, prometheus, loki, alertmanager, homer) cannot start without it.
