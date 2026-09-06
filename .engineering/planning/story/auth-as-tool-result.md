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
- confidence: inferred
  path: contracts/connector-connection/v0alpha2
- confidence: inferred
  path: contracts/connector-connection/v0alpha2/README.md
- confidence: inferred
  path: contracts/connector-connection/v0alpha2/bundle.json
- confidence: inferred
  path: contracts/connector-connection/v0alpha2/connector-connection.schema.json
- confidence: inferred
  path: contracts/connector-connection/v0alpha2/vectors.json
- confidence: inferred
  path: contracts/connector-connection/v0alpha2/vectors.schema.json
- confidence: inferred
  path: contracts/connector-operation/v0alpha3
- confidence: inferred
  path: contracts/connector-operation/v0alpha3/README.md
- confidence: inferred
  path: contracts/connector-operation/v0alpha3/bundle.json
- confidence: inferred
  path: contracts/connector-operation/v0alpha3/connector-operation.schema.json
- confidence: inferred
  path: contracts/connector-operation/v0alpha3/vectors.json
- confidence: inferred
  path: contracts/connector-operation/v0alpha3/vectors.schema.json
- confidence: inferred
  path: crates/connectors-cli/README.md
- confidence: cited
  path: crates/connectors-cli/src/error.rs
- confidence: inferred
  path: crates/connectors-cli/src/lib.rs
- confidence: inferred
  path: crates/connectors-cli/tests/cli_surface.rs
- confidence: inferred
  path: crates/connectors-cli/tests/one_shot_operations.rs
- confidence: inferred
  path: crates/connectors-cli/tests/remediation.rs
- confidence: cited
  path: crates/connectors-client/src/identity.rs
- confidence: inferred
  path: crates/connectors-client/src/lib.rs
- confidence: inferred
  path: crates/connectors-client/src/model.rs
- confidence: inferred
  path: crates/connectors-client/src/personal_oauth.rs
- confidence: inferred
  path: crates/connectors-client/src/remediation.rs
- confidence: inferred
  path: crates/connectors-client/src/response.rs
- confidence: inferred
  path: crates/connectors-console/src/connect.rs
- confidence: inferred
  path: crates/connectors-console/src/envelope.rs
- confidence: inferred
  path: crates/connectors-console/src/lib.rs
- confidence: cited
  path: crates/connectors-console/src/output.rs
- confidence: cited
  path: crates/connectors-console/src/output_tests.rs
- confidence: inferred
  path: crates/connectors-console/src/remediation.rs
- confidence: inferred
  path: crates/connectors-console/tests/remediation.rs
- confidence: cited
  path: crates/connectors-runtime/src/composition.rs
- confidence: inferred
  path: crates/connectors-runtime/src/one_shot.rs
- confidence: cited
  path: crates/connectors-runtime/src/registry.rs
- confidence: inferred
  path: crates/connectors-runtime/src/registry_claims_tests.rs
- confidence: inferred
  path: crates/connectors-runtime/src/remediation_tests.rs
- confidence: inferred
  path: crates/connectors-runtime/tests/one_shot_runtime.rs
- confidence: cited
  path: crates/connectors-runtime/tests/personal_oauth.rs
- confidence: inferred
  path: crates/integration-catalog/src/hosted.rs
- confidence: inferred
  path: crates/integration-catalog/src/lib.rs
- confidence: cited
  path: crates/integration-catalog/src/oauth.rs
- confidence: cited
  path: crates/integration-catalog/src/oauth_acquisition.rs
- confidence: inferred
  path: crates/integration-catalog/src/oauth_remediation.rs
- confidence: inferred
  path: crates/integration-catalog/src/oauth_remediation_tests.rs
- confidence: cited
  path: crates/integration-catalog/src/oauth_tests.rs
- confidence: inferred
  path: crates/integration-catalog/src/remediation_tests.rs
- confidence: cited
  path: crates/protocol/README.md
- confidence: inferred
  path: crates/protocol/examples/connection_v2_bundle.rs
- confidence: inferred
  path: crates/protocol/examples/operation_v3_bundle.rs
- confidence: inferred
  path: crates/protocol/src/connection_v2.rs
- confidence: inferred
  path: crates/protocol/src/connection_v2_schema.rs
- confidence: cited
  path: crates/protocol/src/lib.rs
- confidence: cited
  path: crates/protocol/src/operation.rs
- confidence: inferred
  path: crates/protocol/src/operation/schema_v3.rs
- confidence: inferred
  path: crates/protocol/src/operation/v3.rs
- confidence: inferred
  path: crates/protocol/src/operation/versions.rs
- confidence: cited
  path: crates/protocol/tests/bundles.rs
- confidence: cited
  path: crates/server/src/hosted.rs
- confidence: cited
  path: crates/server/src/hosted/connection_route.rs
- confidence: inferred
  path: crates/server/src/hosted/docs.rs
- confidence: inferred
  path: crates/server/src/hosted/docs/openapi.json
- confidence: inferred
  path: crates/server/src/hosted/docs/openapi.schema.json
- confidence: cited
  path: crates/server/src/hosted/enforcement.rs
- confidence: inferred
  path: crates/server/src/hosted/mcp.rs
- confidence: cited
  path: crates/server/src/hosted/mcp/toolset.rs
- confidence: cited
  path: crates/server/src/hosted/operation_transport.rs
- confidence: inferred
  path: crates/server/src/hosted/remediation.rs
- confidence: cited
  path: crates/server/src/hosted/routing.rs
- confidence: inferred
  path: crates/server/src/hosted/tests/contract_validation.rs
- confidence: inferred
  path: crates/server/src/hosted/tests/docs.rs
- confidence: inferred
  path: crates/server/src/hosted/tests/enforcement.rs
- confidence: inferred
  path: crates/server/src/hosted/tests/mcp.rs
- confidence: inferred
  path: crates/server/src/hosted/tests/remediation.rs
- confidence: cited
  path: crates/server/src/local.rs
- confidence: cited
  path: crates/service/src/connect_session.rs
- confidence: inferred
  path: crates/service/src/lib.rs
- confidence: inferred
  path: crates/service/src/remediation.rs
- confidence: inferred
  path: crates/service/src/remediation_tests.rs
- confidence: cited
  path: crates/service/src/runtime.rs
- confidence: inferred
  path: docs/architecture/authority.md
- confidence: inferred
  path: docs/architecture/interfaces.md
- confidence: inferred
  path: docs/design/22-authentication-remediation.md
- confidence: inferred
  path: ess/generated/clap/crates/connectors-cli/src/main.rs
- confidence: inferred
  path: ess/system/components.yaml
- confidence: inferred
  path: ess/system/domains/connection.yaml
- confidence: inferred
  path: ess/system/domains/runtime.yaml
- confidence: cited
  path: json-schemas.toml
revision: 60
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

## Independent additive protocol slice — 2026-09-06

Published Atlas proposal bfb731377a448ab89443c44852d622601f6c1363 contains the required dated authentication extension to current ADR0041 and separate ADR0044 for Operationv3/Connectionv2. Design22 now records that current citation, the fresh consumer audit and proposed status, retaining earlier0040 paragraphs as history. Complete reviewed rate source/test0c026610 and locally gated schema3 publication candidate0c694509 are retained in this integration history. The full12gate and immutable first/final reviews establish the exact v2 prerequisite; they establish no OAuth runtime completion.

Assign one independent managed protocol slice to the rate implementor after its publication verification handoff. Exact new Rust owners: crates/protocol/src/operation/v3.rs, operation/schema_v3.rs, operation/versions.rs, connection_v2.rs, connection_v2_schema.rs, and examples/operation_v3_bundle.rs plus connection_v2_bundle.rs. Existing hooks are limited to module registrations/API documentation in operation.rs/lib.rs, additive tests in tests/bundles.rs, protocol/README.md and json-schemas.toml. Add exactly README.md,bundle.json,protocol schema,vectors.json,vectors.schema.json under each new Operationv0alpha3 and Connectionv0alpha2 bundle. Root owns AEP/ESS/Git/publication; the implementor owns this exact source/test/generator output scope and its scratch evidence.

Freeze byte-for-byte all existing complete Operationv1/v2 and Connectionv1 bundle files, contracts/artifact-bundle.schema.json, operation/legacy.rs, operation/wire.rs, operation/schema.rs, examples/operation_v2_bundle.rs and connection.rs against the provided baseline. Preserve the internal wire v2 reexport, every previous assertion and rate URI grammar/vector class. No dependency/manifest/lock update is assigned. The prepared stage1-protocol-plan.md is the bounded technical brief; its older ADR0040 and prerequisite pointers are historical, superseded by the exact current citations above.

Use selected original request/response bytes for strict decoding so duplicate fields cannot disappear through a Value intermediate. Operationv3 auth payload is mandatory exactly for authentication_required, non-null, non-retriable and not_attempted with exact safe references; reject incompatible payload/code/success/retry-delay combinations and unknown fields. Pure downgrade to v1/v2 is neutral bounded Unavailable with no URL/capability/auth payload/delay. Preserve ordinary/rate projection behavior and reject unknown identities with no fallback/resend. Connectionv2 input is bounded64KiB inside a128KiB frame; v1 remains64KiB. Bound-to-v1 conversion is a typed refusal, never unbound session creation. Validate nested trusted status and duplicate identities consistently, without claiming a DTO performs real-time expiry, grants or one-use acknowledgement.

Prepare meaningful deciding cases before implementation, retaining actual first executions and compiler/fixture failures separately. Exercise complete ordinary/bound variants, malformed references, profile fields, boundaries, duplicate keys, all unknown/mismatched versions, exact auth downgrade, frozen artifacts and independent Rust/Draft2020-12 vector checks. Prove deterministic regeneration twice and manifest hashes. Compile only with the coordinator's explicit slot, jobs1, own target and prospective12GiB reserve; preserve source/evidence before completed-target cleanup. The OAuth implementor owns the current compile slot, so begin with noncompiling inspection/fixtures. Return complete source/evidence report for independent review. No server/runtime/registry/client/CLI/console/config/custody/session/grant/helper wiring, external consumer changes, live provider actions, release or tag is assigned.

## Prospective ownership for the reviewed OAuth handoff — 2026-09-06

The frozen OAuth source at 6fae9df000986f39d009a2e8503bdff03db41762 supplies the concrete owners that were still provisional in the stage2-runtime-handoff-plan.md proposal. Its first formal whole-unit review is running. Record the following additional machine scopes prospectively; this does not release implementation or a compiling slot before the coordinator hands over reviewed OAuth source. Current protocol source remains 05c94ac457938d2f9a8059f7f905dd8a87ec4dca. No runtime completion or review verdict is asserted.

Root compared every path with current story revision 44; all fifteen were absent. Existing file hashes and the purpose of each prospective edit are in auth-runtime-prospective-scope-comparison.json in the assigned coordinator scratch. The source locations are from the frozen OAuth commit; new owners and conditional helper reuse are marked inferred.

| Confidence | Path | Assigned responsibility |
| --- | --- | --- |
| cited | crates/integration-catalog/src/oauth.rs | Existing policy/session owner; binding and trait delegation only. |
| cited | crates/integration-catalog/src/oauth_acquisition.rs | Share existing exact-index acquisition with bound start. |
| inferred | crates/integration-catalog/src/oauth_remediation.rs | New private bound readiness/start/status/ack owner. |
| inferred | crates/integration-catalog/src/oauth_remediation_tests.rs | New additive exact binding and acknowledgement tests. |
| cited | crates/integration-catalog/src/oauth_tests.rs | Additive regressions, retaining all reviewed OAuth cases. |
| cited | crates/server/src/hosted/operation_transport.rs | Original-byte version dispatch with exact response identity. |
| cited | crates/server/src/hosted/routing.rs | Connection outer-frame bounds and existing authentication route wiring. |
| cited | crates/server/src/hosted/mcp/toolset.rs | Safe authentication projection through actual admitted operation seam. |
| cited | crates/connectors-client/src/identity.rs | Typed 409 without Identity renewal or resend; existing 401 behavior preserved. |
| inferred | crates/connectors-client/src/personal_oauth.rs | If needed, narrow reuse of reviewed private instruction fetch/validation; no generic decoder weakening. |
| cited | crates/connectors-console/src/output.rs | Explicit safe authentication output projection. |
| cited | crates/connectors-console/src/output_tests.rs | Actual hostile valid-envelope formatter tests. |
| cited | crates/connectors-cli/src/error.rs | Nonzero structured authentication result handling. |
| cited | crates/connectors-runtime/src/composition.rs | Default protocol/readiness change only with coordinated supported transport. |
| cited | crates/connectors-runtime/tests/personal_oauth.rs | Additive real composition witnesses; retain all reviewed OAuth assertions. |

Preserve every reviewed OAuth, raw, custody, rate and GitLab assertion. Existing typed test owners hosted/tests/remediation.rs, CLI/tests/remediation.rs and runtime/src/remediation_tests.rs remain the intended test families where they suffice; do not create duplicate authentication_remediation.rs families. The new private OAuth remediation module shares the existing Session map, binding gate, custody owner and exact-index acquisition path. It adds no parallel journal, credential store, policy subsystem or durable session entity.

The separately frozen service-port proposal has 45 evidence members, all independently rehashed by the coordinator; manifest SHA256 74233b129a3e8b8133a970008d39322895fafff51d3b98502648ef23d99437b1. Its three source paths already have typed ownership. Patch application checks pass without source changes. Its tests remain unexecuted, and its constructor-only stable-authority seed hypothesis remains unassigned and not a blocker: no shipped authentication producer for the proposed pair was established.

Measured module sizes at the OAuth source are registry 1,489 lines, client lib 1,500, hosted 1,452, composition 1,463 and CLI lib 1,471. Additional extraction owners require an exact minimal proposal and coordinator scope before edits; this note grants no size waiver or broad refactor. Preserve original test bodies and source preimages in any later explicitly assigned extraction.

The personal authority seam uses the actual immutable configuration for that process, its real configured grant_ref with no fabricated revision, and the existing mutable CurrentAuthority/generation/evidence checks. Restart forgets ephemeral sessions. Hosted wire/grant tests must truthfully keep production acquisition Unsupported until a supported adapter is composed. A valid DTO or an allowed field name alone does not make arbitrary daemon references or messages safe output; the later client/console/MCP paths require closed or exact request-bound projections and actual hostile-envelope cases.
