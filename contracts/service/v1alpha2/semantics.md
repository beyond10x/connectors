# Governed service binding v1alpha2

- **Status:** proposed, not implemented. Nothing in this document is advertised by any descriptor.
- **Family:** service. Predecessor: [service/v1alpha1](../v1alpha1/semantics.md) (implemented). Siblings that this envelope carries: [operations mutation profile](../../operations/v1alpha1/semantics.md), [auth.connection](../../auth/connection/v1alpha1/semantics.md), [auth.custody](../../auth/custody/v1alpha1/semantics.md).
- **Recorded:** 2026-09-08.
- **Why it exists:** the [preserved review of 2026-09-08](../../../.engineering/planning/review-result/concept-stack-integration-20260908.md) (gaps G1–G3, G6, G7, G9) found that the v1alpha1 wire and host carry no verified caller context, so nothing downstream of admission can be governed per caller. Every current consumer of the old Connectors passes an owner context and receives grant and audit facts. This document proposes the service envelope and admission boundary for that gap. The [compatibility decision](../compatibility.md) selects its local binding and exact public encoding; [one-hop delegation](../delegation.md) selects its proof/approval subjects; policy/audit persistence and runtime conformance remain explicit obligations.

## 1. Identity

| Field | Value |
|---|---|
| Wire version | `v1alpha2` on `/v1alpha2/describe` and `/v1alpha2/invoke`; legacy `/v1/...` is a separate explicit projection |
| Admission profiles | `static-bearer` (today's shared service token), `identity-audience` (Identity access token for an exact Connectors-owned audience), `delegated` (federation hop carrying a gateway-signed verified context) |
| Vocabulary | verified context (tenant, principal, optional realm, authority, optional executor), policy decision, grant reference, audit reference, executor assertion |
| Preserved structure | operation ids, input/output schemas, limits, success/error response variants and request correlation, `/healthz`, one-hop federation, descriptor revision and `stale_description` protocol |

The proposal changes response fields and the closed error-code vocabulary (§3.4–3.5),
including `audit_ref`; the serialized error envelope is therefore not unchanged.
The implemented v1alpha1 readers refuse those additions. The [compatibility decision](../compatibility.md) selects `v1alpha2` for this local
proposal and separates it from semantic-family and adapter-kind versions. No
implemented descriptor advertises it and no release/consumer rollout is implied.

A v1alpha2 service answers four questions separately, as `docs/design.md` § 6.2 requires: implemented by this artifact, enabled in this instance, ready with its dependencies, **authorized for this caller**. v1alpha1 answers the first three. The fourth needs a stable caller. Static bearer identifies one receiver-configured service principal, not the individual people sharing its bytes; it cannot provide individual caller isolation.

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| `OwnerContext { tenant_id, agent_id, agent_revision, authority_snapshot_id, authority_snapshot_sha256 }` written by the caller into every hosted request body; "local transport identity is additional evidence, never replaced by the caller-written owner context" | `../connectors/crates/protocol/src/operation.rs:23-40` | change: tenant, principal and realm are never read from the body (ADR 0026). `agent_id`, `agent_revision` and the authority snapshot become an **executor assertion** that the receiver verifies against its grant record; it grants nothing by itself |
| Hosted admission verifies an exact Identity audience owned by Connectors | `../connectors/crates/server/src/hosted/routing.rs:31-41` (`identity_audience: CONNECTORS_AUDIENCE`) | preserve as the `identity-audience` profile. Audience and scope bytes are Connectors deployment data and opaque to Identity (ADR 0021) |
| Trusted access exchange: a product BFF exchanges an external token for the minimum Connector scope, then "lets Connectors re-check the current description, Connection, Grant and approval evidence" | `../atlas/architecture/adr/0031-trusted-access-exchange-preserves-publication-confinement.md`, Decision | preserve; the exchange is Identity's; Connectors sees only the resulting token and re-checks everything itself |
| Grant evaluation is a separate policy from approval redemption | `docs/design.md` § 2.2 and § 2.3; old `docs/design/13-grant-evaluation-and-approval-redemption.md` | preserve: a `Policy` port decides admission; approval spending stays in the mutation profile |
| The old hosted read path can proceed through receiver policy when grant evaluation refuses or is unavailable | `docs/design.md` § 2.3, last paragraph | change: **fail closed**. Policy `Unavailable` is `unavailable` with no dispatch, for reads too. A `static-bearer` instance uses an explicit configured local policy; that is a configured fact, not a fallback |
| `None` and `Some("default")` realm are distinct in credentials, namespaces, idempotency scope, logs and Connector dispatch; Connectors propagates the verified optional realm across signed service dispatch | `../atlas/architecture/adr/0026-authentication-owns-optional-realm-context.md`, Decision | preserve in the verified context, cursor binding, idempotency scope, cache partition, audit and the `delegated` profile |
| Shared static service token compared in constant time | `crates/connectors-host/src/server.rs:57-70` | preserve as the `static-bearer` profile for local and test placements |
| `ResponseEnvelope.connector_audit_ref`, `execution_ref` on every hosted result | `../connectors/crates/protocol/src/operation.rs:185-190` | preserve acknowledged audit correlation; explicit unavailable/incomplete cases and source provenance use compatibility §5 |
| `DatasourceErrorCode::NotGranted`, `StaleAuthority` consumed by Workspace | `../workspace/crates/workspace-service/src/main.rs:1112-1115`; `../connectors/crates/protocol/src/datasource.rs:292-296` | preserve as error codes `not_granted` and `stale_authority` for every contract carried by this wire |
| `OperationDescription { operation_ref, title, description, input_schema, output_schema, effect, approval, connections, description_ref }` compiled by Agent Platform into a Harness toolset | `../connectors/crates/protocol/src/operation.rs:171-180`; `../agent-platform/crates/agent-platform-connectors/src/lib.rs:10-17` | preserve the information: every operation in a v1alpha2 descriptor carries `effects`, `risk`, `approval` from the mutation profile; `connections` comes from `auth.connection`; `description_ref` is the descriptor `revision` |
| Old operation contract versions `v0alpha2`/`v0alpha3` selected explicitly, "no negotiation or fallback" | installed `connectors operation --help`, `--protocol-version` | preserve the principle: a client speaks one explicitly selected codec per interaction; exact mismatch/refusal behavior is in compatibility §2 |

## 3. Types and binding

### 3.1 Authentication and admitted invocation context

```text
AuthenticatedContext {
  tenant: Option<opaque string>  # credential-verified; configured absence for local static principal
  principal: opaque string      # credential-verified or receiver-configured static service identity
  realm: Option<opaque string>  # absent differs from "default"
  authority: Option<AuthorityRef> # current governed authority, or explicit configured local policy
  credential_executor: Option<ExecutorRef>
}

AdmittedInvocationContext {
  authenticated: AuthenticatedContext
  executor: Option<ExecutorRef> # established by credential/configuration/grant validation
  grant_ref: Option<opaque string>
  connection: admitted connection
}
```

These are host values, never caller-written authority. The authentication context exists before application decoding; the admitted invocation context follows strict decode, connection/policy checks and verification of any executor assertion. A body assertion cannot overwrite authentication coordinates. If a credential binds an executor, require exact match; otherwise the policy must independently establish the asserted binding or refuse it. Local static identity and tenant/realm absence are stable configuration facts, not credential bytes or token-rotation identities. No missing/unavailable governed policy falls back to local policy.

### 3.2 Descriptor

[Compatibility §4](../compatibility.md#4-extended-descriptor-and-invocation-surface) is the complete closed field contract. Root additions are admission, safe auth_profiles, a managed-connections flag and safe provides metadata. Operation additions are curation, optional auth alternatives/realization, and explicit limits. The operation profile remains singular. Profile/provider registration internals and credential references are not copied into this safe descriptor.

Successful `GET /v1alpha2/describe` wraps this complete Descriptor in the common Response’s `result`, with `request_id: null` and required audit metadata. Both version fields equal `v1alpha2`; errors use the same envelope without a result. Legacy describe keeps its bare Descriptor shape.

Under identity-audience/delegated, operations are the caller-visible projection of implemented, enabled and policy-admitted operations at describe time. Invoke re-evaluates current admission. The revision binds the selected codec/projection and relevant configuration; a descriptor is not a grant. Reserved or unbound profiles are omitted, even if a vocabulary or implementation sketch names them.

### 3.3 Invocation

The original required version/request_id/operation/revision/input fields remain. The extended codec admits optional connection, idempotency_key, approval and executor assertion with the exact shapes in compatibility §4. A read profile does not gain mutation or connection authority merely because its codec understands those members; unsupported combinations are refused before dispatch.

For example, a managed read may select `connection: "conn_1"`; a mutation may additionally carry `approval: {reference, evidence}` and a keyed operation requires idempotency_key under its profile. A malformed assertion is invalid_input; an assertion that fails its trusted executor/grant binding is not_granted. Absence is accepted only when policy admits direct principal execution. The raw assertion is never an already verified context.

### 3.4 Responses and audit

Exact fields and consistency rules are in [compatibility §5](../compatibility.md#5-extended-responses-audit-and-mutation-observation). These illustrative complete envelope shapes use empty placeholder business results:

```json
{ "version": "v1alpha2", "request_id": "read-1", "status": "success", "result": {}, "audit_ref": "aud-1", "audit_status": "complete" }
{ "version": "v1alpha2", "request_id": null, "status": "error", "error": { "code": "unavailable", "message": "audit admission unavailable" }, "audit_ref": null, "audit_status": "unavailable" }
```

Mutation observation is under `mutation`, with classification, original attempt/request identity, replayed and optional secondary cause as specified in compatibility §5. A replay retains its original classification; replayed is never an effect. An admitted provider success followed by failed outcome persistence may still return success/applied with a persistence cause. If a result cannot safely be delivered after a known effect, error/applied preserves that fact. Unknown is always error/outcome_unknown, independent of a timeout cause.

The receiver's audit_ref/audit_status and optional `{instance,audit_ref,audit_status}` source_audit distinguish gateway and leaf. An unacknowledged audit write never produces an invented reference. Audited reads and mutations need acknowledged pre-dispatch audit admission, then a separate linked final observation. Final audit failure preserves known/unknown business facts and reports incomplete audit; it cannot retrospectively prevent dispatch. Refusals before valid authentication/decoding leave unknown audit coordinates absent and request_id null. The explicit unavailable/no-ref failure path replaces the earlier impossible mandatory-reference promise.

### 3.5 Errors

The closed extended code set is listed in compatibility §5. not_granted is host policy refusal; forbidden is a selected adapter/connection/resource refusal. stale_authority requires fresh authority and a new describe before a deliberate new interaction. Payload states, private auth outcomes and media terminal reasons are not automatically ErrorCode values. Current v1alpha1 readers reject new codes; no error is tunneled through message text to pretend compatibility.

### 3.6 Delegated federation — selected proposal

[One-hop delegated approval and dispatch](../delegation.md) now owns the complete canonical subject, safe preparation read, fixed Ed25519 JWS types, request/receiver/GET correlation, bounded time and durable nonce rules, and sole executing-leaf approval redemption. Its separate proof types replace the withdrawn HMAC sketch; they do not extend the predecessor module-request v1 token. Provider credentials remain at the executing leaf, and a broad downstream bearer supplies no originating caller authority.

The `host.approval.prepare` operation uses the ordinary extended envelope and its own selected `approval-subject` payload/limits, covering both configured and managed connections. It performs no provider use, business reservation or approval spend. Preparation/issuer presentation is not execution authority. Current policy, exact subject reconstruction and the existing keyed-outcome lookup still precede new approval redemption. Direct and federated origin are distinct; only the leaf spends and owns the business attempt.

The protocol is specified, not implemented. Delegated support remains unadvertised until the selected verifier, policy, clock, uniqueness stores and conformance exist. Multi-hop and event-claim ingress remain unavailable; no automatic resign/resend or broad-token fallback is allowed.

## 4. Admission and execution rules

1. Select configured endpoint and fixed version path, apply bounded transport/framing, authenticate into AuthenticatedContext, strictly decode the chosen envelope, validate version/correlation/schema/revision, resolve connection within trusted scope, evaluate current policy and verify executor assertion, then apply mutation approval/ledger rules and dispatch. No application input chooses tenant/principal/realm/authority or expands the destination.
2. Policy failure or unavailable authority is fail-closed for reads and mutations. Describe-time visibility is not a grant. Connection refusal does not reveal whether another scope's connection exists.
3. Realm absence is distinct from Some(default) in policy, idempotency, cursors, cache and audit. Stable local static principal identity does not change when credential bytes rotate.
4. Audited invocation first acknowledges an audit admission, then follows any mutation attempt preparation/spending/dispatch fence, then records its outcome. Audit and attempt records have distinct purposes. Failure handling follows compatibility §5; no log line replaces a durable claim/attempt.
5. Federation uses only the selected per-hop codec/profile intersection and a complete trusted delegation binding. No credential/context substitution, downgrade, reroute or resend follows a refusal or lost response.
6. Legacy projection follows compatibility §3: off by default, explicit unchanged static reads only, exact projected operation/revision checked again by invoke. Hidden mutations cannot be reached with a guessed ID or the new/full descriptor revision.

## 5. Limits

Operation request/result/execution/provider/connect limits are explicit descriptor values from compatibility §7. The ordinary profile retains 64 KiB / 4 MiB and 20 s / 15 s / 5 s; every generic realization (read, mutation or page) requires 256 KiB / 4 MiB and 40 s / 30 s / 5 s. Policy evaluation's proposed 2 s ceiling is inside, not added to, the operation execution budget. Authentication, transport ingress, callbacks, session/control/data and delegation need their own bounded binding rules; ordinary unary support cannot stand in for those rules.

Identity token lifetime/verification comes from the explicitly selected Identity contract and deployment, not a universal lifetime invented by this document. The independent delegation/approval proof windows and nonce/spend rules are fixed by [delegation §5](../delegation.md#5-time-deadlines-and-durable-uniqueness). Audit admission and completion must be bounded inside the execution/delivery policy while preserving actual effect knowledge when a reply cannot be sent.

## 6. Conformance requirements

- The old reader matrix in compatibility §2 distinguishes shape-compatible version mismatch from unknown fields, bare errors and response correlation failure. No automatic fallback or invalid_input-to-unsupported rewrite.
- A new client explicitly selecting legacy codec sees only legacy semantics. Unsupported profile/version/authority fails before invoke; an invalid response after a sent mutation yields uncertainty, not safe retry permission.
- Guessed hidden mutation on legacy invoke is refused before dispatch; a new/full descriptor revision is not a projection revision.
- Missing/invalid credential or unknown envelope fields never create trusted body-sourced context. Safe pre-envelope replies use nullable request correlation and honest audit status/reference.
- Executor mismatch -> not_granted; policy outage -> unavailable, no dispatch. Describe/invoke authority changes are rechecked; realm None and default stay distinct.
- Admission-audit failure -> unavailable/no acknowledged ref/no dispatch. Final-audit failure after known success/refusal preserves that effect and original cause; a possible dispatch remains unknown.
- Every mutation outcome, including replayed refusal/abort/unknown, an uncertain waiter, inaccessible original and outcome-store failure, follows the exact status/classification/cause table.
- Delegation uses the selected canonical preparation/approval trace and exact proof fixtures in delegation.md, including valid/forged/replayed/expired/different-receiver/different-body, optional realm/executor, and describe/prepare/invoke separation. Actual verifier/store/provider conformance remains required before advertisement.

## 7. Compatibility

[The common compatibility document](../compatibility.md) supersedes all earlier projection/version defaults here. The extended service envelope retains v1 semantic-family names; it changes no adapter-specification format. Explicit legacy selection is a separate codec and admitted surface, not stripping fields from an arbitrary governed operation.

Old OwnerContext fields are characterization input, not trusted new context. Tenant/principal/realm must be established again by authentication; executor claims require receiver verification. Description and audit references need explicit facade mapping without equating old execution_ref to an attempt ID merely because both are strings. Historical destructive classifications require reviewed mutation curation. Old CLI protocol flags and source dependency names have separate migration decisions; a multi-codec client is allowed when selection is explicit and no fallback occurs.

## 8. SDK and host obligations

| Obligation | Owner / boundary |
|---|---|
| Two explicit strict codecs/routes, descriptor/profile/limit selection, safe early errors and exact observation/audit shapes | future core/client/host binding; current crates stay v1alpha1 |
| Authentication, current policy and executor/grant verification | host ports; static configuration, Identity audience and delegation are explicit different bindings |
| Audit admission plus linked final observation and truthful failure handling | host persistence port; no prescribed JSONL/store implementation here |
| Mutation attempt/approval/idempotency semantics | operations contract and existing ESS domains; wire values do not implement the dispatch fence |
| Delegated request/approval subjects and verification | [F03 proposal](../delegation.md); runtime binding must implement and prove it, with no broad-token substitute |
| Safe auth/connection metadata and protected acquisition ingress | auth family owner stories; ordinary invoke is not secret entry |
| Adapter authoring curation and generated schemas | Connectors-owned kind/compiler compatibility decision separate from this wire; no silent strict-format extension |
| CLI, service-sdk, Secrets, Identity and consumer bindings | later explicitly scoped implementation/modeling work; no external consumer or Atlas mutation authorized here |

## 9. ESS values and unresolved owner models

Public Version, ErrorCode, MutationObservation, DiagnosticCause, AttemptReference, AuditStatus/SourceAudit, AuthAlternative and OperationLimits values are typed in [service_wire.yaml](../../../ess/domains/service_wire.yaml). Effect knowledge and attempt identity reference the existing mutations types. This is no runtime decoder or public attempt lookup API; required-null versus omitted encoding and cross-field predicates are explicit codec obligations.

Authenticated/admitted contexts remain host values whose trusted derivation must be modeled with their owner bindings. AuditRecord/GrantRecord storage, audit-admission/final-observation identity and their relations to AttemptRecord remain **UNMAPPED** until persistence ownership is settled. The former guessed audit-record cardinality is withdrawn. Connection tenant/principal ownership stays with its auth owner; this document introduces no stub entity or inferred cardinality to satisfy validation.

## 10. Remaining binding decisions

Implement and prove the selected F03 delegation binding; choose and model policy/audit persistence bindings; verify executor and Identity/Secrets integration against their authoritative contracts; define protected callback and duplex/data codecs; and run version/profile/failure conformance before any advertisement. The wire identifier, paths, field/error shapes and legacy projection rules in compatibility.md are this local proposed contract, not unresolved implementation defaults. Consumer rollout, package installation, Atlas registration and external publication remain separately authorized work.
