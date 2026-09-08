# auth.capability/v1alpha1

- **Status:** proposed, not implemented. Extends the existing `AuthenticatedHttp` port (`crates/connectors-sdk/src/lib.rs:53-56`, GET only).
- **Family:** auth. Sibling documents: [connection](../../connection/v1alpha1/semantics.md), [profile](../../profile/v1alpha1/semantics.md), [acquisition](../../acquisition/v1alpha1/semantics.md), [custody](../../custody/v1alpha1/semantics.md), [evidence](../../evidence/v1alpha1/semantics.md).
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `auth.capability/v1alpha1` |
| Capabilities | `http-bearer`, `http-basic`, `http-signing`, `http-anonymous`, `mtls-client-identity`, `socket-peer`, `exec-credential-plugin`, `sip-credential-lease`, `session-authority`, `inbound-verifier`, `mediated-http` |
| Nature | the runtime side of authentication: a connection-bound object handed to provider code that authenticates one permitted request at the trusted execution boundary |

Design responsibility four of four (`docs/design.md:529`): "Authenticate a permitted provider request — connection-bound authentication capability at the execution boundary." The requirement is a defined trusted boundary, no general secret-store access from business operations, and no leakage through public interfaces (`docs/design.md:592`).

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| Current `AuthenticatedHttp::get(segments, query)`; segments encoded individually, no arbitrary URL | `crates/connectors-sdk/src/lib.rs:53-56` | preserve, extend with methods and body |
| Configured service forbids automatic retries; the earlier capability draft prescribed refresh plus read redispatch after 401 | [Configured service](../../../service/v1alpha1/semantics.md), F15/E05 review intake | preserve existing profiles; change only through the explicitly selected proposed [read-refresh-once/v1alpha1 binding](read-refresh-once.md), with one refresh participation, at most two business dispatches and one original budget; no current adapter selects it |
| `ScopedHttp`: base URL, one credential, one header, bearer flag; no redirects, verified TLS, optional CA, no proxy, plaintext only for local test | `crates/connectors-host/src/http.rs:11-35`; `contracts/service/v1alpha1/semantics.md`, Auth and configuration | preserve as `http-bearer`; add `http-basic` |
| SIP driver receives `CredentialSet` and requires exactly username then password, passed to `sipx_call::Credentials` | `../connectors/crates/driver-sip/src/lib.rs:40-42,104-114` | preserve as `sip-credential-lease`: the protocol library sees the bytes; the lease bounds when and for what |
| Kubernetes kubeconfig token, token file, client certificate, exec plugin | `../connectors/crates/integration-kubernetes/src/local.rs:1076-1079`; exec gated by `allow_exec_auth` (`local.rs:311-315`) and run only after activation, never while listing (`docs/design/10-…:50-72` in the old repo) | preserve as `http-bearer`, `mtls-client-identity`, `exec-credential-plugin` |
| RTVBP: 60-second proof-bound single-redemption authority binding endpoint, tenant, actor, connection, grant, operation, channel kind, profile, proof key, lease; DPoP presented on the WebSocket upgrade; serving endpoint atomically redeems `(iss, jti)` before accepting bytes | `../connectors/docs/design/05-native-sip-and-rtvbp.md:297-305` | preserve as `session-authority` (issue) and `inbound-verifier` (redeem) |
| `scheme = "signing"` in twilio, slack, stripe; verification over original bytes | `../connectors/providers/{twilio,slack,stripe}.toml`; `docs/design.md:726` | reserve `http-signing` and `inbound-verifier` webhook profile; out of the six areas |
| Grafana data-source proxy and Kubernetes service proxy route adapters | `../connectors/docs/design/08-…:159-176`, `10-…:102-116` | preserve as `mediated-http`, specified in [mediated_route](../../../discovery/mediated_route/v1alpha1/semantics.md) |
| Docker Engine API over unix socket or TLS client certificate | no old source; vendor facts to verify at authoring | `socket-peer`, `mtls-client-identity` |

### 2.1 Reserved capability disposition

`http-signing` is reserved/refused. The historical Twilio, Slack and Stripe
declarations preserve the name for later translation, but this version selects
no canonicalization algorithm, signed-component policy, provider profile or
conformance source. A profile or descriptor cannot select or advertise it, and
an incoming selection is refused before provider use. A future supported binding
requires those independently reviewed semantics; enum membership and historical
provider declarations do not confer support.

## 3. Types

Each capability is a Rust port in the SDK; none is a wire type. The wire only ever sees the capability *name* in descriptors and profiles.

```text
HttpCapability
  request(method, segments, query, headers_allowlisted, body_bounded) -> HttpResponse
  // bound to: connection, profile, admitted credential/configuration/route binding, destination aperture, deadline, body limits
  // variants: http-bearer, http-basic, http-signing, http-anonymous, mediated-http

TransportIdentity
  tls_client_config() -> ClientIdentity            // mtls-client-identity: certificate chain + key handle
  socket_path() -> PathBuf                          // socket-peer: owner-checked unix socket

ExecCredentialPlugin
  materialize(deadline) -> ShortLivedCredential     // exec-credential-plugin: runs the configured helper under the plugin policy

CredentialLease
  acquire(purpose, deadline) -> LeasedMaterial      // sip-credential-lease: bytes released to the protocol library for one establishment
  release(lease)

SessionAuthority
  issue(binding) -> Authority                       // session-authority: one-time, proof-bound, short-lived
InboundVerifier
  verify(presented, proof, original_bytes) -> Verified(binding) | Refused
  redeem(authority_id) -> Redeemed | AlreadyRedeemed
```

Every capability carries the binding it was created for. Credential-bearing bindings additionally contain the validated generation pin and its admission. Anonymous direct and mediated child capabilities use the explicit no-child-credential binding in profile §4.2; the parent's own capability retains its generation pin. Private bindings do not appear in this safe descriptive example:

```json
{ "connection": "conn_…", "profile": "jira.user_oauth", "destination": { "origin": "https://api.atlassian.com", "base_path": "/ex/jira/<cloud_id>" }, "limits": { "deadline_ms": 15000, "body_bytes": 4194304 } }
```

## 4. Rules

- Binding: a capability is constructed by the host for one connection and one profile, with the admitted destination and limits. Provider code cannot change the credential, the destination origin, or the limits through the request it builds (`docs/design.md:590`).
- Placement for credential-bearing capabilities: the host captures immutable material, validates it through the declared admitted auth flow, and pins that exact generation with the evidence used for admission. Immediately before dispatch the capability places the pinned material; it must not resolve a mutable path afresh or silently substitute a newer custody reference. Provider code never sees `http-bearer`/`http-basic` bytes. The pin is host-private and no public serializer or diagnostic exposes its generation or snapshot reference.
- `http-anonymous` is bound only to the explicit anonymous profile and admitted destination/configuration revision. It performs no credential resolution, placement, cookie/ambient authentication or refresh. Provider/user input cannot add Authorization or select another access profile. A 401 is a provider refusal, never a trigger to acquire credentials or retry under another mode. `mediated-http` similarly owns no child credential: it passes only reviewed target-relative traffic to the independently admitted parent's fixed route, never the parent's secret or arbitrary destination.
- Dispatch consistency for credential-bearing capabilities: final dispatch must establish that the pin, evidence and selected connection binding agree and the generation remains current and usable. Known revocation/expiry, stale required checks, a detected unvalidated replacement, or an unresolved authorized rotating refresh refuses even a pin. Successful refresh/replacement publication atomically cuts off old pending admissions; a new admission is required for the new generation. A capability cannot renew authority just by reloading credentials. See [evidence §4](../../evidence/v1alpha1/semantics.md#4-rules).
- Non-material dispatch still requires a current host admission and immutable configuration/route aperture ordered against revocation and binding changes. `socket-peer` validates and pins the admitted local path/peer/transport identity for use; it does not turn socket metadata into CredentialGeneration or anonymous HTTP. Its concrete peer/path race checks and typed transport admission remain UNMAPPED advertisement gates. Anonymous and mediated binding predicates likewise cannot be represented as a fake material-bearing DispatchAdmission.
- Destination: `http-*` capabilities refuse a request whose resolved URL leaves the admitted origin and base path; redirects are not followed (`docs/design.md:590`; current host behavior).
- Business methods: `GET`, `POST`, `PUT`, `PATCH`, `DELETE` with a bounded body; a mutation method is refused when the operation's profile is not `mutation` (`operations` document). Separately admitted auth checks use private purpose-specific capabilities: for example the bounded SSAR POST in [evidence §4.4](../../evidence/v1alpha1/semantics.md#44-exact-authorization-targets-and-fan-out-budget-f08) grants no business POST authority.
- Provider 401 and refresh: a capability name grants neither an exchange nor redispatch. Only the explicitly selected supported [read-refresh-once/v1alpha1 binding](read-refresh-once.md) permits its one complete business-response 401 to trigger one coordinated refresh participation and at most one new-generation redispatch. It retains the original deadline, exact request/authority and consumed permission-call ledger; failed authorization checks do not become refresh triggers. Legacy and unselected reads retain their own terminal no-retry behavior. Separately admitted auth maintenance remains governed by acquisition, with its own effects and no implicit repeat of the business operation. Mutations never redispatch after refresh. Anonymous/static_config modes never acquire/refresh implicitly, and mediated/federated traffic cannot select this first retry binding; parent maintenance never grants a child a silent repeat.
- Leases (`sip-credential-lease`, `exec-credential-plugin`): material is released for one establishment or one helper run, within a deadline, and is not retained by provider code beyond the protocol object that consumes it. The lease records purpose and time, never the value.
- `exec-credential-plugin` runs only when configuration enables it and only inside an admitted operation or explicitly admitted activation/revalidation step, never during listing or description (old rule preserved). One helper output is one captured generation: identity validation and dispatch must consume the same output. Running the helper a second time needs a new generation and admission. Client-certificate capabilities similarly pin one coherent certificate/key pair through transport use. An identity probe needed for replacement belongs to the declared validation step; it is not an implicit expansion of a business operation.
- `session-authority`: issued per admitted session with a bound lifetime (old profile: 60 s, single redemption); the verifier redeems atomically before any media or data byte is accepted; a second presentation is `AlreadyRedeemed`.
- Establishment redemption is necessary but does not authorize an established session indefinitely. Every data path additionally enforces the authenticated live data lease and cutoff policy in [sessions §4.1](../../../sessions/v1alpha1/semantics.md#41-traffic-cutoff-and-teardown-f06). The 60 s token lifetime does not extend the at-most-2 s live lease, grant grace after revocation/expiry, or revive a terminal session.
- `inbound-verifier` verifies over the original received bytes, not a re-serialized copy (`docs/design.md:726`).
- No public serializer or `Debug` on any type that can hold material (`Secret` pattern, `crates/connectors-sdk/src/lib.rs:33-34`).

## 5. Limits

| Concern | Rule |
|---|---|
| Deadlines | inherited from the operation profile (today 15 s provider, 5 s connect) |
| Body | inherited (64 KiB request, 4 MiB response today); mutation bodies bounded by the operation's declared limit |
| Lease lifetime | one establishment; default ceiling 30 s for SIP, plugin deadline 10 s (first-profile defaults) |
| Authority lifetime | 60 s from the old profile as the starting value |

## 6. Conformance scenarios

- Fake transport records the header for `http-bearer` and `http-basic`; provider code has no access to the value (type-level: the capability owns it).
- Request built with an absolute URL or a `..` segment → refused before dispatch.
- Redirect from the fake provider → not followed; `UpstreamProtocol`.
- Legacy or unchanged read profile 401 → one business call and its documented error, with no refresh/redispatch caused by this invocation. Explicitly selected read-refresh-once/v1alpha1 → follow its [complete bounded sequence and refusal matrix](read-refresh-once.md); second 401 is terminal, and new-generation permission checks cannot reuse consumed per-invocation slots. Mutation redispatch remains zero. Anonymous/static_config read 401 never upgrades auth; mediated child never refreshes the parent or silently repeats a forward.
- Lease released twice → second release is a no-op; lease used after deadline → refused.
- Authority redeemed twice → `AlreadyRedeemed`; media bytes before redemption → refused.
- Swap the custody implementation while preserving the exact immutable snapshot semantics → authentication behavior remains compatible. Swapping material is a new generation, never permission to retain old evidence.
- Publish a replacement between admission and dispatch → the old capability refuses; a fresh admitted capability uses only the validated new generation.
- A helper returns account A material during validation and would return B on a second run → dispatch uses the captured A output or refuses; it never runs the helper again under A's evidence.
- Revoke/expire a pinned generation or authorize a rotating refresh without a definitive result → the pin refuses dispatch.

## 7. Compatibility

- `http-bearer` is today's `AuthenticatedHttp` renamed; `get` remains as a convenience.
- [Service compatibility](../../../service/compatibility.md) is authoritative for the binding. Private capability ports do not themselves change wire bytes. Publishing capability names through `Descriptor.auth_profiles` requires the extended codec; only an unchanged configured port and public surface may keep legacy behavior.
- Read retry is a separate observable profile change, not part of the port rename. The proposed [combined native read profile](read-refresh-once.md#1-explicit-selection-and-compatibility) uses the existing singular Operation.profile and a changed projection revision on v1alpha2; it is excluded from legacy projection and unadvertised until actually implemented. No capability, adapter-kind or invocation field is added here.


## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| `AuthenticatedHttp` → `HttpCapability` with methods and body; basic scheme with user half | `crates/connectors-sdk/src/lib.rs:53-56`; `crates/connectors-host/src/http.rs:11-35` |
| TLS client identity and unix-socket transport in the host HTTP client | `crates/connectors-host/src/http.rs` |
| Generation pin and dispatch/publication/revocation ordering | host admission/capability boundary with the shared auth coordinator |
| Lease, plugin, authority, verifier ports | `crates/connectors-sdk` (traits), `crates/connectors-host` (bindings) |
| Mediated-http implementation | host, per [mediated_route](../../../discovery/mediated_route/v1alpha1/semantics.md) |

## 9. ESS entities

| Entity / value | Notes |
|---|---|
| `AuthProfile.capabilities` | value list (profile document) |
| `SessionAuthority` (identity: authority id), lifecycle `issued → redeemed | expired` | relation `for → Session` (sessions document) |
| Leases | not persisted; transient adapter state (`docs/design.md:959`); generation pins are trusted host state |
| `DispatchAdmission` | transient host decision modeled in [credential_evidence.yaml](../../../../ess/domains/credential_evidence.yaml); references one [CredentialGeneration](../../../../ess/domains/credentials.yaml), without inventing a durable lease entity |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Whether provider code may see `http-basic` bytes for libraries that require them | no; the capability places the header |
| SIP lease ceiling | 30 s |
| `http-signing` canonicalization | deferred with the providers that need it |
