# auth.acquisition/v1alpha1

- **Status:** proposed, not implemented.
- **Family:** auth. Sibling documents: [connection](../../connection/v1alpha1/semantics.md), [profile](../../profile/v1alpha1/semantics.md), [custody](../../custody/v1alpha1/semantics.md), [capability](../../capability/v1alpha1/semantics.md), [evidence](../../evidence/v1alpha1/semantics.md).
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `auth.acquisition/v1alpha1` |
| Paths | `static_config` (deployment binding, no acquisition session); coordinator flows: `static_entry`, `oauth2_authorization_code`, `oauth2_client_credentials`; reserved here: `oauth2_password`, `workload_identity`, `exec_plugin`, `host_issued` (session authority has its separate [sessions](../../../sessions/v1alpha1/semantics.md) contract) |
| Parties | host coordinator (shared), provider auth implementation (adapter-owned), trusted user interface, custody |

Design responsibility two of four: establish, refresh, revoke, or repair authorization, owned by a shared coordinator plus the provider auth implementation (`docs/design.md:527`). The coordinator owns lifecycle, state, expiry, concurrency, recovery; the provider implementation owns request construction and response interpretation (`docs/design.md:551`).

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| Three copies of the authorization-code flow: GitLab PKCE S256 form-encoded, Jira no PKCE JSON body, Slack HTTP Basic through an egress gate; only Jira refreshed under a double-checked lock with skew | `../connectors/crates/connector-oauth/src/lib.rs` module header (measured 2026-08-25) | change: one coordinator, provider-declared differences (`docs/design.md:551`) |
| Connect session: `connect_session_create` returns a short-lived `completion_endpoint` and optional `browser_completion_url`; creator polls `connect_session_status`; only a terminal completed status names `connection_ref` | `../connectors/contracts/connector-connection/v0alpha1/README.md:13-22` | preserve the shape (begin → action → poll → connection ref) |
| Pasted credential posts directly to the Connector process; the old agent saw the opaque one-use URL | same README | change disclosure: a separately admitted trusted UI receives continuation authority; ordinary results get safe refs/action kinds only ([management](../../management.md)) |
| Custody topologies: pasted once, external credential source, stored at execution, workload identity, OAuth2 acquisition | `../connectors/docs/design/07-credential-custody-topologies.md:121-200` | preserve the first, third and fifth as profiles here; second and fourth reserved |
| Slack rotating refresh tokens: exchange and persistence cannot be one transaction; lost response may require reauthorization | `docs/design.md:598` | preserve as the `uncertain_refresh` outcome |
| Refresh coordination per credential set across replicas | `docs/design.md:596` | preserve |

## 3. Types

Coordinator interfaces have distinct bindings, owned as specified in [management](../../management.md). Safe-payload `auth.begin` and `auth.status` are host-owned admitted management operations; forwarding additionally needs the complete selected federation binding. `auth.complete` is a trusted callback/protected-entry port, not ordinary invoke input or a generic forwardable operation. Refresh and revoke retain their private/admitted entry points below (`docs/design.md:785`):

```text
auth.begin(profile, requested_scopes?, repair_of?: connection) -> AuthorizationAction
auth.complete(acquisition_ref, evidence)                        -> AcquisitionStatus  (trusted completion port; separately bound)
auth.status(acquisition_ref)                                    -> AcquisitionStatus
auth.refresh(connection)                                        -> RefreshOutcome     (host-internal, not caller-invocable)
auth.revoke(connection)                                         -> RevocationOutcome  (via connections.revoke)
```

```json
{ "acquisition": "acq_…", "expires_unix_ms": 0,
  "action": { "kind": "browser" } }
{ "acquisition": "acq_…", "expires_unix_ms": 0,
  "action": { "kind": "protected_entry" } }
{ "acquisition": "acq_…", "state": "pending" }
{ "acquisition": "acq_…", "state": "completed", "connection": "conn_…" }
{ "acquisition": "acq_…", "state": "failed", "reason": "expired | refused_by_provider | identity_mismatch | insufficient_scope | custody_unavailable" }
```

These ordinary result examples contain no actionable URL. The separately bound trusted UI channel receives the expiring owner-bound URL and protected-entry field schema; status/discovery never reissue them. `pending` is acquisition state and need not identify a public Connection. A completed acquisition records acknowledged baseline publication, while current connection status is queried separately.

Private provider interface (adapter-owned, never serialized):

```text
begin(registration, requested_access)  -> ProviderAuthorizationRequest
exchange(validated_callback_evidence)  -> CredentialUpdate
refresh(current_credential_set)        -> CredentialUpdate
revoke(current_credential_set)         -> RevocationOutcome
```

`CredentialUpdate` separates safe metadata (external identity, granted scopes, expiry, kind) from sensitive material (`docs/design.md:562`).

Outcomes of refresh: `refreshed`, `reauthorization_required`, `insufficient_scope`, `invalid_or_revoked`, `custody_unavailable`, `uncertain` (`docs/design.md:600`).

## 4. Rules

### 4.0 Acquisition paths and required declarations

“Specified” below means this proposed contract defines a path. It does not mean the current host, adapter schema, profile reader or codec implements it. An advertised supported profile needs all of those bindings and its validation/placement implementation. Configuration flags do not promote a reserved flow to supported.

| Flow | Disposition and required declaration | Fields/path not permitted |
|---|---|---|
| `static_config` | Specified deployment binding. Profile declares accepted material/transport shape and admitted validation plan; configuration supplies scoped references, or the explicit no-child-credential binding in profile §4.2. | No auth.begin, acquisition id, protected-entry action, OAuth endpoints, grants, registration or callback. No credential bytes in ordinary configuration/result data. |
| `static_entry` | Specified protected entry. Profile declares expected entry fields/material shape, identity/grant interpretation and validation plan with sources. Host must bind custody, coordinator and admitted protected UI/ingress. | No OAuth endpoints, OAuth grants, redirect or browser PKCE fields. Protected form submission alone is not completion/publication. |
| `oauth2_authorization_code` | Specified browser flow. Require nonempty authorize_url and token_url, vendor sources, authorization_code grant, explicit PKCE choice and vendor justification, registration kind and sourced client-auth method/policy, callback binding, token/identity/grant interpretation, validation and refresh policy. Refresh grant only when supported by the cited profile. | No inferred endpoints, token encoding or PKCE from another provider. Registration values remain configuration/custody. |
| `oauth2_client_credentials` | Specified noninteractive flow. Require nonempty token_url, vendor sources, client_credentials grant, configured confidential registration and declared client-auth/token/identity/grant interpretation and validation. This first profile does not request/accept refresh-token behavior; obtaining another credential set is a new admitted acquisition. | authorize_url, callback/redirect, PKCE, browser action and refresh_token grant are rejected; no trusted browser channel is required. |
| `oauth2_password`, `workload_identity`, `exec_plugin`, `host_issued` | Reserved in this acquisition contract. Preserve names for reviewed future bindings; reject selection/advertisement as implemented here. Exec/session mechanism constraints elsewhere remain requirements for those future/separate profiles. | No inferred generic auth.begin/completion path or flag-only enablement. |

All profile declarations remain credential/reference-free, with bounded sources and material/validation descriptions. Required safe endpoint strings must meet the selected host's reviewed egress policy; their exact provider values and request formats need cited vendor evidence. This matrix changes no actual adapter-kind schema.

`static_config` is activation of an explicitly configured binding, not a pasted credential or a managed acquisition. The host separately admits activation/revalidation, resolves only the configured references at that boundary, captures coherent immutable material, validates the candidate and publishes its metadata under the same binding/revocation fence as other credential paths. Read-only custody need not copy an externally supplied secret into a writable store; its immutable snapshot must remain available for dispatch. Unavailable custody/validation refuses activation. Listing does not activate, run an exec helper or resolve secrets. Changes require fresh capture/validation; filenames and revisions do not prove identity continuity. Binding metadata publication must be acknowledged, but no fictitious acquisition/completed event is emitted. Static no-credential profiles instead validate their explicit destination/route binding and applicable checks, with no capture, external identity or custody write. No static binding automatically refreshes or starts a browser.

Managed-flow sequence (specializes `docs/design.md:543-549`; static_config follows the preceding activation path):

1. Host admits `auth.begin` against caller, tenant, target connection (for repair), profile, and configured registration.
2. Coordinator creates one-purpose expiring state correlated to the request. Interactive flows bind the allowed callback or entry URL; noninteractive client credentials have no completion URL. State is single-use.
3. For browser/protected-entry flows, the owning coordinator delivers the one-use action URL only to the separately admitted trusted UI channel. Ordinary results carry acquisition ref, expiry and action kind; no usable completion authority. Without the needed protected delivery/ingress binding that interactive flow is unavailable. Non-interactive client credentials skip this UI step and may complete in begin through their admitted registration/provider/custody path. Current authority, owner, expiry and one-use checks remain required at completion.
4. Interactive completion evidence (OAuth callback code and state, or a posted static credential) is validated for correlation, expiry, one-time use. Client credentials exchanges directly from its admitted registration. The provider implementation performs the applicable exchange/entry validation and validates returned identity, token kind and granted scopes against the profile minimum, recording actual grants; optional requested grants are not mandatory publication requirements ([profile §4.1](../../profile/v1alpha1/semantics.md#41-baseline-and-operation-requirements)).
5. Repair: the returned external identity must match the bound one, else `identity_mismatch` and the connection remains in its prior state.
6. Baseline validation, including any explicitly mandatory global verification, must succeed. Custody stores the complete credential set durably before the coordinator publishes its active reference under the current binding/revocation fence; both acknowledgements precede `completed`. Optional verification does not prevent publication; required operation-only checks still gate those operations. A failed independent repair preserves the prior binding subject to its own current validity, expiry and revocation.
7. The caller receives the connection ref and safe status. Completion never executes a previously failed business operation.

### 4.1 Refresh exclusion, authorization and recovery

Refresh is triggered by `auth.evidence` (expiry near) or by a provider `401` at the execution boundary; it is never caller-invocable. The **host coordinator and its metadata binding** own cross-replica exclusion and the durable refresh ledger. Custody owns only immutable sensitive versions. A custody CAS or an expiring lease alone cannot authorize an exchange (`docs/design.md:596-600`).

The serialization key is one host-private `CredentialGeneration` from [the shared ESS model](../../../../ess/domains/credentials.yaml): an immutable material capture bound to an instance, connection, profile and provider authority, with an expected external identity. It is neither a secret-derived identifier, a file path, nor a custody version. Capturing bytes does not validate identity. A refresh result receives a new generation even if the account is unchanged; validation and dispatch obey [evidence](../../evidence/v1alpha1/semantics.md). The host must prevent the same rotating refresh material from entering two independently refreshable generations or connection bindings. Repeated observations/restarts reuse the existing private capture association where the material is unchanged; aliases that cannot be established safely are refused. UUID uniqueness alone cannot prevent duplicate use of the same token.

Each attempt has a unique id, a source generation, a current owner and a non-reusable ownership token (fence), and the private connection publication fence it expects. All replicas use one linearizable coordinator authority for this metadata; process mutexes, unlocked files and disconnected local ledgers are insufficient. These are required logical operations, not a choice of database:

| Atomic coordinator operation | Preconditions and durable effect |
|---|---|
| Reserve | Compare the current source/binding and non-revocation; acquire the source's unique reservation and create `Reserved`. A contender observes the existing attempt and waits or refuses; it cannot send. |
| Authorize exchange | Compare `Reserved`, current owner/fence, source/binding and non-revocation; atomically mark `Authorized`, consume that source's one-exchange authority and invalidate its outstanding dispatch admissions before any possible provider call. Only the original live invocation that observes a committed success may send once. A timeout/unknown commit result grants no send. Re-reading or replaying success never grants another send. Provider clients must disable implicit retries. |
| Fence before authorization | CAS `Reserved → Fenced` against authorization, invalidate the old owner, then release the reservation for a new attempt on the still-current source. Lease expiry merely permits attempting this CAS. The old owner must fail its later authorization. |
| Store response | After receiving and validating a complete candidate under F05 and durably storing the whole credential set, atomically attach its new generation to the `Authorized` attempt under the current fence and mark `ResponseStored`. A loose orphan blob is not a committed response record. |
| Resolve authorized owner loss | Atomically inspect/CAS against response storage. If `ResponseStored` won, recover publication only. Otherwise make `Authorized → Uncertain`; the source remains consumed. No lease transfer, restart, unchanged active pointer or claimed absence of send permits another exchange. |
| Recover committed response | Fence the old publication owner and move `ResponseStored → Recovering` with a new owner/token. Recover exactly the recorded candidate, rechecking custody and evidence. This grants publication only. In this first profile a lost recovery owner causes discard/repair, rather than a second recovery transfer. |
| Publish | In one metadata transaction compare attempt state, current owner/fence, expected source and private publication fence, exact candidate, current F05 evidence and non-revocation. Replace active version/generation, advance the private publication fence, mark `Published`, and invalidate all old-generation dispatch admissions together. |
| Revoke or replace binding | Use the same serialized metadata authority as publish/authorize, advance the private publication fence and invalidate outstanding dispatch admissions. Revocation is monotonic for that binding; a stale refresh cannot clear it. Reauthorization creates a new private publication fence and fresh material. |

A successful authorization is the irreversible boundary even when the process dies **before** the network send. After it, uncertainty is conservative: the token may have been consumed. Transport failure, negative-looking provider responses, candidate validation failure, custody failure after exchange, response loss and local commit ambiguity never reopen source authorization. An unusable result requires repair/reauthorization. No retry exception based on alleged provider grace or token reuse is selected here.

Publication uses a strict cutoff: once it commits, no admission pinned to the old generation may open a new provider dispatch. A dispatch already opened before the cutoff is not retroactively undone. Pinning preserves the validated bytes; it cannot override known revocation, expiry or invalidation. At exchange authorization the old generation's refresh authority is consumed; old-generation dispatch is withheld atomically with that authorization while refresh is unresolved. Publication makes the new validated generation available; uncertainty leaves refresh and dispatch blocked pending repair.

If revocation commits first, publication refuses. If publication commits first, revocation applies to the newly active generation and blocks later dispatch. Revocation cannot undo an already-authorized provider exchange; it prevents its result from restoring authority. A stale owner's publication fails even if its response is valid. A committed publication whose reply was lost is observed from the ledger, never repeated as a new exchange. Superseded, revoked or quarantined source records retain the fact of consumed authorization; cleanup must not allow their identity or material to be reintroduced as refreshable.

Required durability includes surviving host restart with reservation/consumption, ownership and publication decisions intact. A binding that cannot provide the stated atomicity, fencing and durable recovery must refuse rotating refresh; falling back to per-process locking or a second exchange is forbidden. In-memory test bindings may simulate the protocol but claim no restart durability. Safe outcome names remain `refreshed`, `reauthorization_required`, `insufficient_scope`, `invalid_or_revoked`, `custody_unavailable` and `uncertain`; detailed ledger refusals below are private, not new public error envelopes. `uncertain` records the exchange observation and requires reauthorization, never “not configured.”

Scope and identity checks apply to refresh results; rotation never widens permission. Refresh success never retries a business write (`docs/design.md:600`; `operations` mutation profile). The separate read-retry contract decision is not settled here.

### 4.2 Failure matrix

| Observation | Required next action | Another exchange with the source? |
|---|---|---|
| Two replicas before authorization | One reservation/authorization; contender waits and re-reads active generation | No |
| Owner lost while durably `Reserved` | Fence old owner atomically, then reserve a new attempt | Yes, only after the fence wins before authorization |
| Authorization commit unknown, or committed but owner lost before send | Quarantine/repair unless committed response is recoverable | No |
| Provider may have consumed token; response lost | `uncertain`, require reauthorization, retain consumed record | No |
| Response exists only in memory, or custody write outcome unknown | Quarantine/repair; orphan cleanup cannot infer a result | No |
| Committed candidate/response, owner lost before publish | Fence and recover publication of that candidate; check current evidence | No |
| Candidate identity/scope invalid, unavailable or expired | Refuse publication, discard/repair; retain consumed source | No |
| Stale owner publishes after recovery transfer | Ownership conflict; current owner may publish the same candidate | No |
| Revocation or binding replacement wins publication race | Refuse publication; never clear revocation or overwrite successor | No |
| Publication committed, response to owner lost | Observe `Published` and current binding; no exchange replay | No |
| Stored-response recovery owner also lost | Discard candidate/repair in this first profile | No |

Static entry (`static_entry`): the protected entry page posts to the coordinator over the host's admitted ingress; the value is written to custody and the connection created; the page and URL expire. For `http_basic` profiles the entry has two fields (user half, secret half) as in the Atlassian API token (`../connectors/providers/jira.toml`, `user_env`).

Client credentials (`oauth2_client_credentials`): no browser; `begin` performs the exchange with the configured registration and returns a `completed` status directly.

## 5. Ordering, limits

| Concern | Rule |
|---|---|
| Acquisition expiry | first-profile default 600 s for browser flows, 300 s for entry pages; to be confirmed |
| Concurrent begins for the same connection repair | allowed; the first completion wins; later completions are `expired` |
| Refresh skew | refresh before expiry by a declared margin (old Jira used a skew; value to be measured, `connector-oauth` header) |
| Retry | provider exchange is not retried on unknown outcome |

## 6. Conformance scenarios (`docs/design.md:985`)

- Callback with mismatched state → refused, no exchange call on the fake provider.
- Callback replayed after completion → refused, one connection exists.
- Expired acquisition completed → `expired`.
- Repair returning a different account id → `identity_mismatch`; no replacement publication. Prior material is usable only if independently still current, valid and unconsumed.
- Fake provider returns fewer scopes than `minimum` → `insufficient_scope`; no candidate publication/completed (ordinary validation precedes custody write). Optional requested write scope omitted with minimum read scope present → baseline publication/completed permitted, read eligible and write insufficient_scope.
- Two replicas refresh the same generation concurrently → one committed authorization and at most one exchange; both eventually observe the new generation or the same refusal. Exercise the coordinator binding, not a fake custody lease.
- Owner loss before authorization → fence old attempt before successor reservation. Loss after authorization (including before send) → no second exchange.
- Committed response recovery → current publication owner succeeds; stale owner is refused; revocation/identity replacement wins without resurrection.
- Publication cutoff → old-generation admissions cannot newly dispatch; already-opened transport is not rolled back.
- Fake provider rotates then drops the response → `uncertain`; connection `reauthorization_required`; no second refresh call.
- Swap the custody binding (in-memory ↔ file) → provider auth implementation unchanged (`docs/design.md:1014`).

## 7. Compatibility

- Old `connect_session_*` methods map onto `auth.begin`/`auth.status`; a facade can preserve names.
- Old per-integration OAuth copies are not carried.
- [Service compatibility](../../../service/compatibility.md) is authoritative for the binding. Safe begin/status payloads require the selected management schema; protected entry and completion evidence need their dedicated trusted binding. A unary version does not supply callback protection.


## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| Coordinator module with acquisition state store (expiring, one-use), durable per-generation refresh ledger, atomic authorization/fencing and publication/revocation | new host module |
| Separately admitted protected UI delivery and callback/entry ingress with bounded bodies and owner/correlation/one-use checks | future host binding; exact codec/paths remain unselected, no generic invocation or discovery disclosure |
| SDK trait for the private provider interface (`begin`/`exchange`/`refresh`/`revoke`) with a `CredentialUpdate` type that has no `Debug`/`Serialize` on its sensitive half (pattern: `Secret`, `crates/connectors-sdk/src/lib.rs:33-34`) | `crates/connectors-sdk` |
| Shared OAuth mechanics (state, PKCE optional, token response parsing) as SDK helpers without owning the transport | `crates/connectors-sdk` (old rationale: `connector-oauth` header, "does not own the request") |

## 9. ESS entities

| Entity | Notes |
|---|---|
| `Acquisition` (identity: acquisition ref), proposed lifecycle pending → completed / failed / expired | Not yet declared in ESS; its owner, optional repair target and yielded connection are coordinator facts whose persistent model remains an explicit obligation. No implemented entity/relations are claimed here. |
| `CredentialSet` | defined in custody; custody version is not refresh identity |
| `RefreshAttempt` | [ESS](../../../../ess/domains/refresh.yaml): `Reserved → Authorized → ResponseStored → Published`, pre-authorization fencing, stored-response recovery, terminal uncertainty/discard; exactly one source-generation reference |
| `CredentialGeneration` | [shared ESS](../../../../ess/domains/credentials.yaml): host-private immutable capture; expected identity, not verified evidence |
| Typed coordinator decisions | Trusted transient values selected inside the atomic operation; never caller authority or reusable approval |
| Trusted UI and callback host identity | UNMAPPED |

The [verification record](verification.md) separates compiled authored scenarios from runtime obligations. ESS checks typed values, references and lifecycle causation; it does not prove per-source uniqueness, real lease fencing, actual send counts, byte pinning or cross-entity atomicity. These remain explicit `UNMAPPED` implementation obligations.

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Callback host placement in federation | completion terminates at the one owning logical coordinator through a separately admitted protected ingress; safe begin/status use the selected management binding; no state copying or automatic reroute/resend ([management](../../management.md)) |
| PKCE choice | explicit per authorization-code profile with vendor justification; no inherited/default choice; absent for other first-profile flows |
| Expiry values | 600 s / 300 s first-profile defaults |

Persistence ownership is consolidated in [design §31](../../../../docs/design.md#31-host-persistence-ownership-and-atomicity). AcquisitionCoordinatorPort and RefreshCoordinatorPort retain distinct state ownership while using the required connection authority fence; CustodyPort owns only sensitive immutable versions. This inventory does not supply a backend or execute its atomicity predicates.
