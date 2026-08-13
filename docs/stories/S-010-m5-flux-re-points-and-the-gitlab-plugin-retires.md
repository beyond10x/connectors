---
id: S-010
title: "M5 — flux re-points at the platform and the gitlab plugin is deleted"
pillar: Clients
status: backlog
priority:
design: docs/design/02-architecture.md
epic: build-order
areas: [protocol, server, docs]
note: "architecture §9 milestone M5. Exit: flux invokes gitlab through the platform and the gitlab plugin is deleted. Private predecessor records are provenance; this story restates the normative parity and cutover rules"
---

# M5 — flux re-points at the platform and the gitlab plugin is deleted

## Goal

Prove the client contract against its first native client: flux holds one Service Account token,
projects the effective catalogue into tools, invokes and subscribes — and the first native plugin
retires onto the declared surface, deleted rather than deprecated. The seam keeps B10x's
one-way dependency rule fixed; only the trust domain and release origin change.

## Acceptance

- [ ] flux's embedded platform client drives the whole client contract against this platform's
      **published protocol identities**: authenticate once, discover the effective catalogue and
      project it to tools, invoke, subscribe. This repository depends on nothing of flux — the
      dependency arrow exists only at runtime, in one direction, over versioned contracts
      ([dependency rules](https://github.com/b10x/architecture/blob/main/architecture/dependency-rules.md)).
- [ ] flux's CLI manages a **personal-posture** instance: verified download, supervised local
      process, zero-configuration `connectors serve`, owner-bound state — carrying forward the
      predecessor's managed-local-install requirement without its ~31k-LOC local-management
      ceremony; the supervision contract is redesigned lean, not copied.
- [ ] **First-wave parity**: gitlab's frozen behavioral
      fixtures pass through the platform over the op inventory the plugin **actually serves**
      (descriptor grants plus event-store audit evidence), not its whole surface.
- [ ] The gitlab plugin artifact is **deleted** in the same flux release train as its proven
      replacement — per plugin, no batching of unproven cutovers.
- [ ] The following cutover preconditions are met or explicitly re-scoped against the new surface,
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
- Provenance: an unpublished Flux plugin-retirement record supplied the original routing table and
  wave order. It is not required authority: the fixture, parity, precondition, and same-release
  deletion rules are restated in this story. B10x's durable ownership and dependency
  decisions are [ADR 0009](https://github.com/b10x/architecture/blob/main/adr/0009-b10x-agent-is-provider-and-harness-agnostic.md),
  [ADR 0006](https://github.com/b10x/architecture/blob/main/adr/0006-b10x-supersedes-selfdirect-housing.md),
  and the [dependency rules](https://github.com/b10x/architecture/blob/main/architecture/dependency-rules.md).
- gitlab is wave 1 because its connector is shipped and frozen migration fixtures
  already exist. Waves 2–6 (slack/jira/confluence/opsgenie, the observability set behind declared
  destinations, aws/huggingface's new credential schemes, kubernetes/docker/sql/websearch, and the
  final host deletion) are follow-on stories, not this one.
- Depends on [S-008](S-008-m3-connect-a-provider-and-invoke-it.md) and, for gitlab's channel/event
  surface, [S-009](S-009-m4-events-reach-a-client-by-push-and-by-pull.md).
