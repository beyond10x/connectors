# auth.acquisition/v1alpha1

- **Status:** proposed, not implemented.
- **Family:** auth. Sibling documents: [connection](../../connection/v1alpha1/semantics.md), [profile](../../profile/v1alpha1/semantics.md), [custody](../../custody/v1alpha1/semantics.md), [capability](../../capability/v1alpha1/semantics.md), [evidence](../../evidence/v1alpha1/semantics.md).
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `auth.acquisition/v1alpha1` |
| Profiles | `static_entry`, `oauth2_authorization_code`, `oauth2_client_credentials`; reserved: `workload_identity`, `exec_plugin`, `host_issued` (the last is specified in [sessions](../../../sessions/v1alpha1/semantics.md)) |
| Parties | host coordinator (shared), provider auth implementation (adapter-owned), trusted user interface, custody |

Design responsibility two of four: establish, refresh, revoke, or repair authorization, owned by a shared coordinator plus the provider auth implementation (`docs/design.md:527`). The coordinator owns lifecycle, state, expiry, concurrency, recovery; the provider implementation owns request construction and response interpretation (`docs/design.md:551`).

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| Three copies of the authorization-code flow: GitLab PKCE S256 form-encoded, Jira no PKCE JSON body, Slack HTTP Basic through an egress gate; only Jira refreshed under a double-checked lock with skew | `../connectors/crates/connector-oauth/src/lib.rs` module header (measured 2026-08-25) | change: one coordinator, provider-declared differences (`docs/design.md:551`) |
| Connect session: `connect_session_create` returns a short-lived `completion_endpoint` and optional `browser_completion_url`; creator polls `connect_session_status`; only a terminal completed status names `connection_ref` | `../connectors/contracts/connector-connection/v0alpha1/README.md:13-22` | preserve the shape (begin → action → poll → connection ref) |
| Pasted credential posts directly to the Connector process; the agent sees only the opaque one-use URL | same README | preserve: ordinary model results never contain secret material or reusable completion authority (`docs/design.md:545`) |
| Custody topologies: pasted once, external credential source, stored at execution, workload identity, OAuth2 acquisition | `../connectors/docs/design/07-credential-custody-topologies.md:121-200` | preserve the first, third and fifth as profiles here; second and fourth reserved |
| Slack rotating refresh tokens: exchange and persistence cannot be one transaction; lost response may require reauthorization | `docs/design.md:598` | preserve as the `uncertain_refresh` outcome |
| Refresh coordination per credential set across replicas | `docs/design.md:596` | preserve |

## 3. Types

Coordinator interfaces have distinct bindings. Safe `auth.begin` and `auth.status` are admitted management operations; forwarding additionally needs the complete selected federation binding. `auth.complete` is a trusted callback/protected-entry port, not ordinary invoke input or a generic forwardable operation. Refresh and revoke retain their private/admitted entry points below (`docs/design.md:785`):

```text
auth.begin(profile, requested_scopes?, repair_of?: connection) -> AuthorizationAction
auth.complete(acquisition_ref, evidence)                        -> AcquisitionStatus  (trusted completion port; separately bound)
auth.status(acquisition_ref)                                    -> AcquisitionStatus
auth.refresh(connection)                                        -> RefreshOutcome     (host-internal, not caller-invocable)
auth.revoke(connection)                                         -> RevocationOutcome  (via connections.revoke)
```

```json
{ "acquisition": "acq_…", "expires_unix_ms": 0,
  "action": { "kind": "browser", "url": "https://<host callback service>/…one-use…" } }
{ "acquisition": "acq_…", "expires_unix_ms": 0,
  "action": { "kind": "protected_entry", "url": "https://<host>/…one-use…", "fields": ["token", "user"] } }
{ "acquisition": "acq_…", "state": "pending" }
{ "acquisition": "acq_…", "state": "completed", "connection": "conn_…" }
{ "acquisition": "acq_…", "state": "failed", "reason": "expired | refused_by_provider | identity_mismatch | scope_insufficient | custody_unavailable" }
```

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

Flow (specializes `docs/design.md:543-549`):

1. Host admits `auth.begin` against caller, tenant, target connection (for repair), profile, and configured registration.
2. Coordinator creates one-purpose expiring state correlated to the request and the allowed callback or entry URL. State is single-use.
3. The action URL is presented by a trusted interface. Ordinary results carry the URL once; it is not reusable authority after completion or expiry.
4. Completion evidence (OAuth callback code and state, or a posted static credential) is validated for correlation, expiry, one-time use. The provider implementation performs the exchange and validates returned identity, token kind, and granted scopes against the profile.
5. Repair: the returned external identity must match the bound one, else `identity_mismatch` and the connection remains in its prior state.
6. Custody: the credential set is written durably (`auth.custody`) before the connection's active reference is published; both happen before `completed` is observable.
7. The caller receives the connection ref and safe status. Completion never executes a previously failed business operation.

### 4.1 Refresh exclusion, authorization and recovery

Refresh is triggered by `auth.evidence` (expiry near) or by a provider `401` at the execution boundary; it is never caller-invocable. The **host coordinator and its metadata binding** own cross-replica exclusion and the durable refresh ledger. Custody owns only immutable sensitive versions. A custody CAS or an expiring lease alone cannot authorize an exchange (`docs/design.md:596-600`).

The serialization key is one host-private `CredentialGeneration` from [the shared ESS model](../../../../ess/domains/credentials.yaml): an immutable material capture bound to an instance, connection, profile and provider authority, with an expected external identity. It is neither a secret-derived identifier, a file path, nor a custody version. Capturing bytes does not validate identity. A refresh result receives a new generation even if the account is unchanged; validation and dispatch obey [evidence](../../evidence/v1alpha1/semantics.md). The host must prevent the same rotating refresh material from entering two independently refreshable generations or connection bindings. Repeated observations/restarts reuse the existing private capture association where the material is unchanged; aliases that cannot be established safely are refused. UUID uniqueness alone cannot prevent duplicate use of the same token.

Each attempt has a unique id, a source generation, a current owner and a non-reusable ownership token (fence), and the connection binding revision it expects. All replicas use one linearizable coordinator authority for this metadata; process mutexes, unlocked files and disconnected local ledgers are insufficient. These are required logical operations, not a choice of database:

| Atomic coordinator operation | Preconditions and durable effect |
|---|---|
| Reserve | Compare the current source/binding and non-revocation; acquire the source's unique reservation and create `Reserved`. A contender observes the existing attempt and waits or refuses; it cannot send. |
| Authorize exchange | Compare `Reserved`, current owner/fence, source/binding and non-revocation; atomically mark `Authorized`, consume that source's one-exchange authority and invalidate its outstanding dispatch admissions before any possible provider call. Only the original live invocation that observes a committed success may send once. A timeout/unknown commit result grants no send. Re-reading or replaying success never grants another send. Provider clients must disable implicit retries. |
| Fence before authorization | CAS `Reserved → Fenced` against authorization, invalidate the old owner, then release the reservation for a new attempt on the still-current source. Lease expiry merely permits attempting this CAS. The old owner must fail its later authorization. |
| Store response | After receiving and validating a complete candidate under F05 and durably storing the whole credential set, atomically attach its new generation to the `Authorized` attempt under the current fence and mark `ResponseStored`. A loose orphan blob is not a committed response record. |
| Resolve authorized owner loss | Atomically inspect/CAS against response storage. If `ResponseStored` won, recover publication only. Otherwise make `Authorized → Uncertain`; the source remains consumed. No lease transfer, restart, unchanged active pointer or claimed absence of send permits another exchange. |
| Recover committed response | Fence the old publication owner and move `ResponseStored → Recovering` with a new owner/token. Recover exactly the recorded candidate, rechecking custody and evidence. This grants publication only. In this first profile a lost recovery owner causes discard/repair, rather than a second recovery transfer. |
| Publish | In one metadata transaction compare attempt state, current owner/fence, expected source and binding revision, exact candidate, current F05 evidence and non-revocation. Replace active version/generation, advance the binding revision, mark `Published`, and invalidate all old-generation dispatch admissions together. |
| Revoke or replace binding | Use the same serialized metadata authority as publish/authorize, advance the binding revision and invalidate outstanding dispatch admissions. Revocation is monotonic for that binding; a stale refresh cannot clear it. Reauthorization creates a new binding revision and fresh material. |

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
- Repair returning a different account id → `identity_mismatch`; prior credential still active.
- Fake provider returns fewer scopes than `minimum` → `scope_insufficient`; nothing written to custody.
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
| Callback/entry HTTP ingress with bounded bodies, separate from the operations surface | `crates/connectors-host/src/server.rs` (new routes) |
| SDK trait for the private provider interface (`begin`/`exchange`/`refresh`/`revoke`) with a `CredentialUpdate` type that has no `Debug`/`Serialize` on its sensitive half (pattern: `Secret`, `crates/connectors-sdk/src/lib.rs:33-34`) | `crates/connectors-sdk` |
| Shared OAuth mechanics (state, PKCE optional, token response parsing) as SDK helpers without owning the transport | `crates/connectors-sdk` (old rationale: `connector-oauth` header, "does not own the request") |

## 9. ESS entities

| Entity | Notes |
|---|---|
| `Acquisition` (identity: acquisition ref), lifecycle `pending → completed | failed | expired` | relations: `for_profile → AuthProfile` (one), `repairs → Connection` (zero or one), `yields → Connection` (zero or one) |
| `CredentialSet` | defined in custody; custody version is not refresh identity |
| `RefreshAttempt` | [ESS](../../../../ess/domains/refresh.yaml): `Reserved → Authorized → ResponseStored → Published`, pre-authorization fencing, stored-response recovery, terminal uncertainty/discard; exactly one source-generation reference |
| `CredentialGeneration` | [shared ESS](../../../../ess/domains/credentials.yaml): host-private immutable capture; expected identity, not verified evidence |
| Typed coordinator decisions | Trusted transient values selected inside the atomic operation; never caller authority or reusable approval |
| Trusted UI and callback host identity | UNMAPPED |

The [verification record](verification.md) separates compiled authored scenarios from runtime obligations. ESS checks typed values, references and lifecycle causation; it does not prove per-source uniqueness, real lease fencing, actual send counts, byte pinning or cross-entity atomicity. These remain explicit `UNMAPPED` implementation obligations.

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Callback host placement in federation | the owning adapter service's host serves the callback; the gateway forwards `auth.begin`/`status` as management operations (`docs/design.md:785`) |
| PKCE default | `none` unless the profile source says supported; then `S256` |
| Expiry values | 600 s / 300 s first-profile defaults |
