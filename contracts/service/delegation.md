# One-hop delegated approval and dispatch

**Proposed specification, not implemented.** Owner: `story:contracts-federated-approval` (F03). This completes the semantic selection left open by [service compatibility](compatibility.md) and [service v1alpha2](v1alpha2/semantics.md); it does not make a runtime advertise delegated support. It covers a client, one gateway and one executing leaf, including implicit configured and explicit managed connections. Multi-hop forwarding remains unsupported.

## 1. Evidence and distinct authorities

Design §16 requires source-owned identity, admitted routes, both gateway and leaf policy, and no resend or rerouting after uncertainty. Design §23.3 requires one active mutating route and one one-time approval ledger during cutover. The implemented v1alpha1 gateway rewrites operation aliases/revisions under a configured downstream token; it establishes none of the originating caller authority required here.

Current Atlas ADRs 0021, 0026 and 0031, reviewed at `38033fb4557e3b01c85379be95530f9f5e15e6ca`, require receiver-owned audience/scope, authentication-owned optional realm and current authority, and separation of access exchange from Connector admission and approval. Predecessor Connectors `81459ac42ddd518d3942f4b079841e9e0ed6efc8` has an existing signed module-request design in `docs/design/11-hosted-module-request-authority.md` and `crates/integration-platform/src/module_signing.rs`; its closed owner schemas/receiver are in predecessor Platform `b04014154f6ee0cf7c0231f0dee5d85dbee03c24`, `model/modules/contracts/module/v0alpha1/` and `model/modules/sdk/module-http/`. Its historical Platform ADR 0041 is not current Atlas ADR 0041.

Preserve that predecessor's separate signing key, exact audience/request binding and durable one-use delivery discipline. Its `b10x.module-request.v1+jws` / `DLModule` protocol lacks realm, executor and canonical approval subjects; its bytes and module audiences are not extended or reused here. Its expiry-skew/replay-store mismatch is not preserved. Old hosted approval issuance/storage is authoritative evidence, not caller-made records accepted by field equality; the old stored record did not bind descriptor revision and is not the new proof format.

| Authority | Responsibility |
|---|---|
| Client's authenticated principal and optional executor | Initiating identity; application input cannot manufacture either. Absent executor/realm/tenant is explicit null, never an empty/default identity. |
| Configured gateway issuer | Attests the already admitted originating context and exact forwarded request, within receiver-configured projection bounds. A signature proves who attested, not that an arbitrary issuer honestly authenticated a person. |
| Executing leaf policy | Independently checks current authority, realm policy, operation/connection/resource/result access and allowed gateway projection. Effective access is the intersection. A gateway token or approval never substitutes for this check. |
| Configured approval issuer | Approves one exact canonical subject for a bounded validity interval. It is distinct from gateway transport authentication and current execution grants. |
| Executing logical leaf | Sole approval verifier/redemption coordinator, business attempt owner and idempotency owner. Its replicas share the same durable uniqueness/fencing authorities. The gateway owns only its own admission/audit and forwarding decisions. |

## 2. Canonical subject and revisions

`CanonicalApprovalSubject` is a closed value:

```json
{
  "format": "connectors.approval-subject/v1",
  "target": {
    "instance": "leaf-prod",
    "operation": "issue.create",
    "connection": "conn-team",
    "connection_revision": "connection-r7",
    "contract": "operations/v1alpha1",
    "profile": "mutation",
    "descriptor_revision": "leaf-d9",
    "configuration_revision": "leaf-config4"
  },
  "authority": {
    "scope": { "tenant": "tenant-a", "realm": null, "caller": "principal-a", "executor": null },
    "current_authority": { "id": "authority-a", "sha256": "<64 lowercase hex>" },
    "executor": null
  },
  "origin": { "kind": "federated", "authority_ref": "gateway-prod" },
  "route": { "gateway_instance": "gateway-prod", "route_id": "prod", "route_revision": "route-r3" },
  "canonicalization": "adapter-v1-canonical-json",
  "input_sha256": "<64 lowercase hex>",
  "approval_mode": "required"
}
```

All members are required. `route` is null for direct origin; direct origin authority_ref equals the executing instance. For federated origin it is non-null, gateway_instance equals origin.authority_ref and the authenticated delegation issuer, and exactly one route is represented. IDs are opaque, nonempty, case-sensitive UTF-8 strings, at most 256 bytes without control characters; do not normalize, concatenate with delimiters, or derive them from URLs/credential bytes. Digests are 64 lowercase hexadecimal characters. Canonical subjects are at most 8 KiB of canonical UTF-8 JSON. These bounds also apply to the corresponding assertion coordinates.

`authority.scope` uses the trusted stable caller and optional stable executor identities in the leaf's admitted identity namespace. The configured issuer-to-identity mapping must be injective and agreed by gateway and leaf; two authentication issuers' similarly spelled subjects cannot silently collide. If a deployment cannot preserve the same qualified identity, it refuses delegation. `current_authority` is the verified current authority snapshot or null for an explicitly configured local policy. `authority.executor` is null with scope.executor, or `{agent,revision,authority_snapshot:{id,sha256}}`; agent equals scope.executor. The leaf verifies that executor binding against current policy; its snapshot is not automatically the principal's current_authority snapshot. Scope identity stays stable across authority/key refresh, while approval binds these precise admitted authority values. No synthetic executor fills the no-executor case.

The leaf reconstructs the entire subject from verified context, its resolved operation/connection/configuration and validated input. The input digest is the existing sorted-key canonical JSON rule, not a digest of the whole transport. The whole subject is also serialized with that rule when a digest is needed, tagged by its format. Compare complete decoded coordinates and exact canonical bytes, not an unqualified name or only a loosely specified digest. No provider secret, credential reference/generation, URL, custody handle or private callback is in the public subject.

| Coordinate | Role and change behavior |
|---|---|
| Leaf operation/connection, family/profile, descriptor/configuration/connection revisions | Signed execution identity. Any change requires a new subject and approval for a new attempt, even if an alias still looks the same. Same-identity credential rotation alone changes none of them. |
| Stable gateway instance and route id/revision | Signed origin/binding identity. Route revision changes on receiver, destination/trust, operation/connection mapping or authority-boundary changes. Repointing never preserves an old subject. Receiver configuration independently admits that issuer/route to its own target; the gateway cannot select a different endpoint from a token or input. |
| Gateway public alias and full descriptor/projection revision | Presentation/freshness only, recorded separately in preparation. The current supplied gateway revision is always checked before routing. An unrelated gateway-only description refresh may retain a still-identical canonical subject; after fresh describe and deliberate resubmission, its approval can still be used. No automatic retry, rewriting or signature repair occurs. |
| Request ids, nonce, deadline, signing kid, credential bytes | Delivery/correlation only, never business identity or a fresh idempotency namespace. |

For F02, the executing leaf's fingerprint uses its own target revisions and a required nullable `route` coordinate with the above three route fields. The stable origin namespace is gateway identity, not route/kid/token bytes. A changed route revision with the same live key therefore conflicts, rather than becoming an independent namespace permitting another effect. Direct versus federated origin and different gateways remain distinct namespaces and approval subjects. An exact admitted existing-key observation still precedes approval verification; refreshing current authority does not require the original approval to become usable again.

## 3. Safe approval preparation read

The proposed extended descriptor may expose the host-owned operation **`host.approval.prepare`**, contract `operations/v1alpha1`, profile **`approval-subject`**. Its mandatory curation is read-only (`effects` is [] at a leaf or [network] at a forwarding host, semantic_effects [], risk low, idempotency natural, approval not_required). This reserved host id cannot collide with an adapter operation; collision refuses service configuration. It is never projected onto legacy v1alpha1. This introduces a selected operation payload, not another envelope member or a provider operation.

The ordinary outer Invocation supplies the caller's current descriptor revision, optional target connection selector and executor assertion. Its business input is the closed object `{operation, input}`: operation names the intended mutation in that same descriptor, input is that mutation's prospective business input. The outer connection selects the intended target connection under this helper profile; it grants neither general management access nor permission to read another connection. Unknown helper inputs, an unimplemented/non-mutation target, unsupported connection mapping or a stale description refuse. No nested tenant/realm/authority field is accepted as authority.

Under current disclosure/preparation admission at **both** hops, resolve either the configured implicit connection or the caller-selected managed connection to the leaf's safe stable ref and semantic revisions. Validate target input and executor binding; read admitted metadata only. Do not resolve/read provider credential material, refresh/acquire credentials, perform provider verification/network I/O, create a business AttemptRecord, reserve a business key or spend approval. Gateway-to-leaf control traffic, ordinary admission audit and transport nonce consumption are permitted and are not business approval/reservation. Inability to establish exact metadata fails closed; a guessed subject is not useful preparation.

Success is the common audited Response whose result is the closed value:

```text
{
  subject: CanonicalApprovalSubject,
  subject_sha256: canonical subject digest,
  presentation: {instance, operation, connection: nullable safe selector, descriptor_revision},
  approval_policy: {mode, issuer: nullable string, audience: nullable string, max_lifetime_seconds: nullable integer}
}
```

The leaf constructs subject and approval_policy. The gateway preserves them and supplies only its checked presentation coordinates; it verifies response correlation, selected receiver, operation, canonical input and route identity. For direct use, presentation is the leaf. A required-approval policy names the exact configured issuer and leaf approval audience and a 300-second maximum; not_required uses null issuer/audience/lifetime. `event_claim` retains the same canonical/leaf ownership rules, but its event-to-claim admission/proof binding remains unavailable until the events owner specifies it; this helper must not advertise support for that unbound mode.

The client/approval issuer must display and approve the **canonical leaf subject**, not just the friendly alias. The issuer obtains it through its own admitted preparation or another explicitly trusted authenticated delivery of that result, rechecks its own issuance policy and the human/subject binding, and never trusts a caller-edited presentation label as operation evidence. Issuance and approval UI are outside this helper. A preparation value is not signed execution authority, a lease, reservation or promise that invoke will be admitted; stale connection/authority/route/leaf revisions cause the eventual leaf reconstruction to differ and refuse a new attempt.

Helper limits are explicit: **272 KiB request, 64 KiB result, 20 s total, 15 s downstream-control ceiling, 5 s connect**. The request includes the helper wrapper and room for a target invocation bounded at 256 KiB; validate that target's own projected invocation size too. The eventual invoke must independently fit its own limit including the issued approval proof; preparation does not reserve ingress capacity or guarantee a later frame will fit. No provider call is authorized by these ceilings. These dedicated helper limits must be advertised in Operation.limits and supported by bounded ingress; they are not ordinary/generic execution defaults. A smaller host refuses the profile. Control calls and policy/audit share the total budget; no hop gets a fresh total.

## 4. Cryptographic framing and proof separation

Both new proofs use compact JWS with protected header exactly `{alg,kid,typ}`, canonical unpadded base64url segments and no unprotected header, detached payload, compression, `b64`/critical extensions, key URLs or embedded keys. Header JSON is at most 512 bytes, claims JSON at most 12 KiB, complete token at most 18 KiB, signature exactly 64 bytes; every JSON object is closed and duplicate members are rejected recursively. Both issuers serialize header/claims as canonical UTF-8 JSON; receivers verify the original encoded signing input and reject noncanonical encodings. Untrusted decoded iss/kid are bounded local key-lookup hints only. Verification selects an already configured public key for the expected issuer/type/audience and fixed algorithm; it never fetches a key or chooses trust from the token.

The new algorithm identifier is **`Ed25519`**, using the fully specified JOSE algorithm from [RFC 9864 §2.2](https://www.rfc-editor.org/rfc/rfc9864.html#section-2.2), with compact framing from [RFC 7515 §3.1](https://www.rfc-editor.org/rfc/rfc7515.html#section-3.1) and Ed25519 key/signature representation from [RFC 8037](https://www.rfc-editor.org/rfc/rfc8037.html). Do not silently accept the predecessor's polymorphic EdDSA identifier, another curve, `none` or a symmetric algorithm. These are standard cryptographic operations with a new application claim contract, not an inherited module authorization protocol. Sources checked 2026-09-08; Connectors had no admitted public-standard fetch operation, so the RFCs were read directly.

Signing material is purpose-specific and deployment-owned; never reuse Identity/session/provider/service-bearer credentials. Receiver configuration fixes stable issuer and leaf identity, exact opaque audience, allowed tenant/realm/principal/executor classes, operation/connection/route scope, key id/public key/type, key validity/revocation and the realm policy. Private keys and claim-bearing Authorization values never enter descriptors, logs or audit. Rotation may overlap explicitly configured keys for bounded intervals; revoked keys fail current verification. Kid/key bytes never change stable caller/origin or uniqueness namespaces. Key overlap does not reset any nonce or approval state.

### 4.1 Gateway delivery assertion

Use **`Authorization: ConnectorDelegation <JWS>`**, with typ **`b10x.connectors-delegation.v1+jws`**, only on the selected extended routes. Exactly one Authorization field is allowed; no parallel bearer fallback. HTTPS with verified configured receiver identity is mandatory for this binding; discovered addresses, redirects and caller-provided destinations cannot replace it. Proxies may not rewrite/decompress/re-encode signed request bytes or target; reject Content-Encoding, a query string, an unexpected route/method or a GET body. Normal HTTP transfer framing is removed before hashing the unchanged entity bytes. Ambiguous duplicate length/transfer framing is refused by the transport binding.

The closed claims are:

```text
{
  iss, aud, receiver,
  purpose: describe | prepare | invoke,
  method: GET | POST,
  target: /v1alpha2/describe | /v1alpha2/invoke,
  body_sha256,
  authority: AdmittedAuthority,
  route: RouteBinding,
  origin_request_id: nullable string,
  request_id: nullable string,
  operation: nullable string,
  connection: nullable string,
  descriptor_revision: nullable string,
  execution_deadline_unix_ms,
  iat, nbf, exp,
  jti
}
```

All members are required. iss = route.gateway_instance, receiver is the configured leaf instance and aud its exact delegation audience. jti is 32 cryptographically random bytes encoded as 64 lowercase hex, never reused by that stable issuer. Timestamps are integer Unix seconds, except the explicitly millisecond deadline. Each must be a JSON integer in [0, 9007199254740991], never a boolean, fraction or string; all arithmetic and unit conversion is checked against that bound. Claims carry admitted authority, not caller assertions signed without verification. Connection is the forwarded safe selector or null for implicit selection; the leaf resolves it to its actual canonical connection. The raw body digest covers every forwarded envelope byte, including approval evidence, idempotency key and executor assertion, without treating those values as transport authority.

| Purpose | Exact request binding |
|---|---|
| describe | GET /v1alpha2/describe, empty-body SHA-256; origin_request_id, request_id, operation, connection and descriptor_revision all null. A fresh jti supplies private hop correlation; the public Response request_id stays null. Current caller-scoped describe policy still applies. |
| prepare | POST /v1alpha2/invoke; operation = host.approval.prepare; request_id/revision/connection equal the strict outgoing Invocation. Its input names the mapped leaf target mutation. origin_request_id is the initiating gateway helper request id. |
| invoke | POST /v1alpha2/invoke; operation is the actual mapped leaf business operation, never host.approval.prepare; request_id/revision/connection equal that Invocation. origin_request_id is the initiating gateway invoke id. A fresh host-generated leaf request id is correlation, not a new business key. |

The receiver first bounds the transport/header/raw body, verifies signature, fixed type/algorithm, receiver/audience, exact raw request and time, and admits the configured issuer projection/realm policy. It then consumes the transport nonce durably. After definite acknowledgement, immediately before granting authenticated entry, it rechecks the complete §5 time predicates, current configured key validity/revocation and issuer projection/realm admission, and remaining signed deadline. This final entry decision is the admission serialization point; it must use current trusted observations, not the pre-await snapshot. Failure gives no authenticated entry and leaves the acknowledged nonce consumed. No rollback or transport retry follows. It then constructs authenticated context and strictly decodes the selected invocation and compares its correlation/operation/revision/connection/executor with the authenticated claims, then performs current leaf policy and resolved subject checks. A body executor must match the gateway-admitted executor and independently satisfy leaf policy; body authority never wins. Describe decodes no Invocation. Another delegated ingress cannot be forwarded again: the gateway accepts this profile only as a leaf, not as authority for a second outgoing hop.

After signature, exact receiver/request and fixed-shape jti have been verified, responses echo **`Connector-Dispatch-Id: <jti>`** even for later clock/policy/nonce/framing refusals; before those checks the header is absent. It is a correlation value, not another grant or a signature. The gateway requires exactly one matching header, the configured authenticated TLS peer, a valid E02 envelope/HTTP mapping, and matching invoke request id when an invoke envelope was valid. GET still has public request_id null. Authenticated early failures may have null request_id. A missing/mismatched correlation or invalid reply after a sent mutation supplies no definitive business observation; handle it as §7 uncertainty, never resend. Response data is not accepted from a cached or unrelated interaction.

### 4.2 Issuer approval assertion

The existing Invocation `approval` remains exactly `{reference,evidence}`; both strings are forwarded unchanged. Evidence is compact JWS with typ **`b10x.connectors-approval.v1+jws`**, the fixed Ed25519 header above, and closed claims `{iss,aud,reference,subject,iat,nbf,exp}`. subject is the complete CanonicalApprovalSubject, approval_mode required. aud is the executing leaf's exact configured approval audience, distinct from its delegation audience. Reference is a newly issued 256-bit random opaque identifier (64 lowercase hex); outer and signed references match exactly. A configured issuer never reissues a reference with another subject or validity interval. Proofs cannot be exchanged across types/audiences.

For a **new** candidate only, the leaf verifies its configured approval issuer/key/policy, signature, time, reference, and exact recomputed subject. It does not trust a proof because the gateway read it or because an issuer string matches. At the atomic spend decision, it rechecks proof/key/current admission and expiry; an earlier check is not a spend lease. Signature failure, subject/expiry mismatch and unsupported issuer give axis-free approval_refused without dispatch. Required missing proof is approval_required. A known previously spent reference is approval_replayed only after current result/disclosure admission; the F02 authoritative winner recheck still precedes returning that refusal after a miss. An exact admitted existing-key observation does not revalidate, refresh or re-spend the old approval.

## 5. Time, deadlines and durable uniqueness

The first proof profiles require a trusted clock interval `[lower, upper]` containing actual current time, width at most four seconds (at most ±2 s about its midpoint). If that bound cannot be established, refuse new proof admission/spend; never treat clock failure as expiry or absence. The issuer sets **iat = t**, where t is the floor of its bounded clock interval midpoint in Unix seconds; **nbf = iat − 5**, and **exp = iat + 25 for delivery** or **exp = iat + 295 for approval**. Receivers require these exact equalities for the selected proof type, using checked arithmetic and the integer bounds in §4.1; shorter, overlong, wrong-profile and malformed windows all refuse. Thus the complete spans are exactly 30 and 300 seconds. At initial verification, final delivery entry and approval spend, proof validity additionally requires **nbf <= lower and upper < exp**, with no extra expiry skew. Equality at expiry refuses. Underflow/overflow refuses both issuance and reception. The five-second not-before margin permits bounded inter-host clock differences; it does not permit a receiver to accept an arbitrary future iat merely because iat < exp.

Delivery time permits one authenticated entry only after the final post-nonce-acknowledgement decision in §4.1. A store suspension across expiry, key/projection revocation, loss of bounded clock certainty or deadline exhaustion refuses that entry, even when consume succeeded. Expiry after admission does not by itself erase a running attempt or turn it into a new one. It does not extend current authority, the approval's validity at spend, or the execution deadline. The gateway derives the signed absolute deadline from its remaining monotonic execution budget using its conservative clock lower bound. The leaf uses the remaining signed deadline (against its upper bound) and its own configured ceiling, whichever is earlier. Policy, control, preparation, audit and provider work consume that remaining budget; no hop, signature or refresh resets it. An already elapsed deadline refuses new dispatch and waiting follows the original-outcome rules. A deadline is not an approval subject or a business key.

| Record | Logical owner and uniqueness | Retention and refusal |
|---|---|---|
| Delegation nonce receipt | Executing leaf; exact structured `(stable gateway issuer, receiver instance, jti)` across every ingress/replica. Kid, signature, path and request id are excluded. | Atomic durable consume before admission. Duplicate is not_granted with no original disclosure; unavailable/ambiguous consume is unavailable and grants no entry. Never evict a live nonce for capacity. Retire only once trustworthy lower >= exp; no accepted token can remain valid then. Uncertain time/restart extends retention, not reuse. |
| Approval redemption | Executing leaf; exact `(approval issuer, reference)` bound to that one leaf subject and one Prepared AttemptRecord. Route/alias/kid/request id are excluded. | Acknowledged atomic spend once, with at most one successful approval redemption per Prepared attempt. Ambiguous/unavailable spend cannot open the dispatch gate. First profile retains spent uniqueness tombstones without automatic expiry; capacity refuses new spending before dispatch. Payload compaction cannot resurrect a reference or delete the linked attempt/audit evidence. |
| Key reservation/AttemptRecord | Existing F02/F01 executing-leaf owners; namespace, fingerprint and gate remain as specified. | Exact admitted outcomes can be observed with fresh transport authority and no second approval spend; Pending/Quarantined remain conservative across restart. |

Store exact tuple coordinates or an injective canonical tagged-array encoding (`connectors.delivery-nonce/v1`, `connectors.approval-spend/v1`); hashes may index but must not replace exact tuple comparison. The signature-verifying issuer must not reuse jti or approval references on key rotation/restart. Storage copies, failover and local backups must not activate the same stable leaf identity with divergent/empty uniqueness history. The same approval cannot be spent via direct and gateway ingress independently; subject origin binding refuses the wrong ingress, and all admitted leaf ingress shares the one spend authority. Concrete backend/replication/retention implementation remains with persistence ownership; a host unable to meet these semantics cannot advertise the profile.

## 6. End-to-end admission and single spend

1. Client describes the gateway under current authentication, sees the admitted alias/curation, and deliberately performs the preparation read with its selected connection and input. Gateway and leaf resolve the safe subject without provider use or business reservation.
2. The configured issuer approves that exact canonical subject under its own issuance/human policy. Client invokes the gateway's current descriptor with unchanged reference/evidence, input, selected connection and optional business key.
3. Gateway authenticates and rechecks its current policy/projection/mapping. It records its audited admission before forwarding. It forwards one correctly mapped leaf invocation with the original input value, key and approval strings unchanged, and one fresh delivery assertion binding its final serialized bytes. It never redeems approval or creates a second business AttemptRecord. Only correlation/alias/selected connection/revision mapping changes, as explicitly signed.
4. Leaf verifies and consumes delivery authority, then current caller/result access and exact resolved leaf input/metadata. It inspects the F02 namespace/key before validating fresh approval. Exact live matches follow wait/replay/quarantine; conflicting fingerprints refuse. Current access denial discloses no original key or effect.
5. For a new candidate, reconstruct the canonical subject, verify required approval, prepare provider inputs without dispatch and establish credential readiness. Preserve F02's authoritative winner recheck after a miss before returning any approval/preflight refusal. Preparation/preflight cannot broaden the previously approved metadata; an effect-relevant change requires re-resolution and exact subject comparison.
6. Leaf acknowledges its admission audit and atomically prepares its one AttemptRecord plus any keyed reservation. It performs the one approval spend bound to that Prepared attempt, then separately wins the durable Prepared→Dispatching gate. Uncertain spend or gate acknowledgement grants no send; abort/recovery fencing follows F01. Only the live definite gate winner calls the provider once.
7. Leaf records its known applied/refused or conservative unknown observation. Gateway validates the correlated response, preserves the original attempt/request/effect/replay facts, and adds its own audit result plus source_audit from the leaf. Current disclosure must still be admitted when returning a waited/replayed result. Recovery never resends.

Approval spend is not the send fence. A spent approval plus aborted Prepared attempt remains spent and not_attempted. Transport nonce consumption proves neither approval redemption nor dispatch. Gateway audit completion proves neither leaf admission nor business outcome.

## 7. Failure and correlation matrix

| Boundary | Required outward observation |
|---|---|
| Gateway current caller/result admission fails | Safe refusal without mutation metadata or original disclosure, irrespective of a cached key. |
| Gateway refuses preparation, stale mapping or signing before any downstream send | No new business dispatch. For a keyed request that could name an existing original, do not label the original not_attempted without an admitted authoritative observation; ordinary refusal may omit mutation. For a definitely new unkeyed candidate, admitted not_attempted with null original identity is permitted. |
| Delivery signature/type/audience/raw-body binding or proof-purpose framing invalid | Unauthorized before application context; no business dispatch, spend or trusted body coordinates. A replayed valid nonce is not_granted; nonce-store uncertainty or loss of trusted clock/policy availability is unavailable. These are transport admission facts, not a classification of any earlier business effect. |
| After nonce acknowledgement, proof expired/key revoked/projection denied, or signed deadline exhausted | No authenticated entry, approval spend or provider dispatch; keep nonce consumed. Expired/revoked proof is unauthorized, current projection denial is not_granted, elapsed deadline is timeout; unavailable trusted facts remain unavailable. |
| Strict application decoding or claims-to-Invocation comparison fails after entry | E02 invalid_input for malformed input; unauthorized for a valid decoded envelope that contradicts authenticated claims. No business dispatch or approval spend; consumed nonce remains consumed. |
| Nonce consumed; leaf policy or approval fails | No new send; preserve exact safe refusal and admitted disclosure rules. The consumed delivery is never retried. |
| Approval spend uncertain; gate never granted | Fence Prepared to Aborted if possible; this candidate not_attempted, proof potentially spent. Do not refund or infer usability from a lost acknowledgement. |
| Gate acknowledgement or downstream response uncertain | outcome_unknown unless a valid definitive original observation exists. A gateway cannot infer not_attempted from its local socket future, nonce state, deadline or empty cache. No reroute/resign/resend. |
| Valid leaf applied/refused result; gateway audit write fails | Preserve effect and original identity; current gateway audit incomplete with acknowledged anchor, source_audit retains leaf audit facts. |
| Definitive applied result cannot be delivered safely | E02 error/applied where verified knowledge exists; otherwise unknown. Never fabricate a result, original attempt id or leaf audit reference. |
| Deliberate same-key observation with fresh delivery and old spent/expired approval | Recheck current access, observe exact original key outcome before fresh approval validation, no second spend/gate/send. An old transport nonce still refuses. |
| Gateway-only unrelated descriptor refresh | Refuse stale submitted projection, refresh/describe explicitly, then deliberate invocation may reuse a still-identical canonical approval. Leaf/route subject changes require new approval and conflict with a live old-key fingerprint. |

## 8. ESS and verification boundary

[Delegation ESS](../../ess/domains/delegation.yaml) gives the subject, preparation, proof/context values and nonce/redemption records typed homes. A preparation is a value, not a persisted entity. A nonce receipt references exactly one existing ServiceConfiguration. A successful approval redemption references exactly one executing ServiceConfiguration and one existing Prepared attempt. It is a separate receipt, not a second AttemptRecord or permission to send. Authority/issuer/route configuration entities and backend mechanics remain UNMAPPED where their ownership is not yet modeled. Existing idempotency executor absence and route fingerprint are explicit values.

Required fixtures cover exact canonical equality; both connection selection modes; harmless projection refresh versus target/route changes; absent realm/executor; different gateway/direct origin; request/body/purpose/receiver/algorithm/key substitution; clock boundaries and nonce retention; duplicate/ambiguous nonce versus duplicate/ambiguous approval spend; same-key observation with expired original approval; and gateway loss before/after the leaf gate. Model/scenario compilation and cryptographic byte vectors must be reported separately from real receiver, database, clock, policy or provider execution. No such runtime is implemented here. Evidence and two independent rechecks are recorded in [verification](../../docs/evidence/federated-approval-20260908/verification.md).

The event-claim ingress, external issuer UI/API, concrete Identity/policy/custody clients, durable backend and multi-hop topology remain separately owned bindings. Their absence is an advertisement gate, not permission to substitute broad gateway or caller-made authority. The canonical one-hop subject, proof bytes, verifier/redemption ownership and refusal semantics above are selected now; no later implementation may silently choose different subjects or spend twice.
