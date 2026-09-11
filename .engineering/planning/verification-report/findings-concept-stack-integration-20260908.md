---
format: aep.planning-md/1
id: verification-report:findings-concept-stack-integration-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:concept-stack-integration-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 6e34f5b9c8934849170b9385214e8eaf99500b30619e5a4bc931ecabccdc0557
relations:
- verifies: review-result:concept-stack-integration-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:concept-stack-integration-20260908

This supplements [the immutable original](../review-result/concept-stack-integration-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

The original report's verdict and scope remain authoritative; no new verdict is assigned.

## Transcription method

10 findings remain in the scope of this report's final stated conclusion.
Closed items in a same-pass disposition and verification/command tables are excluded.
Each nonempty message reproduces a source section or table row verbatim, including
its original identifier and citations. P0/P1, major and blocking map to blocker;
P2, minor and should-fix map to warning; P3 and nit map to note. Ungraded observations
remain unspecified. No per-finding verdict or introduced/pre-existing classification
is invented. The file is the first explicit source citation; when only shorthand
citations are present, legacy-source-excerpt identifies the original report itself.
The full citation context remains in the message and original. This transcription
makes no claim that paraphrased findings across different historical rounds have
identical comparison signatures.

## Findings

```findings
[
  {
    "file": "crates/connectors-core/src/lib.rs",
    "category": "legacy-ungraded",
    "severity": "unspecified",
    "message": "| G1 | Caller context on the wire | `Invocation { version, request_id, operation, revision, input }` — no tenant, principal, realm, authority (`crates/connectors-core/src/lib.rs:92-99`) | `OwnerContext { tenant_id, agent_id, agent_revision, authority_snapshot_id, authority_snapshot_sha256 }` on every hosted request (`../connectors/crates/protocol/src/operation.rs:23-29`); verified tenant/realm/authority/executor before decoding, `None` ≠ `Some(\"default\")` | ADR 0026 |",
    "line": 92
  },
  {
    "file": "crates/connectors-host/src/server.rs",
    "category": "legacy-ungraded",
    "severity": "unspecified",
    "message": "| G2 | Admission | One shared static bearer, constant-time compare (`crates/connectors-host/src/server.rs:57-70`); no Identity dependency (`Cargo.lock`: 0 hits for `identity-client`) | Connectors owns and verifies an exact Identity audience (`../connectors/crates/server/src/hosted/routing.rs:31-41`); trusted access exchange into Connector scopes | ADR 0021, 0031; ROADMAP row `identity → connectors` (deployed) |",
    "line": 57
  },
  {
    "file": "contracts/operations/v1alpha1/semantics.md",
    "category": "legacy-ungraded",
    "severity": "unspecified",
    "message": "| G3 | Grants / approvals | `ErrorCode` has no grant code (`connectors-core/src/lib.rs:13-27`); approval binding exists only in the *proposed* mutation profile (`contracts/operations/v1alpha1/semantics.md:43-55, 100-109`, currently uncommitted) | Grant evaluation + one-time approval redemption are what O1 means by \"governed reach\" | ROADMAP O1; old `docs/design/13-grant-evaluation-and-approval-redemption.md` |",
    "line": 43
  },
  {
    "file": ".engineering/planning/review-result/concept-stack-integration-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "unspecified",
    "message": "| G4 | Secrets custody | `SecretStore::read(reference)` only (`connectors-sdk/src/lib.rs:42-44`); no `secrets-client` dependency | Connectors reaches Secrets through the released client and a workload identity; Vault path to be removed | ADR 0023; ROADMAP `connectors → secrets` (intended) |"
  },
  {
    "file": "docs/design.md",
    "category": "legacy-ungraded",
    "severity": "unspecified",
    "message": "| G5 | service-sdk factory seam | v2 design, contracts and adapter docs contain 0 mentions of `service-sdk`, `ConnectorServiceFactory` or any ADR (`grep -c` over `docs/design.md`, `contracts/`, `docs/adapters/`) | `service-connectors` implements old `ConnectorBackend` + `ConnectorServiceFactory` with `PrincipalContext`, `ServiceDeployment`, `ServiceDispatch`, `VerifiedAuthContext`, `GrantId`, `RealmId` (`../service-sdk/crates/service-connectors/src/lib.rs:13-33`); Todo → connectors → devcenter arrow depends on it | ADR 0027, 0029 |"
  },
  {
    "file": "../agent-platform/crates/agent-platform-connectors/src/lib.rs",
    "category": "legacy-ungraded",
    "severity": "unspecified",
    "message": "| G6 | Agent-platform projection | `Operation { id, description, contract, profile, input_schema, output_schema }` — no effect class, risk or approval posture (`connectors-core/src/lib.rs:61-69`) | `agent-platform-connectors` compiles `b10x.connector-operation.v0alpha1` `OperationDescription` with `EffectClass` and `ApprovalPosture` into a Harness toolset (`../agent-platform/crates/agent-platform-connectors/src/lib.rs:10-17`) | ROADMAP `agent-platform → connectors (projection)` |",
    "line": 10
  },
  {
    "file": "../workspace/crates/workspace-service/src/main.rs",
    "category": "legacy-ungraded",
    "severity": "unspecified",
    "message": "| G7 | Datasource seam | `datasource.records/v1alpha1` is pages + cursors + provenance; no binding, no `NotGranted`/`StaleAuthority` | Workspace imports `DatasourceBinding`, `DatasourceErrorCode::{NotGranted, StaleAuthority}` (`../workspace/crates/workspace-service/src/main.rs:24, 1112-1115`); org-brain intends `b10x.connector-datasource.v0alpha1` (`../org-brain/docs/design/org-brain-v0.1.md:76`) | ROADMAP `workspace → connectors`, `connectors → org-brain` |",
    "line": 24
  },
  {
    "file": "docs/design.md",
    "category": "legacy-ungraded",
    "severity": "unspecified",
    "message": "| G8 | MCP | Design mentions MCP only as \"can be a client-facing binding and/or an adapter\" (`docs/design.md` § 19); no `b10x-mcp-*` dependency | Connectors exposes a governed outbound MCP adapter and the hosted inbound `/mcp` transport | ADR 0028; ROADMAP `mcp → connectors` (pinned) |"
  },
  {
    "file": "crates/connectors-client/src/lib.rs",
    "category": "legacy-ungraded",
    "severity": "unspecified",
    "message": "| G9 | Client API surface | `Client`: `describe`, `invoke`, `send` (`crates/connectors-client/src/lib.rs:71-111`) | Old `HostedClient`: 12 async methods (`../connectors/crates/connectors-client/src/lib.rs:463-737`) incl. `issue_approval`, `connection`, `event`, `datasource` | consumer Cargo pins below |",
    "line": 71
  },
  {
    "file": "../atlas/docs/catalog.md",
    "category": "legacy-ungraded",
    "severity": "unspecified",
    "message": "| G10 | Crate-name collision | v2 workspace crate is named `connectors-client` (`Cargo.toml` workspace deps) | Atlas component id `connectors/connectors-client` already exists (`../atlas/docs/catalog.md:221`); 3 consumers pin that name by git | ADR 0017 (typed catalog) |",
    "line": 221
  }
]
```

