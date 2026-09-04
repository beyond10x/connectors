---
format: aep.planning-md/1
id: story:m5-flux-re-points-and-the-gitlab-plugin-retires
kind: story
status: active
title: M5 — flux re-points at the platform and the gitlab plugin is deleted
refs:
- provider: legacy
  reference: S-010
relations:
- derived_from: epic:build-order
scope:
- confidence: cited
  path: crates/protocol
- confidence: cited
  path: crates/server
- confidence: cited
  path: docs
revision: 4
---
## Acceptance

Verbatim from `docs/stories/S-010-m5-flux-re-points-and-the-gitlab-plugin-retires.md:24`. **read**

- [ ] flux's embedded platform client drives the whole client contract against this platform's
      **published protocol identities**: authenticate once, discover the effective catalogue and
      project it to tools, invoke, subscribe. This repository depends on nothing of flux — the
      dependency arrow exists only at runtime, in one direction, over versioned contracts
      ([dependency rules](https://github.com/beyond10x/b10x/blob/bf6859717f986dc0e2a3b8a713e087d426741d92/architecture/architecture/dependency-rules.md)).
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
- [ ] A client whose still-valid presented authority is unchanged survives a vendor Connection
      reauthorization and a platform restart without repeating Identity login or personal-local
      bootstrap. Normal Identity expiry/rotation remains Identity-owned and does not become a
      Connectors service-token lifecycle.

## Context

Prove the client contract against its first native client: Flux presents one current
Connectors-audience Identity authority in hosted posture or the owner-held local bearer in personal
posture, projects the effective catalogue into tools, invokes and subscribes — and the first native
plugin retires onto the declared surface, deleted rather than deprecated. Connectors mints neither
the hosted authority nor vendor credentials. The seam keeps B10x's one-way dependency rule
fixed; only the trust domain and release origin change.

Source frontmatter: pillar Clients · areas [protocol, server, docs] · design `docs/design/02-architecture.md`. **read**

Source `note:` field, quoted: “Externally gated: Flux must record B10x adoption in its own repository before this family can schedule M5. Private predecessor records are provenance; this story restates the normative parity and cutover rules”

## Status

`blocked` in the source. Quoted from `docs/stories/S-010-m5-flux-re-points-and-the-gitlab-plugin-retires.md:5`: `status: blocked`. **read**

## Provenance

Migrated from `docs/stories/S-010-m5-flux-re-points-and-the-gitlab-plugin-retires.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-24 · 9 revision(s)
- Legacy id `S-010`, recorded as the reference `legacy:S-010`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
