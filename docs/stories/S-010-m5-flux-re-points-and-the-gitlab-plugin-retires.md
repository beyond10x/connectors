---
id: S-010
title: "M5 — flux re-points at the platform and the gitlab plugin is deleted"
pillar: Clients
status: backlog
priority:
design: docs/design/02-architecture.md
epic: build-order
areas: [protocol, server, docs]
note: "architecture §9 milestone M5. Exit: flux invokes gitlab through the platform; the gitlab plugin deleted. Wave 1 of flux-roadmap decision 0024 (alignment note 2026-08-13: the target surface is this platform), under 0026 §4's one-way dependency arrow"
---

# M5 — flux re-points at the platform and the gitlab plugin is deleted

## Goal

Prove the client contract against its first native client: flux holds one Service Account token,
projects the effective catalogue into tools, invokes and subscribes — and the first native plugin
retires onto the declared surface, deleted rather than deprecated. The seam keeps the shape decision
0026 §4 fixed; only the trust domain and release origin change.

## Acceptance

- [ ] flux's embedded platform client drives the whole client contract against this platform's
      **published protocol identities**: authenticate once, discover the effective catalogue and
      project it to tools, invoke, subscribe. This repository depends on nothing of flux — the
      dependency arrow exists only at runtime, in one direction, over versioned contracts
      (decision 0026 §4).
- [ ] flux's CLI manages a **personal-posture** instance: verified download, supervised local
      process, zero-configuration `connectors serve`, owner-bound state — decision 0004's managed
      local install retargeted to the selfdirect trust domain and release origin, without the
      predecessor's ~31k-LOC local-management ceremony (0026's fate table: the supervision contract
      is redesigned lean, not carried).
- [ ] **Wave 1 parity, measured the way decision 0024 §4 requires**: gitlab's frozen behavioral
      fixtures pass through the platform over the op inventory the plugin **actually serves**
      (descriptor grants plus event-store audit evidence), not its whole surface.
- [ ] The gitlab plugin artifact is **deleted** in the same flux release train as its proven
      replacement — per plugin, no batching of unproven cutovers.
- [ ] Decision 0024's wave-0 preconditions are met or explicitly re-scoped against the new surface,
      each named in this story's Progress: self-serve onboarding (connect / grant / list / doctor) is
      real, `flux app run` constructs the platform client with the same fail-closed
      withdraw-on-refresh-failure semantics as the interactive path, and a credential migration path
      exists (re-acquisition is acceptable; silently copying secrets out of the local store is not).
- [ ] A client whose token is unchanged survives a connection reauthorization and a platform restart
      without re-authenticating — the "authenticate once" claim, proven rather than asserted.

## Progress
- (not started)

## Notes

- Exit criterion, verbatim from architecture §9: *"flux invokes gitlab through the platform; the
  gitlab plugin deleted."*
- Decisions: `~/projects/flux-roadmap/decisions/0024-every-native-plugin-retires-onto-a-declared-surface.md`
  (routing table, wave order, the fixture rule, and its 2026-08-13 alignment note retargeting the
  surface) and `0026-the-family-consolidates-into-selfdirect-connectors.md` §4.
- gitlab is wave 1 because its connector is shipped and decision 0001's frozen migration fixtures
  already exist. Waves 2–6 (slack/jira/confluence/opsgenie, the observability set behind declared
  destinations, aws/huggingface's new credential schemes, kubernetes/docker/sql/websearch, and the
  final host deletion) are follow-on stories, not this one.
- Depends on [S-008](S-008-m3-connect-a-provider-and-invoke-it.md) and, for gitlab's channel/event
  surface, [S-009](S-009-m4-events-reach-a-client-by-push-and-by-pull.md).
