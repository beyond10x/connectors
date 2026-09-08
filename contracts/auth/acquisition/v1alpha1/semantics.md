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

Coordinator operations (host-admitted management operations on the adapter service, forwardable through federation as a management contract, `docs/design.md:785`):

```text
auth.begin(profile, requested_scopes?, repair_of?: connection) -> AuthorizationAction
auth.complete(acquisition_ref, evidence)                        -> AcquisitionStatus
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

Refresh:

- Triggered by `auth.evidence` (expiry near) or by a provider `401` at the execution boundary; never by a caller.
- Serialized per credential set across replicas through a custody lease (`docs/design.md:596`). A second refresher waits for the outcome, then re-reads the active reference.
- A refresh whose provider response is lost after the provider may have rotated the token is `uncertain`; the coordinator marks the connection `reauthorization_required` rather than refreshing again blindly (`docs/design.md:598`).
- Scope and identity checks apply to refresh results; rotation never widens permission.
- Refresh success never retries a business write (`docs/design.md:600`; `operations` mutation profile).

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
- Two replicas refresh the same set concurrently (fake lease) → one exchange on the fake provider; both observe the new reference.
- Fake provider rotates then drops the response → `uncertain`; connection `reauthorization_required`; no second refresh call.
- Swap the custody binding (in-memory ↔ file) → provider auth implementation unchanged (`docs/design.md:1014`).

## 7. Compatibility

- Old `connect_session_*` methods map onto `auth.begin`/`auth.status`; a facade can preserve names.
- Old per-integration OAuth copies are not carried.

## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| Coordinator module with acquisition state store (expiring, one-use) and refresh lease | new host module |
| Callback/entry HTTP ingress with bounded bodies, separate from the operations surface | `crates/connectors-host/src/server.rs` (new routes) |
| SDK trait for the private provider interface (`begin`/`exchange`/`refresh`/`revoke`) with a `CredentialUpdate` type that has no `Debug`/`Serialize` on its sensitive half (pattern: `Secret`, `crates/connectors-sdk/src/lib.rs:33-34`) | `crates/connectors-sdk` |
| Shared OAuth mechanics (state, PKCE optional, token response parsing) as SDK helpers without owning the transport | `crates/connectors-sdk` (old rationale: `connector-oauth` header, "does not own the request") |

## 9. ESS entities

| Entity | Notes |
|---|---|
| `Acquisition` (identity: acquisition ref), lifecycle `pending → completed | failed | expired` | relations: `for_profile → AuthProfile` (one), `repairs → Connection` (zero or one), `yields → Connection` (zero or one) |
| `CredentialSet` | defined in custody |
| Trusted UI and callback host identity | UNMAPPED |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Callback host placement in federation | the owning adapter service's host serves the callback; the gateway forwards `auth.begin`/`status` as management operations (`docs/design.md:785`) |
| PKCE default | `none` unless the profile source says supported; then `S256` |
| Expiry values | 600 s / 300 s first-profile defaults |
