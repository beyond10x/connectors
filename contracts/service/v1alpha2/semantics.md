# Governed service binding v1alpha2

- **Status:** proposed, not implemented. Nothing in this document is advertised by any descriptor.
- **Family:** service. Predecessor: [service/v1alpha1](../v1alpha1/semantics.md) (implemented). Siblings that this envelope carries: [operations mutation profile](../../operations/v1alpha1/semantics.md), [auth.connection](../../auth/connection/v1alpha1/semantics.md), [auth.custody](../../auth/custody/v1alpha1/semantics.md).
- **Recorded:** 2026-09-08.
- **Why it exists:** the review of 2026-09-08 (`.local/review/2026-09-08-concept-and-stack-integration-review.md`, gaps G1–G3, G6, G7, G9) found that the v1alpha1 wire and host carry no verified caller context, so nothing downstream of admission can be governed per caller. Every current consumer of the old Connectors passes an owner context and receives grant and audit facts. This document is the one wire change that closes that gap; it is the "Beyond10x service binding" that `docs/design.md` § 7 reserves without specifying.

## 1. Identity

| Field | Value |
|---|---|
| Wire version | `v1alpha2` for every contract carried by `/v1/describe` and `/v1/invoke` |
| Admission profiles | `static-bearer` (today's shared service token), `identity-audience` (Identity access token for an exact Connectors-owned audience), `delegated` (federation hop carrying a gateway-signed verified context) |
| Vocabulary | verified context (tenant, principal, optional realm, authority, optional executor), policy decision, grant reference, audit reference, executor assertion |
| Unchanged | operation ids, input/output schemas, limits, error envelope shape, `/healthz`, one-hop federation, descriptor revision and `stale_description` protocol |

A v1alpha2 service answers four questions separately, as `docs/design.md` § 6.2 requires: implemented by this artifact, enabled in this instance, ready with its dependencies, **authorized for this caller**. v1alpha1 answers the first three. The fourth needs a caller, and a shared static bearer is not one.

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| `OwnerContext { tenant_id, agent_id, agent_revision, authority_snapshot_id, authority_snapshot_sha256 }` written by the caller into every hosted request body; "local transport identity is additional evidence, never replaced by the caller-written owner context" | `../connectors/crates/protocol/src/operation.rs:23-40` | change: tenant, principal and realm are never read from the body (ADR 0026). `agent_id`, `agent_revision` and the authority snapshot become an **executor assertion** that the receiver verifies against its grant record; it grants nothing by itself |
| Hosted admission verifies an exact Identity audience owned by Connectors | `../connectors/crates/server/src/hosted/routing.rs:31-41` (`identity_audience: CONNECTORS_AUDIENCE`) | preserve as the `identity-audience` profile. Audience and scope bytes are Connectors deployment data and opaque to Identity (ADR 0021) |
| Trusted access exchange: a product BFF exchanges an external token for the minimum Connector scope, then "lets Connectors re-check the current description, Connection, Grant and approval evidence" | `../atlas/architecture/adr/0031-trusted-access-exchange-preserves-publication-confinement.md`, Decision | preserve; the exchange is Identity's; Connectors sees only the resulting token and re-checks everything itself |
| Grant evaluation is a separate policy from approval redemption | `docs/design.md` § 2.2 and § 2.3; old `docs/design/13-grant-evaluation-and-approval-redemption.md` | preserve: a `Policy` port decides admission; approval spending stays in the mutation profile |
| The old hosted read path can proceed through receiver policy when grant evaluation refuses or is unavailable | `docs/design.md` § 2.3, last paragraph | change: **fail closed**. Policy `Unavailable` is `unavailable` with no dispatch, for reads too. A `static-bearer` instance uses an explicit allow-all policy; that is a configured fact, not a fallback |
| `None` and `Some("default")` realm are distinct in credentials, namespaces, idempotency scope, logs and Connector dispatch; Connectors propagates the verified optional realm across signed service dispatch | `../atlas/architecture/adr/0026-authentication-owns-optional-realm-context.md`, Decision | preserve in the verified context, cursor binding, idempotency scope, cache partition, audit and the `delegated` profile |
| Shared static service token compared in constant time | `crates/connectors-host/src/server.rs:57-70` | preserve as the `static-bearer` profile for local and test placements |
| `ResponseEnvelope.connector_audit_ref`, `execution_ref` on every hosted result | `../connectors/crates/protocol/src/operation.rs:185-190` | preserve as `audit_ref` on every governed response, success or error |
| `DatasourceErrorCode::NotGranted`, `StaleAuthority` consumed by Workspace | `../workspace/crates/workspace-service/src/main.rs:1112-1115`; `../connectors/crates/protocol/src/datasource.rs:292-296` | preserve as error codes `not_granted` and `stale_authority` for every contract carried by this wire |
| `OperationDescription { operation_ref, title, description, input_schema, output_schema, effect, approval, connections, description_ref }` compiled by Agent Platform into a Harness toolset | `../connectors/crates/protocol/src/operation.rs:171-180`; `../agent-platform/crates/agent-platform-connectors/src/lib.rs:10-17` | preserve the information: every operation in a v1alpha2 descriptor carries `effects`, `risk`, `approval` from the mutation profile; `connections` comes from `auth.connection`; `description_ref` is the descriptor `revision` |
| Old operation contract versions `v0alpha2`/`v0alpha3` selected explicitly, "no negotiation or fallback" | installed `connectors operation --help`, `--protocol-version` | preserve the principle: a client speaks one wire version; mismatch is `unsupported` |

## 3. Types

### 3.1 Verified context (host-internal, never a caller-written wire field)

```text
VerifiedContext {
  tenant:    opaque string          # from the verified credential
  principal: opaque string          # human or service principal acting
  realm:     Option<opaque string>  # None and Some("default") are different values
  authority: AuthorityRef           # what this principal may currently do; opaque id + digest
  executor:  Option<ExecutorRef>    # the agent revision acting for the principal, if any
}
```

The host constructs this from the admission profile. No application field, query parameter or independent caller-set header may set any of it (ADR 0026). It is passed to adapter code as part of the invocation context and to the policy, audit and cursor ports. It is never serialized into a public result or error message.

### 3.2 Descriptor additions

```json
{
  "version": "v1alpha2",
  "instance": "engineering-gitlab",
  "adapter": "connectors.gitlab",
  "revision": "…",
  "admission": { "profile": "identity-audience", "audience": "opaque deployment-owned bytes" },
  "operations": [
    {
      "id": "project.get",
      "description": "…",
      "contract": "operations/v1alpha1",
      "profile": "read",
      "effects": ["network"],
      "semantic_effects": [],
      "risk": "low",
      "idempotency": { "kind": "natural" },
      "approval": "not_required",
      "input_schema": {},
      "output_schema": {}
    }
  ],
  "configuration_schema": {}
}
```

`admission.profile` is what a caller needs to authenticate (`docs/design.md` § 6.2: "reveal only what a caller needs to authenticate/negotiate"). `admission.audience` is present only for `identity-audience`; it is opaque bytes the deployment chose, never a product or provider name (ADR 0021). The per-operation fields are the mutation profile's fields, now present on every operation so a read declares itself as one. Their values are defined in the [mutation profile](../../operations/v1alpha1/semantics.md) § 3 and are not redefined here.

Under `identity-audience` and `delegated`, `operations` is the caller-visible projection: enabled operations that the policy port admits for the verified context at describe time. A later `invoke` re-evaluates; describe-time visibility is not a grant.

### 3.3 Invocation additions

```json
{
  "version": "v1alpha2",
  "request_id": "…",
  "operation": "project.get",
  "revision": "…",
  "connection": "conn_… (optional; configured profile selects implicitly)",
  "input": {},
  "idempotency_key": "optional (mutation profile)",
  "approval": { "reference": "opaque", "evidence": "opaque, issuer-signed" },
  "executor": {
    "agent": "opaque id",
    "revision": 12,
    "authority_snapshot": { "id": "opaque", "sha256": "hex" }
  }
}
```

`executor` is an **assertion**. The receiver compares it with the executor binding recorded on the grant that admits this call; a mismatch or an unknown snapshot is `not_granted`. Absence is admitted only when policy admits the principal acting directly. Unknown fields are refused as today.

### 3.4 Response additions

```json
{ "version": "v1alpha2", "request_id": "…", "status": "success", "result": {}, "audit_ref": "opaque" }
{ "version": "v1alpha2", "request_id": "…", "status": "error", "error": { "code": "not_granted", "message": "…" }, "audit_ref": "opaque" }
```

`audit_ref` is present on every governed response, including refusals before dispatch, so a run's record can cite the row that names the refusal (ROADMAP O1: "a call outside it is a named refusal, not a missing row"). Under `static-bearer` it may be absent. Mutation outcome fields belong to the mutation profile.

### 3.5 Error codes added

| Code | Meaning | Dispatch |
|---|---|---|
| `not_granted` | policy refused this verified context for this operation and connection | none |
| `stale_authority` | the authority reference in the verified context no longer matches the current authority; the caller must obtain a fresh credential or authority snapshot and describe again | none |

`unauthorized` keeps its meaning (no or invalid credential). `forbidden` remains the adapter-level refusal (out-of-scope project, namespace, database) and is distinct from `not_granted`, which is host policy. The mutation profile's `approval_*`, `idempotency_conflict` and `outcome_unknown` are carried unchanged.

### 3.6 Delegated context (federation hop)

```text
DelegatedContext {
  tenant, principal, realm, authority, executor      # the gateway's VerifiedContext, verbatim
  gateway_instance: string                            # stable instance identity of the signer
  request_id:       string                            # the downstream request id it is bound to
  issued_unix_ms, expires_unix_ms
}
signature = HMAC-SHA256(route_key, canonical_json(DelegatedContext))
```

Carried on a single header the leaf's `delegated` profile reads; verified before any body decoding; bound to one request id; expires. The route key is receiver-owned configuration on both sides of one configured route (`docs/design.md` § 16: "route identifiers are receiver-owned configuration"). It is a transport adapter carrying verified facts internally, which ADR 0026 permits; it is not a caller-set header, because the leaf accepts it only from a configured route and only with a valid signature.

## 4. Rules

1. **Admission before decoding.** Transport → admission profile → `VerifiedContext` → strict envelope decode → operation and revision → connection within admitted scope → policy → approval (mutation profile) → dispatch. A failure at any step stops the sequence; the recorded step is the audit row's `stage`.
2. **No caller-set coordinates.** Tenant, principal, realm, authority and executor identity come from the admission profile only. A body `executor` is compared, never trusted.
3. **Fail closed.** Policy `Unavailable` → `unavailable`, no dispatch, for reads and mutations. There is no receiver-policy bypass when the policy store is down.
4. **Scope.** A `connection` outside the verified context's admitted scope (`auth.connection` § 4, "scope (tenant, principal)") is `forbidden` before policy evaluation; the response does not reveal whether the connection exists.
5. **Realm is a value, not a flag.** `None` and `Some("default")` are distinct in the policy key, cursor binding, idempotency scope, cache partition, audit row and delegated context. No component normalizes.
6. **Authority currency.** `stale_authority` is decided against the receiver's current view of the authority reference (digest match). It is decided before approval verification and before any provider call.
7. **Audit is value-free and always written.** One audit row per admitted-or-refused invocation: tenant, principal, realm, authority, executor, operation, connection, revision, input digest, stage, outcome class, timestamps. No secret, no provider payload, no error message text from the provider. For the mutation profile the attempt ledger row references the audit row; the audit row does not replace the ledger.
8. **Projection is not a grant.** Describe-time visibility uses the same policy port but records no grant reference; invoke re-evaluates and records `grant_ref`.
9. **Federation.** The gateway admits the caller under its own profile, forwards under `delegated` with the caller's verified context, and the leaf evaluates its own policy. The gateway never substitutes its identity for the caller (`docs/design.md` § 16 item 3). Depth stays 1. The gateway returns the leaf's `audit_ref` alongside its own.
10. **One wire version per client.** A `v1alpha1` invocation to a v1alpha2 service is `unsupported` unless the instance explicitly enables the `v1alpha1` projection, which is permitted only under `static-bearer` and hides every operation whose `approval` is not `not_required` or whose `effects` contain `external_write`. A v1alpha2 invocation to a v1alpha1 service is refused by the current strict decoder (`crates/connectors-core/src/lib.rs`, `deny_unknown_fields`) as `invalid_input`; the client reports it as `unsupported`. No fallback, no resend.

## 5. Ordering and limits

| Bound | First-profile default | Note |
|---|---|---|
| Policy evaluation deadline | 2 s | exceeding it is `unavailable`; counted inside the 20 s service deadline of v1alpha1 |
| Delegated context validity | 30 s from `issued_unix_ms` | one request id; a reused context is `unauthorized` at the leaf |
| Identity token verification | as the pinned `identity-client` defines; no local override | five-minute one-use access tokens per ROADMAP O1 row |
| Audit write | before the response is sent; for mutations, before the dispatch gate opens (ties to the attempt ledger) | an audit write failure on a read is `unavailable`, no dispatch |
| Descriptor projection size | unchanged 4 MiB response bound | projection can only remove operations |

Ordering: the audit row for a refusal is written before the refusal is returned. The audit row for a success is written before the result is returned. There is no ordering guarantee between audit rows of concurrent requests beyond timestamps.

## 6. Conformance scenarios

- Missing or invalid credential under every profile → `unauthorized`, zero provider requests, one audit row with `stage: admission`.
- Valid `identity-audience` token for a different audience → `unauthorized`; the message does not name the expected audience.
- Body carries a `tenant` or `realm` field → `invalid_input` (unknown field) before policy; no audit row names a tenant from the body.
- `executor` assertion mismatching the grant's executor binding → `not_granted`; provider fixture sees zero requests.
- Policy port returns `Unavailable` on a read → `unavailable`; zero provider requests; audit `stage: policy`.
- Authority digest changed between describe and invoke → `stale_authority` before approval and dispatch.
- Two contexts differing only in realm `None` vs `Some("default")` → different cursor validity, different idempotency scope, two audit rows with different realm values, zero normalization.
- Describe under two verified contexts with different policy → two different `operations` lists from one instance; invoking a hidden operation → `not_granted`, not `not_found`.
- Federation: caller admitted at the gateway; leaf receives a valid delegated context → leaf audit row names the caller's principal, not the gateway; leaf policy refusal → `not_granted` surfaced unchanged with both `audit_ref`s.
- Delegated context replayed with a second request id, or after expiry, or from an unconfigured route → `unauthorized` at the leaf; zero dispatch.
- `v1alpha1` client against a v1alpha2 `identity-audience` instance → `unsupported`; against a `static-bearer` instance with projection enabled → mutation operations absent from the descriptor and `not_found` on invoke.
- Every response and refusal under `identity-audience` carries a non-empty `audit_ref`; grepping the audit sink for any credential byte, provider payload token or provider error text finds nothing.

## 7. Compatibility and projection

| From | To | Disposition |
|---|---|---|
| v1alpha1 wire (`static-bearer`) | v1alpha2 | additive on the descriptor and invocation; the strict readers make it a version bump, as the mutation profile § 7 already decided |
| v1alpha1 clients | v1alpha2 `static-bearer` instance | optional projection hides governed operations; no other loss |
| v1alpha1 clients | v1alpha2 `identity-audience`/`delegated` instance | refused; there is no way to carry a verified context to a client that cannot present one |
| Old `OwnerContext` | v1alpha2 | `tenant_id` → verified context; `agent_id`, `agent_revision`, `authority_snapshot_*` → `executor` assertion |
| Old `description_ref` | v1alpha2 | descriptor `revision` |
| Old `connector_audit_ref` / `execution_ref` | v1alpha2 | `audit_ref`; execution reference belongs to the mutation profile's attempt id |
| Old `DatasourceErrorCode::{NotGranted, StaleAuthority}` | v1alpha2 | `not_granted`, `stale_authority` |
| Old `OperationDescription.effect: ReadOnly/Mutating/Destructive`, `approval: NotRequired/Required` | v1alpha2 | `effects` (+ `risk`, `semantic_effects`) and `approval`; a compatibility facade maps `Destructive` to `effects ∋ external_write` with `semantic_effects ∋ irreversible` and is a separate tested binding (`docs/design.md` § 23.2) |
| Old `--protocol-version v2|v3` | v1alpha2 | one wire version per client build; the flag has no successor |

The compatibility matrix across every proposed family is owned by `story:contracts-wire-compatibility`; this document supplies its rows for the service envelope.

## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| `Admission` port: `verify(headers, route) -> VerifiedContext`; three implementations: static bearer (today's `Credential`), Identity audience over the pinned `identity-client` behind a host feature `identity`, delegated HMAC | `crates/connectors-host/src/server.rs:57-70` (today: static only) |
| `Policy` port: `admit(&VerifiedContext, operation, connection, input_digest) -> Admitted { grant_ref } | Refused { code } | Unavailable`; local allow-all binding for `static-bearer`; a file-backed grant binding for tests; the hosted binding is a later story | new host module |
| `Audit` port: append-only, value-free rows; JSONL file binding first, durable binding later; same conformance tests | new host module |
| `SecretStore` binding over the released `secrets-client` with a workload identity; connector-created values non-revealable; behind a host feature `secrets` | `crates/connectors-sdk/src/lib.rs:42-44`; ADR 0023 |
| `Adapter::invoke` receives an `InvocationContext { connection, verified: &VerifiedContext }` instead of ambient state; aligns with `auth.connection` § 8 | `crates/connectors-sdk/src/lib.rs:18-31` |
| Core types: `Descriptor.admission`, per-operation curation fields, `Invocation.{connection, executor}`, `Response.audit_ref`, two error codes; versioned codec so v1alpha1 readers stay byte-identical | `crates/connectors-core/src/lib.rs` |
| Client: `Client::with_identity_token(token)`, `Client::with_executor(assertion)`; treat a missing `audit_ref` under a governed profile as a protocol error | `crates/connectors-client/src/lib.rs` |
| Federation: sign and forward the delegated context per route; verify at the leaf; return both audit references | `crates/connectors-host/src/federation.rs` |
| Adapter spec kind: the descriptor curation fields are declared per operation and refused when absent under v1alpha2 | `spec-kinds/adapter/v2/schema.json`, `crates/connectors-spec` |
| Generated adapters from service-sdk: a `ConnectorServiceFactoryDescriptor` lowers to an adapter/v2 instance plus a generated `Adapter` implementation; the generated service's realm policy (Required/Optional/Forbidden, ADR 0026) is enforced by the host's admission before the generated handler runs | `../service-sdk/crates/service-connectors` (today targets the old `ConnectorBackend`); ADR 0027, 0029 |
| CLI: `--token-file` stays for `static-bearer`; an Identity-token source and executor assertion for governed calls; verb names are not decided here | `apps/connectors/src/main.rs` |

## 9. ESS entities

| Entity / value | Notes |
|---|---|
| `VerifiedContext` | a value carried by the host, not an entity; no lifecycle |
| `AuditRecord` | UNMAPPED: identity (host-generated id), fields as § 4 rule 7; relation to `AttemptRecord` (`ess/domains/mutations.yaml`) is one-to-zero-or-one; model before the audit story is decomposed |
| `GrantRecord` | UNMAPPED: the hosted binding's shape depends on the policy store; the local file binding is not an entity claim |
| Tenant / principal ownership of a `Connection` | remains UNMAPPED as in `auth.connection` § 9; this document adds the **rule** (scope decided from the verified context) without asserting the relation's cardinality |
| `OperationDeclaration.{effects, risk, approval, …}` | UNMAPPED additions already listed by the mutation profile § 9 |

## 10. Open decisions

| Decision | Default taken | Evidence needed before freezing |
|---|---|---|
| Where the executor assertion travels | request body, as today's consumers already send it | Agent Platform and Devcenter send it through the new client without a second seam |
| Delegated context signature | HMAC-SHA256 with a per-route configured key | one gateway, two leaves, key rotation without dropped requests; JWS only if a third party must verify |
| `v1alpha1` projection availability | `static-bearer` only, off by default | no governed instance ever serves an ungoverned reader |
| First policy binding | file-backed grants for tests, allow-all for local; hosted binding in a later story once the grant record is modeled | the same port tests pass on both |
| First audit binding | append-only JSONL under the service's state directory | crash between audit write and response leaves a row, never a response without a row |
| Describe without a credential | not admitted; anonymous cases belong to `story:contracts-anonymous-auth` | that story's disposition |
| Audience string ownership | Connectors deployment data, opaque to Identity, never in source | ADR 0021 vectors on both sides |
