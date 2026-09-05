---
format: aep.planning-md/1
id: story:auth-as-tool-result
kind: story
status: proposed
title: 'Not-connected is a next step: the response carries a connect URL'
tags:
- ready
- wave-cli
refs:
- provider: legacy
  reference: S-014
relations:
- derived_from: epic:carried-constraints
scope:
- confidence: inferred
  path: contracts
- confidence: cited
  path: crates/client/src/identity.rs
- confidence: cited
  path: crates/client/src/lib.rs
- confidence: cited
  path: crates/client/src/response.rs
- confidence: inferred
  path: crates/connectors-cli/src/lib.rs
- confidence: cited
  path: crates/connectors-runtime/src/registry.rs
- confidence: cited
  path: crates/domain/src/grant.rs
- confidence: cited
  path: crates/integration-gitlab/src/backend.rs
- confidence: cited
  path: crates/integration-slack/src/backend/api_runtime.rs
- confidence: cited
  path: crates/integration-slack/src/backend/connection_runtime.rs
- confidence: cited
  path: crates/protocol/src/connection.rs
- confidence: cited
  path: crates/protocol/src/operation.rs
- confidence: cited
  path: crates/protocol/tests/bundles.rs
- confidence: cited
  path: crates/server/src/hosted.rs
- confidence: cited
  path: crates/server/src/hosted/admission.rs
- confidence: cited
  path: crates/server/src/hosted/connection_route.rs
- confidence: cited
  path: crates/server/src/hosted/enforcement.rs
- confidence: inferred
  path: crates/server/src/hosted/tests
- confidence: cited
  path: crates/server/src/local.rs
- confidence: cited
  path: crates/service/src/connect_session.rs
- confidence: cited
  path: crates/service/src/runtime.rs
- confidence: inferred
  path: ess/system/domains/connection.yaml
- confidence: inferred
  path: ess/system/domains/runtime.yaml
revision: 37
---
## Acceptance

Verbatim from `docs/stories/S-014-auth-as-tool-result.md:22`. **read**

- [ ] A grant-admitted invocation against a missing or degraded connection returns a **distinct,
      structured protocol response** under its own protocol identity, carrying: the integration it
      needs, a connect URL backed by a freshly created connect session, that session's expiry, and how
      to resume. It is not an HTTP status shared with authorization refusals, and it is not an error
      string a client has to pattern-match.
- [ ] It is **not an authorization bypass**: the response is produced only *after* grant admission
      admits the operation. A principal whose grants do not admit the operation receives the ordinary
      refusal and learns nothing about which connections exist or which integrations are configured —
      no enumeration oracle, consistent with the grant rule that a refusal never names the axis that
      refused.
- [ ] The connect URL carries no credential and no tenant secret; the session is single-purpose
      and bound to exactly the integration the operation needs (the domain model's
      `allowed_integrations` binding), so handing the URL to a human cannot widen anything.
- [ ] **Degraded is distinguished from missing**: a degraded connection yields a
      **reauthorize-in-place** URL and the connection id is unchanged afterwards; a missing connection
      yields a create-connection URL. A client that stored the connection id keeps it in the first
      case.
- [ ] The client can complete the loop with the same still-valid personal-local or
      Connectors-audience Identity authority: after the human finishes vendor authorization, the same
      invocation succeeds without a second Identity login or local bootstrap. Identity expiry and
      rotation remain Identity-owned. The end-to-end test includes the wait/poll shape a client uses
      to learn the Connect Session completed.
- [ ] The response shape is fixture-covered in `protocol` conformance, positive and adversarial
      (expired session, session completed for a different integration, session already consumed), so
      a future SDK test suite shares the fixtures verbatim.

## Context

When a granted invocation needs a connection that does not exist or has degraded, answer with a
**structured next step** — a connect URL a human can complete, backed by a real connect session —
instead of an error an agent can only report. This is the pattern designed for an agent that hits the
wall mid-task, and it is a client-contract promise in the vision, not a convenience.

Source frontmatter: pillar Platform · areas [protocol, service, server]. **read**

Source `note:` field, quoted: “research §5 pattern 8, called out there as 'the single most agent-native pattern found' (Arcade authorize→waitForCompletion, Pipedream MCP returning a connect URL, Composio Connect Links mid-conversation). Vision, client contract: not-connected is a next step, not an error”

## Status

`backlog` in the source. Quoted from `docs/stories/S-014-auth-as-tool-result.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-014-auth-as-tool-result.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-14 · 2 revision(s)
- Legacy id `S-014`, recorded as the reference `legacy:S-014`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill

## Readiness

Selected for implementation on 2026-09-06 at the operator's request. wave-cli: 4 of 10. The ready tag records selection; proposed is the pre-implementation lifecycle state, and existing active work stays active. Existing dependencies and implementation evidence requirements still apply.

## Scope

Derived 2026-09-06 by aep-drive story-scoper; coordinator records the returned surfaces.

- `crates/protocol/src/operation.rs` — cited.
- `crates/protocol/src/connection.rs` — cited.
- `contracts` — inferred.
- `crates/protocol/tests/bundles.rs` — cited.
- `crates/domain/src/grant.rs` — cited.
- `crates/service/src/runtime.rs` — cited.
- `crates/service/src/connect_session.rs` — cited.
- `crates/server/src/hosted.rs` — cited.
- `crates/server/src/hosted/admission.rs` — cited.
- `crates/server/src/hosted/enforcement.rs` — cited.
- `crates/server/src/hosted/connection_route.rs` — cited.
- `crates/server/src/local.rs` — cited.
- `crates/connectors-runtime/src/registry.rs` — cited.
- `crates/client/src/lib.rs` — cited.
- `crates/client/src/response.rs` — cited.
- `crates/client/src/identity.rs` — cited.
- `crates/connectors-cli/src/lib.rs` — inferred.
- `crates/integration-slack/src/backend/api_runtime.rs` — cited.
- `crates/integration-slack/src/backend/connection_runtime.rs` — cited.
- `crates/integration-gitlab/src/backend.rs` — cited.
- `ess/system/domains/connection.yaml` — inferred.
- `ess/system/domains/runtime.yaml` — inferred.
- `crates/server/src/hosted/tests` — inferred.

Low confidence until prerequisites are settled. Existing grants bind a Connection and cannot authorize missing-connection creation implicitly. Domain authority forbids ConnectSession URLs in model tool results: authentication remediation belongs to the trusted outer CLI/control-plane boundary. Distinguish transport outage from credential expiry, bind repair to the original Connection, and complete personal OAuth custody first. Frozen wire changes require the coordinated migration authority, not editing v0alpha1 in place.

Would collide with any unit editing these files; directory entries require an additional containment review because AEP compares scope strings exactly.

## Execution queue

CLI execution queue 2026-09-06: 10 of 10. The urgent Slack delivery follow-up leads the queue. Original wave-cli readiness ordering remains historical context. Dependencies and measured scope govern dispatch order; priority is not a claim that prerequisites have landed.
