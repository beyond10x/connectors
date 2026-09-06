---
format: aep.planning-md/1
id: story:auth-as-tool-result
kind: story
status: active
title: An admitted Connection has a structured authentication next step
tags:
- ready
- wave-cli
refs:
- provider: legacy
  reference: S-014
relations:
- derived_from: epic:carried-constraints
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: contracts/connector-connection/v0alpha2
- confidence: cited
  path: contracts/connector-connection/v0alpha2/README.md
- confidence: cited
  path: contracts/connector-connection/v0alpha2/bundle.json
- confidence: cited
  path: contracts/connector-connection/v0alpha2/connector-connection.schema.json
- confidence: cited
  path: contracts/connector-connection/v0alpha2/vectors.json
- confidence: cited
  path: contracts/connector-connection/v0alpha2/vectors.schema.json
- confidence: cited
  path: contracts/connector-operation/v0alpha3
- confidence: cited
  path: contracts/connector-operation/v0alpha3/README.md
- confidence: cited
  path: contracts/connector-operation/v0alpha3/bundle.json
- confidence: cited
  path: contracts/connector-operation/v0alpha3/connector-operation.schema.json
- confidence: cited
  path: contracts/connector-operation/v0alpha3/vectors.json
- confidence: cited
  path: contracts/connector-operation/v0alpha3/vectors.schema.json
- confidence: cited
  path: crates/catalog-cli/tests/offline_binary.rs
- confidence: cited
  path: crates/connectors-cli/Cargo.lock
- confidence: cited
  path: crates/connectors-cli/README.md
- confidence: cited
  path: crates/connectors-cli/src/error.rs
- confidence: cited
  path: crates/connectors-cli/src/lib.rs
- confidence: cited
  path: crates/connectors-cli/src/tests.rs
- confidence: cited
  path: crates/connectors-cli/tests/cli_surface.rs
- confidence: cited
  path: crates/connectors-cli/tests/closed_pipe.rs
- confidence: cited
  path: crates/connectors-cli/tests/one_shot_operations.rs
- confidence: cited
  path: crates/connectors-cli/tests/remediation.rs
- confidence: cited
  path: crates/connectors-client/Cargo.toml
- confidence: cited
  path: crates/connectors-client/src/identity.rs
- confidence: cited
  path: crates/connectors-client/src/lib.rs
- confidence: cited
  path: crates/connectors-client/src/model.rs
- confidence: cited
  path: crates/connectors-client/src/personal_oauth.rs
- confidence: cited
  path: crates/connectors-client/src/remediation.rs
- confidence: cited
  path: crates/connectors-client/src/response.rs
- confidence: cited
  path: crates/connectors-client/src/tests.rs
- confidence: cited
  path: crates/connectors-console/Cargo.lock
- confidence: cited
  path: crates/connectors-console/src/connect.rs
- confidence: cited
  path: crates/connectors-console/src/envelope.rs
- confidence: cited
  path: crates/connectors-console/src/lib.rs
- confidence: cited
  path: crates/connectors-console/src/output.rs
- confidence: cited
  path: crates/connectors-console/src/output_tests.rs
- confidence: cited
  path: crates/connectors-console/src/remediation.rs
- confidence: cited
  path: crates/connectors-console/tests/remediation.rs
- confidence: cited
  path: crates/connectors-runtime/Cargo.lock
- confidence: cited
  path: crates/connectors-runtime/src/composition.rs
- confidence: cited
  path: crates/connectors-runtime/src/composition_tests.rs
- confidence: cited
  path: crates/connectors-runtime/src/lib.rs
- confidence: cited
  path: crates/connectors-runtime/src/one_shot.rs
- confidence: cited
  path: crates/connectors-runtime/src/registry.rs
- confidence: cited
  path: crates/connectors-runtime/src/registry_claims_tests.rs
- confidence: cited
  path: crates/connectors-runtime/src/registry_tests.rs
- confidence: cited
  path: crates/connectors-runtime/src/remediation_tests.rs
- confidence: cited
  path: crates/connectors-runtime/tests/one_shot_runtime.rs
- confidence: cited
  path: crates/connectors-runtime/tests/personal_oauth.rs
- confidence: cited
  path: crates/integration-catalog/src/hosted.rs
- confidence: cited
  path: crates/integration-catalog/src/lib.rs
- confidence: cited
  path: crates/integration-catalog/src/oauth.rs
- confidence: cited
  path: crates/integration-catalog/src/oauth_acquisition.rs
- confidence: cited
  path: crates/integration-catalog/src/oauth_remediation.rs
- confidence: cited
  path: crates/integration-catalog/src/oauth_remediation_tests.rs
- confidence: cited
  path: crates/integration-catalog/src/oauth_tests.rs
- confidence: cited
  path: crates/integration-catalog/src/remediation_tests.rs
- confidence: cited
  path: crates/protocol/README.md
- confidence: cited
  path: crates/protocol/examples/connection_v2_bundle.rs
- confidence: cited
  path: crates/protocol/examples/operation_v3_bundle.rs
- confidence: cited
  path: crates/protocol/src/connection_v2.rs
- confidence: cited
  path: crates/protocol/src/connection_v2_schema.rs
- confidence: cited
  path: crates/protocol/src/lib.rs
- confidence: cited
  path: crates/protocol/src/operation.rs
- confidence: cited
  path: crates/protocol/src/operation/schema_v3.rs
- confidence: cited
  path: crates/protocol/src/operation/v3.rs
- confidence: cited
  path: crates/protocol/src/operation/versions.rs
- confidence: cited
  path: crates/protocol/tests/bundles.rs
- confidence: cited
  path: crates/server/Cargo.toml
- confidence: cited
  path: crates/server/src/hosted.rs
- confidence: cited
  path: crates/server/src/hosted/connection_route.rs
- confidence: cited
  path: crates/server/src/hosted/docs.rs
- confidence: cited
  path: crates/server/src/hosted/docs/openapi.json
- confidence: cited
  path: crates/server/src/hosted/docs/openapi.schema.json
- confidence: cited
  path: crates/server/src/hosted/enforcement.rs
- confidence: cited
  path: crates/server/src/hosted/mcp.rs
- confidence: cited
  path: crates/server/src/hosted/mcp/toolset.rs
- confidence: cited
  path: crates/server/src/hosted/operation_transport.rs
- confidence: cited
  path: crates/server/src/hosted/remediation.rs
- confidence: cited
  path: crates/server/src/hosted/routing.rs
- confidence: cited
  path: crates/server/src/hosted/tests.rs
- confidence: cited
  path: crates/server/src/hosted/tests/contract_validation.rs
- confidence: cited
  path: crates/server/src/hosted/tests/docs.rs
- confidence: cited
  path: crates/server/src/hosted/tests/enforcement.rs
- confidence: cited
  path: crates/server/src/hosted/tests/mcp.rs
- confidence: cited
  path: crates/server/src/hosted/tests/remediation.rs
- confidence: cited
  path: crates/server/src/local.rs
- confidence: cited
  path: crates/service/src/connect_session.rs
- confidence: cited
  path: crates/service/src/lib.rs
- confidence: cited
  path: crates/service/src/remediation.rs
- confidence: cited
  path: crates/service/src/remediation_tests.rs
- confidence: cited
  path: crates/service/src/runtime.rs
- confidence: cited
  path: docs/architecture/authority.md
- confidence: cited
  path: docs/architecture/deployment.md
- confidence: cited
  path: docs/architecture/interfaces.md
- confidence: cited
  path: docs/design/22-authentication-remediation.md
- confidence: cited
  path: ess/generated/clap/PLAN.md
- confidence: cited
  path: ess/generated/clap/TARGET.md
- confidence: cited
  path: ess/generated/clap/crates/connectors-cli/Cargo.toml
- confidence: cited
  path: ess/generated/clap/crates/connectors-cli/src/handler.rs
- confidence: cited
  path: ess/generated/clap/crates/connectors-cli/src/main.rs
- confidence: cited
  path: ess/generated/clap/crates/connectors-cli/src/tree.rs
- confidence: cited
  path: ess/system/components.yaml
- confidence: cited
  path: ess/system/domains/connection.yaml
- confidence: cited
  path: ess/system/domains/runtime.yaml
- confidence: cited
  path: json-schemas.toml
revision: 42
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

## Current scoped acceptance and implementation decision — 2026-09-06

Keep the original acceptance and context in `story:auth-as-tool-result` and `docs/stories/S-014-auth-as-tool-result.md` unchanged as history. Append the following scoped acceptance and update the current title to “An admitted Connection has a structured authentication next step” through the coordinator's planning transaction:

- An authenticated invocation which passes the exact Connection/operation grant and reaches credential preflight before any operation dispatch can return a typed `authentication_required` response under ConnectorOperation v0alpha3. It names the already admitted operation, Connection, derived integration and credential purpose, distinguishes initial authorization from reauthorization, and states `not_attempted` and the trusted next action. It contains no URL, session capability, provider code, registration or credential material.
- A configured stable Created binding with no callable description uses an explicit trusted ConnectorConnection v0alpha2 remediation start. The receiver derives admission metadata from that exact configured binding without advertising it as a callable member, evaluates the current operation grant, and creates a real single-purpose session only after management/self-service authority also permits it. Neither route creates a nonexistent placement or uses a label/profile to select another binding.
- Unknown and unadmitted identities receive the same ordinary opaque refusal. An absent grant store is an outage. Neither the existing hosted read-path fallback nor a generic `Unavailable`, socket failure, channel reconnect state, provider 403 or ambiguous execution becomes authentication authority.
- Trusted outer CLI/control plane can receive the existing protected human completion URL or instruction projection, poll the bound session, and acknowledge completion once. Every request revalidates the current principal, exact target and applicable grant; expired, failed, consumed and mismatched sessions cannot yield a resume acknowledgement. Model/MCP output remains an explicit safe allowlist with no URL or capability.
- Initial authorization and reauthorization retain the configured Connection reference. Completion for another integration/Connection is refused before credential publication where the owner can observe it, and is also rejected at acknowledgement. A session cannot add scopes, select credentials, change routes or replace a grant.
- After acknowledgement the caller reads a fresh operation description, validates its intended input against that description, and explicitly submits a new invocation. The server never saves or resends the invocation. `OutcomeUnknown`, timeout after dispatch, protocol mismatch and rate advice never trigger this loop automatically.
- A still-valid personal-local identity or Connectors-audience Identity token completes the loop without a new Identity login. Token rotation with the same admitted principal/realm is supported; expiry, revocation, scope reduction, owner changes and changed policy are re-evaluated, not bypassed by a session.
- Positive and adversarial fixtures cover both new protocol identities, exact downgrade loss, wrong versions, grant opacity, Created versus absent, credential versus transport failure, one-use completion, authority rotation/expiry, trusted/model output and zero dispatch before explicit resumption. Provider and affected in-repository client/server gates pass before a runtime completion claim.

These changes resolve two conflicts in the legacy acceptance: grants do not authorize creation of a missing Connection, and ConnectSession URLs are not model tool results (`docs/design/01-domain-model.md:88`, `:193`, `:289`; `crates/domain/src/grant.rs:131`).


Design22 and ten additive ESS value types are recorded as implementation preparation. The complete model validates and compiles; these checks prove no runtime behavior. Rate v2 and personal OAuth runtime remain prerequisites for shared-source changes. Changed wire implementation waits for a separately recorded Atlas auth migration, preserving the rate decision and its acceptance independently.

## Publication continuation — 2026-09-07

The same approved CLI wave continues from public main 0c69450921ab1794c81dadec915b717a61bf0983. The coordinator carries the tested nonplanning source into this isolated publication checkout and records its exact content equivalence before CI. Raw execution records and the original implementation branches remain preserved separately. Published branch history is not rewritten. This public store keeps its existing journal prefix and records current scope and evidence through AEP.

Seven selected stories already reached published main. Personal OAuth passed its two ordinary reviews; the first whole-authentication review is running against the frozen identical source. The several-credentials story remains held under its existing decision blocker. No version bump, release tag, live provider action or additional credential-store support is authorized here. Source publication, normal documentation delivery, supported source installation and the two already authorized daemon replacements remain in the approved delivery sequence.

Current source includes Operation v3 default with explicit operation-only v2, Connection v2 bound remediation, schema4 with matching readers, and development-only GitLab public PKCE/device authorization through a dedicated unsealed store. Hosted remediation remains Unsupported after admission where no acquisition owner exists. Protected instructions stay outside model output; completion ends with fresh validation and an explicit new invocation. Full affected local gates pass: server115, runtime455 per default/no-default configuration with two existing PostgreSQL ignores, console107, CLI140, strict affected checks and the final catalog/Markdown/story/ESS gate. Initial root1301pass/2fail/4oldignores was closed by full affected server115 and catalog-cli14 rechecks. Complete sharded CI and auth review remain pending.

Exact runtime final seal: eba21e5f93152eb0aacd01dadf31a19569401421c8a8397f3b09216d11ed7b04 (179 members). Exact client final seal:334ab4c9adf141862dcb9a701e930f80dcdf909dfb3a1c1142dc75af3c8fcae7 (95 members). All first failures remain in those records. Counts describe executed cases per command, not a unique aggregate. Future review results are recorded in full before any findings are routed.
