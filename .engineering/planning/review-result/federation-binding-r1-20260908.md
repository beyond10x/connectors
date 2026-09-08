---
format: aep.planning-md/1
id: review-result:federation-binding-r1-20260908
kind: review-result
status: active
title: Independent F03 binding review round 1
relations:
- reviews: story:contracts-federated-approval
revision: 1
---
# Independent F03 foundation review B

Baseline: clean Connectors v2 main `b5ace3f29d6deb1dbc9e5ddd50a8a8b02152cb2f`. This reviews the existing unbound federation placeholder and the sources that should govern its replacement. It is not a review of root's later semantic draft. Frozen sources and repository heads are in `source-hashes.json` and `snapshot/`. No tracked/planning/runtime files changed; no other reviewer output read, no agents delegated, no service calls or private key material accessed.

Verdict: **F03 needs the concrete subject and protocol requirements below before it can leave the unbound state**. E02's existing refusal/unadvertisement posture is correct. A new Connectors-specific successor to the already used signed-request discipline is preferable to treating the withdrawn HMAC sketch as an established protocol.

## Existing contracts precede new protocol design

Current authoritative Atlas is the clean supplied checkout at `38033fb4557e3b01c85379be95530f9f5e15e6ca`. ADR 0026 requires authentication-owned tenant/principal/current authority/optional executor/optional realm, no body/route/independent-header override, and exact distinction between absent realm and default. ADR 0021 makes audience/scope bytes opaque deployment data and reserves operation-level authorization to the relying party. ADR 0031 treats the product exchange, publication confinement, Connector grant/description checks and human approval as distinct boundaries; its exchange is not a generic gateway impersonation credential. These are current architectural requirements, not proof that a federation verifier exists.

There **is** an existing signed service-request contract in the predecessor platform, despite no literal signed-dispatch symbol:

- Old Connectors `docs/design/11-hosted-module-request-authority.md` and `crates/integration-platform/src/module_signing.rs`, at old HEAD `81459ac42ddd518d3942f4b079841e9e0ed6efc8`, sign compact Ed25519 JWS with protected `alg: EdDSA`, `kid`, `typ: b10x.module-request.v1+jws`, carried as `Authorization: DLModule ...`.
- The exact closed schemas live in predecessor Platform `model/modules/contracts/module/v0alpha1/schemas/module-request-{protected,claims}.schema.json`, at `b04014154f6ee0cf7c0231f0dee5d85dbee03c24`. They bind issuer/audience, tenant, initiating sub and immediate act, module operation, exact method and target path/query, raw body SHA-256, nullable idempotency digest, authority snapshot, request/trace correlation, grants, iat/nbf/exp and jti. Typ and algorithm are fixed; receiver key selection is from configured public keys.
- Platform `model/modules/sdk/module-http/src/lib.rs` verifies signature, exact receiver/request/binding/time and permitted operation, then awaits durable nonce redemption before constructing context. Its `signed_admission_binds_identity_request_grant_and_single_use` test asserts original admission, repeated-token refusal and altered-body refusal. I inspected the test; I did not execute it.
- Its predecessor **Platform ADR 0041** is historical evidence in a different architecture namespace. It must not be cited as current Atlas ADR 0041 or used to impose old Work/Ontology ingress/health/deployment rules on this repository.

The old claim set has **no realm, full executor assertion, leaf connection/metadata revision, descriptor/presentation subject or approval binding**. Current old Connectors source and cached origin/main `eb0b45140b9658135577a7f1ee9a21facc54ec29` both lack realm in ModuleRequestClaims. Both the owner schema and receiver are closed. Adding those values under the frozen module-request v1 type would break its reader. Existing module audiences, grants and DLModule authority cannot simply be reused for Connectors federation.

Preserve the tested discipline and primitive via a **distinct closed Connectors delegation type/schema and explicit admission profile**, with independently configured issuer/audience/key trust. This is a justified new binding, not invention of a wholly unrelated signing design. The current service-sdk service-connectors bridge converts already verified PrincipalContext into SDK identity; it is not a signed federation verifier or a substitute for leaf authentication.

One old implementation boundary should be characterized rather than copied: module-http allows an expiry plus five-second clock skew, but passes plain exp to ReplayStore; module-eventlog rejects expires_at <= now. Thus effective acceptance differs across replay-store implementations during that skew tail. The new binding must use one consistent acceptance interval and retention horizon. Old replay identifiers are also implementation-specific; F03 needs an injective structured tuple rather than inheriting ad hoc string layout.

## Concrete F03 requirements

### FB-01 — P1: one canonical approval subject, independently visible before approval

An alias such as gateway gitlab__issue.create and a gateway descriptor revision are not the leaf's issue.create and leaf revision. Choose one exact, versioned canonical subject that both the issuer and leaf can verify. It must bind trusted caller identity (tenant/realm/principal/executor as applicable), trusted origin, receiver instance, source-owned operation/family/profile, resolved connection and effect-relevant connection/configuration revisions, canonical input digest and canonicalization identifier, plus the applicable operation/descriptor revision. Approval issuer, reference, expiry and approval mode remain explicit. Public source-qualified names and safe references may be disclosed only under current caller/result admission.

A host-owned **approval preparation read** can expose this subject for an implicitly selected or managed connection. It must have a closed admitted payload and safe result, no provider dispatch, no secret entry, no approval spend and no attempt reservation. A returned subject is information for approval, not execution authority. The issuer must approve exactly those values; the eventual receiver recomputes and compares them. A tampered caller copy cannot become canonical because it arrived through the gateway.

### FB-02 — P1: distinguish presentation freshness from signed effect identity

Record gateway-visible alias/projection revision separately from leaf operation/connection/descriptor/configuration coordinates. For each revision, state whether it is part of signed approval identity or a checked routing freshness condition. A fresh gateway mapping cannot rewrite a signed approval, rebind a connection or redirect an attempt to another receiver. Alias/configuration changes are rechecked before forwarding, and leaf semantic changes are rechecked before approval spending. A route becoming stale yields explicit refusal/current descriptor retrieval, never automatic resend or signature repair. If unchanged leaf approval can survive an unrelated gateway-only refresh, say so precisely and retain the current projection check; otherwise refuse and require new approval. Do not leave this to an implementation inference.

### FB-03 — P1: authenticated gateway facts are scoped trust, not cryptographic proof of arbitrary callers

Leaf receiver configuration fixes the permitted gateway issuer/key set, exact receiver/audience, tenant/realm/subject classes, route scope and allowed authority presentation. A signature establishes that this configured issuer asserted a context; it does not independently prove the issuer honestly authenticated the human. State that trust explicitly. The leaf must still check current authority and its own narrowing policy, operation/connection access and result disclosure. Effective permission is the intersection, never the gateway's broad service identity substituted for the caller. A revoked/missing/unavailable authority or policy fails closed. Authentication, supplied executor assertion and admitted executor binding remain distinct; signing caller-written coordinates does not validate them.

### FB-04 — P1: bind exact receiver and actual request bytes, including bodyless describe

The new assertion must bind receiver instance/audience, method and exact selected route/target, the raw bounded body digest, selected operation/connection/revisions and the full immutable approved subject or its injective canonical digest. The raw digest covers approval, key, executor and all envelope members actually sent; no middleware may reserialize or decompress those bytes after the signature boundary without an explicitly specified verified transformation.

Separate outer cryptographic checks from semantic checks: bound headers/raw bytes and verify the signed authentication envelope before application decoding; then strictly decode the invocation and compare body correlation and resolved semantic coordinates before policy/redemption/dispatch. Request ID alone is not content binding. Duplicate headers, duplicate JSON members, unknown fields, unsupported typ/alg, unknown key, alternate base64 forms, compressed/body transformations and route/query ambiguities need explicit refusal rules and byte vectors.

GET describe has no invocation envelope. Give it a fresh private signed correlation/nonce and an exact empty-body/method/path binding, with its own purpose or operation discriminator. Its public E02 response request_id remains **null**. An invoke assertion cannot authorize describe or vice versa. Describe remains caller-scoped policy projection and creates no business approval or execution grant.

### FB-05 — P1: nonce redemption and approval spending have different owners and meanings

The selected leaf owns atomic durable **transport nonce** consumption for its delegation receiver. A stable key such as (issuer, receiver, jti) must not change on signing-key rotation; kid and credential bytes are not new authority namespaces. Scope and injective encoding must be explicit, with one uniqueness authority across all admitted replicas. Missing, unavailable or ambiguously acknowledged storage cannot admit another use. Capacity refuses new authority rather than evicting live nonces. A duplicate nonce is authentication refusal, not a cached successful invocation.

Only the execution leaf owns **approval verification/spending** and the mutation attempt/key ledger. The gateway may precheck, prepare and forward the unchanged evidence, but never spends it. Consuming a nonce proves no approval spend or provider effect. A deliberate caller retry receives a new transport nonce and may observe the original keyed attempt under current admission without spending approval again. It does not re-dispatch. A captured identical signed transport request is refused even when its contained idempotency key would otherwise identify a replay.

### FB-06 — P1: time, key rotation and replay retention must describe one acceptance window

Choose exact integer units, accepted bounds, iat/nbf/exp relation, maximum life and bounded clock uncertainty; use checked arithmetic and define equality at expiry. Receivers unable to establish the bound fail closed. Nonce retention covers every instant at which any still-admitted key could validate the assertion, including allowed skew. A wall-clock jump, restart, changed key ID or process replacement cannot shorten retention or make old authority new. Signing-key overlap is bounded receiver configuration, private/public key custody is separate, and key revocation wins current admission. No provider/Identity/service bearer is reused as signing material. Delegation lifetime never extends approval expiry, authority lifetime or the remaining execution budget.

### FB-07 — P1: preserve the settled leaf mutation fence through every failure window

Trace at least: gateway admission/preparation -> leaf authentication and nonce acceptance -> leaf current policy/result admission -> keyed lookup/replay decision -> verify new approval/preflight -> acknowledged audit admission and durable Prepared attempt -> leaf spend -> durable open gate -> at most one provider send -> truthful terminal observation. Exact ordering of early audit and transport redemption may differ if specified, but neither may grant a provider send. Keep winner-recheck races from F02, and do not require original approval still live for an admitted exact replay.

Nonce consumed then policy refused: no provider effect and no approval spending. Approval spent then pre-gate abort: approval stays spent and effect is not_attempted. Gate acknowledgement uncertain or gateway loses the downstream response: effect unknown unless a valid definitive original observation is available. Recovery may fence Prepared but cannot infer dispatch from spend status or reacquire permission from Dispatching. No gateway deadline, token refresh, route refresh or new signature permits automatic resend.

### FB-08 — P1: failure correlation and disclosure must preserve E02 semantics

The gateway returns its own public request correlation and audit facts, preserves original attempt/request identity and exact effect classification from an authenticated valid leaf response, and retains separate source_audit provenance. A leaf pre-envelope failure is not a fabricated business attempt. If a forwarded mutation may have reached the leaf and its reply is absent/invalid, the gateway cannot manufacture not_attempted from its local send failure, nonce state or generic timeout. Current caller/result denial omits original mutation metadata, including replay existence, as E02 already requires. No path or diagnostic discloses private signing keys, credential material or hidden leaf connection inventory.

## Minimal end-to-end textual trace

1. Caller authenticates and describes the gateway; sees only admitted operation aliases and safe curation.
2. Admitted preparation read resolves alias/selected connection to the exact canonical receiver subject under the current projection. It returns that safe immutable value without spending or dispatching.
3. Issuer approves the exact subject for the trusted caller and origin. Caller presents the reference/evidence unchanged with its operation input.
4. Gateway rechecks its selected projection, caller scope, subject mapping and authority; creates one request-bound signed leaf assertion with a fresh transport nonce. It does not redeem approval.
5. Leaf verifies its configured issuer/receiver/body/time/context binding, atomically consumes the nonce, rechecks current narrowing policy and resolves the identical canonical subject. Any mismatch refuses without spending or business dispatch.
6. Leaf observes an existing admitted key, or creates the one new Prepared attempt, spends approval exactly once, and opens the durable dispatch gate. Only the definite winner can send once.
7. Leaf and gateway preserve E02 known/refused/not_attempted/unknown and original/replayed identity semantics. A lost answer never triggers a second send.

Repeat the trace with a different caller/realm/origin/receiver/key ID/operation/input/connection/revision, altered raw bytes, GET purpose substitution, clock boundary, nonce replay and shared-store concurrent redemption. Each variation has an explicit expected refusal/uncertainty and a named zero/one spend owner. Add approval-spend and nonce-acknowledgement failure cases independently; conflating them is precisely the defect F03 must prevent.

## Limits of this foundation review

The verified current Atlas ADRs provide architectural authority; predecessor platform schemas and source provide pinned characterization and a design precedent. Neither proves new federation implementation. New typed subject/assertion/redemption ownership must use the ESS workflow before later entity-bearing implementation decomposition; strings and a valid signature are not substitute entity relations. F03 may define local semantic types and conformance vectors now, but no old runtime, service-sdk bridge, static read federation or E02 type model makes the new profile callable.
