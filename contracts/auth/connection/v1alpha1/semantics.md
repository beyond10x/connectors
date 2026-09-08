# auth.connection/v1alpha1

- **Status:** proposed, not implemented.
- **Family:** auth. Sibling documents: [profile](../../profile/v1alpha1/semantics.md), [acquisition](../../acquisition/v1alpha1/semantics.md), [custody](../../custody/v1alpha1/semantics.md), [capability](../../capability/v1alpha1/semantics.md), [evidence](../../evidence/v1alpha1/semantics.md).
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `auth.connection/v1alpha1` |
| Profiles | `configured` (one connection bound at startup by configuration, as today), `managed` (connections created, repaired and revoked at runtime through management operations) |
| Vocabulary | connection ref, external identity, scope (`tenant`, `principal`), actor (`app`, `user`), route (`direct`, `via`), status |

A connection is a stable binding to authorized external access (`docs/design.md:143`). It is the thing operations select, credentials attach to, cursors bind to, and routes hang off. Today each adapter instance has exactly one implicit connection fixed by configuration (`contracts/service/v1alpha1/semantics.md`, Auth and configuration). This contract makes the connection explicit so that one Atlassian service can hold several users' OAuth identities, and so that a Prometheus connection can declare it is reached via a Grafana connection.

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| `b10x.connector-connection.v0alpha1`: value-free control contract; no field can carry a credential, credential address, provider URL, or transport ticket | `../connectors/contracts/connector-connection/v0alpha1/README.md:3-11` | preserve |
| `search`/`describe` return lifecycle, initiation, route, channel summaries; paired `scope` (`tenant`/`principal`) and `actor` (`app`/`user`) and admitted `auth_profile` | same README, paragraph 4 | preserve `scope`, `actor`, `auth_profile`; channels move to the deferred `events` family |
| Connect sessions (`connect_session_create`/`status`) | same README, paragraph 3 | move to `auth.acquisition`; this contract only exposes the resulting connection |
| A connection has exactly one route for its lifetime: `direct` or `via_connection { parent, resource_binding, adapter }`; one mediated hop; parent grant not inherited | `../connectors/docs/design/08-discovery-observations-and-mediated-connections.md:122-160` | preserve; `resource_binding` stays private; `adapter` becomes the `route.mediated_http` profile name |
| Connections remain stable across reauthorization; multiple accounts per provider | `docs/design.md:91` | preserve |
| Current per-instance implicit connection, token rotation by replacing the bound file | `contracts/service/v1alpha1/semantics.md`, Auth and configuration | preserve as the `configured` profile |
| Old class hierarchy catalog → integration → connection | `docs/design.md:151` | remove; a connection references an adapter instance and an auth profile, nothing above |

## 3. Types

```json
{
  "connection": "conn_…",
  "instance": "engineering-atlassian",
  "auth_profile": "jira.user_oauth",
  "scope": "principal",
  "actor": "user",
  "external_identity": { "kind": "atlassian_account", "display": "safe label", "id": "opaque or provider account id" },
  "route": { "kind": "direct" },
  "status": { "state": "ready", "since_unix_ms": 0, "observed_unix_ms": 0, "valid_until_unix_ms": 0, "next_action": null },
  "revision": "config or management revision this connection was admitted under",
  "created_unix_ms": 0
}
```

Route variant:

```json
{ "route": { "kind": "via", "parent": "conn_grafana_1", "profile": "grafana-datasource-proxy" } }
```

Status is a connection-wide viability observation, not operation authorization. Its closed state vocabulary is below; §4.1 defines the precedence and evidence requirements.

| `state` | Meaning | `next_action` |
|---|---|---|
| `ready` | published binding and fresh required connection-wide checks permit consideration of business use | null |
| `pending` | a real connection record awaits publication/activation or required baseline revalidation | null during acquisition; `revalidate` when an existing binding needs admitted validation |
| `reauthorization_required` | established credential is positively missing/invalid/revoked, violates the profile minimum, or rotating refresh left it possibly consumed | `reconnect` |
| `custody_unavailable` | required custody dependency is unavailable/unknown; this does not prove invalid credentials | null |
| `parent_degraded` | required mediated parent/binding is unavailable, not viable or stale | null |
| `disabled` | local connection enablement is false | null |
| `revoked` | locally revoked terminal binding; evidence/parent changes cannot restore it | null |

`insufficient_scope` is an acquisition/operation refusal, **not a global state derived from all enabled operations**. Missing a write grant does not invalidate a connection whose baseline and read requirements hold. `external_identity` is required-null before a pending record has an established binding and throughout an explicit anonymous/parent-authenticated child binding; never invent an observed account. An acquisition may remain pending without creating a public Connection at all. A repair retains the established identity even if its replacement candidate is unvalidated.

Management operations (profile `managed`; **host-owned and separately admitted**, as specified in [the management boundary](../../management.md); selected extended operation envelopes do not make them adapter business operations):

| Operation | Effect |
|---|---|
| `connections.list` | safe summaries for the caller's admitted scope |
| `connections.describe` | one connection, safe status and evidence summary |
| `connections.revoke` | host mutation: commits local terminal cutoff independently of provider readiness; separately reports any supported/admitted provider revocation, invalidates cursors and sessions per their contracts |

Managed credential creation and repair use `auth.acquisition` flows whose completion yields a connection. Deployment-supplied static_config and explicit no-child-credential bindings use separately admitted configuration activation/materialization, without inventing an acquisition session ([acquisition §4.0](../../acquisition/v1alpha1/semantics.md#40-acquisition-paths-and-required-declarations)).

Business errors use the existing extended set: `connection_not_ready` for globally non-ready non-route conditions or unavailable required operation validation, `route_unavailable` for a degraded required parent, and operation-local `insufficient_scope` for known missing required grants. Receiver grant-policy denial is `not_granted`; adapter/connection/resource restrictions and provider permission denial use `forbidden`; unavailable provider permission evidence remains `unavailable`. Disclosure admission precedes safe state/evidence detail. Management has its own prerequisites and is not refused solely by this business-readiness gate.

## 4. Rules

- Identity: the connection ref is stable across same-identity reauthorization, refresh, token rotation, compatible adapter upgrade, and process restart. Identity equality means `(kind, stable subject)` within the same provider authority, instance and auth profile; display labels are not identity. A repair or configured replacement whose observed identity differs from the bound one is refused unless an explicitly admitted reassignment flow says otherwise (`docs/design.md:547`). File overwrite and refresh are never reassignment. Before a separately authorized reassignment can publish, old evidence, admissions, cursors and sessions must be invalidated; profiles without such a flow refuse reassignment.
- Selection: an invocation selects a connection by ref or, for the `configured` profile, implicitly. A request cannot select a connection outside the caller's admitted scope; the receiver resolves the scope from verified caller context, never from a request field (`docs/design.md:383`).
- Separation of facts: implemented, enabled, ready, authorized remain separately answerable (`docs/design.md:350`). `status.state` reports connection-wide viability under §4.1; eligibility and authorization are per invocation.
- Value freedom: no response, log, or error carries a credential, secret reference, provider base URL of a private deployment, proxy path, or parent resource binding.
- Route: fixed at creation. One mediated hop. A connection cannot be its own parent. Parent revocation or degradation makes each otherwise enabled, non-revoked child ineligible through `parent_degraded` under §4.1; children never fall back to a direct route (`docs/design/08:146-148` in the old repo: no accidental direct-egress fallback).
- Multiplicity: one adapter instance holds many connections; one auth profile may back many connections (many users); one established external identity may back at most one non-revoked connection per instance and profile. Required-null identity in explicit no-child-credential profiles is absence, never one shared provider account or a uniqueness key. Their host-qualified connection refs remain distinct; route kind, parent, fixed target, access mode or host-ownership changes require a new independently admitted connection, with no reassignment flow selected here. Admitted configuration/evidence changes may advance revision only while preserving that fixed binding; tenant-header/policy changes invalidate affected evidence and continuations, never silently repurpose the ref. The named persistence owners and atomic groups are consolidated in design §31.
- Concurrency: status transitions are serialized per connection; a repair completing while a revoke is in flight yields `revoked` (revocation wins). Publication of a validated generation atomically cuts off old pending dispatch admissions. Unresolved rotating refresh also withholds dispatch on possibly consumed material. Final dispatch admission participates in the same ordering; a pin does not bypass revocation, refresh uncertainty or publication.
- A new observation or parent credential generation invalidates mediated route evidence, but does not itself change the fixed target. Explicit same-target revalidation may advance the route evidence revision under [mediated route §4.1](../../../discovery/mediated_route/v1alpha1/semantics.md#41-observation-continuity-and-route-revalidation); stale history never authorizes it, and parent/target/mode/owner changes still require a new connection.
- Revision identity: public connection `revision` records effect-relevant configuration/management meaning. The host also retains a separate private auth publication fence for generation publication, revocation and pending admission ordering. A same-identity refresh/evidence recollection or same-target route revalidation alone advances private evidence/fences as needed, not the semantic revision used by F02/F03 fingerprints; changed target/identity/meaning still requires a new semantic binding/revision under the fixed-connection rules.
- Credential identity: connection `revision` and credential generation are distinct. Evidence is bound to a host-private immutable material generation, not a path, store version or revision. Detecting a replacement blocks dispatch until its identity and required checks are validated in an explicitly admitted auth-validation step. If that step is unavailable or not admitted, return `connection_not_ready`; an ordinary invocation must not invent an identity probe. See [evidence §§4.1–4.3](../../evidence/v1alpha1/semantics.md#41-credential-generation-and-identity).


### 4.1 Connection viability and operation eligibility

Read the authoritative metadata under current disclosure admission first. Metadata unavailable gives `unavailable`, not an invented pending/ready/reconnect state. Local revocation differs from provider credential revocation: the first is terminal; the second can be repaired on a non-revoked binding. `enabled` is an administrative fact independent of all evidence. Status is a reduction of these facts, **not a persisted lifecycle that transitions through every provider observation**.

Apply the following rows in order, first matching row wins. Every check used for success must match the current applicable F05 material generation or explicit no-child-credential configuration/route binding ([profile §4.2](../../profile/v1alpha1/semantics.md#42-explicit-access-bindings-without-child-credentials)), and be unexpired. The latter publishes a validated binding with applicable checks, not fake material or a provider account; absent credential/custody/scope dependencies supply no failure and no authority. A negative observation from obsolete material does not poison a replacement either. Unknown and unavailable never become positive evidence.

| Priority | Current authoritative condition | Global state |
|---|---|---|
| 1 | local terminal revocation | `revoked` |
| 2 | connection disabled | `disabled` |
| 3 | established active credential positively missing/invalid/revoked or below mandatory profile minimum; detected identity mismatch; unresolved consumed rotating source | `reauthorization_required` |
| 4 | required custody unavailable or its availability unknown | `custody_unavailable` |
| 5 | required parent/binding not currently viable or route stale/unknown | `parent_degraded` |
| 6 | no validated publication yet, or another required connection-wide check is stale, unknown, not_run or unsatisfied | `pending` |
| 7 | validated publication and all required global checks current | `ready` |

An initial flow with no established material uses pending, not “missing old credential.” A failed independent repair does not overwrite valid old evidence or state; concurrent expiry/revocation/refresh consumption still applies. Parent failures cannot override local revoked/disabled. A profile explicitly lacking a dependency omits that dependency's readiness test; missing material for a credential-requiring profile never selects anonymous access.

Global scope checks use the profile's **minimum publication grants**, not a union of scopes of enabled operations. Requested optional grants, exact resource permissions and operation-only verification belong to eligibility. Supported versus required checks are selected by [profile §4.1](../../profile/v1alpha1/semantics.md#41-baseline-and-operation-requirements). A successful optional probe cannot compensate for failed mandatory identity/credential checks.

For one business request, current host admission and operation enablement come first, without revealing a forbidden connection's state. Then require global ready and either one `requires_auth` alternative matching that connection's exact profile/purpose with all its required granted scopes, or the exact explicitly configured binding when this member is absent under service compatibility §4. Absence is never an anonymous/missing-credential fallback or permission to select another binding. Both branches require the operation's current checks for the exact resource/input and generation. Never combine alternatives across profiles/connections. Known missing scope is `insufficient_scope`; unknown grants require validation (`connection_not_ready`), not an invented missing-scope list. Permission denial is `forbidden`; unavailable/stale permission evidence must be obtained under the existing admitted budget or refuses `unavailable`. An unsatisfied operation-only verification refuses `connection_not_ready` for that request without changing global state. A check explicitly not required contributes no failure and no permission. Receiver grant-policy denial is `not_granted`; a known operation/configuration or adapter/connection/resource restriction is `forbidden`. Host policy outage refuses `unavailable`, not implicit local policy.

`connections.describe` and invocation use these same rules for the same scope of question. Unqualified describe reports only global viability. A separately admitted exact-operation eligibility query, if a binding exposes one, must name that operation/target and cannot grant dispatch authority. Describing never runs a provider probe or resolves credential material. Per-check expiry and generation remain authoritative; the observation's `since_unix_ms` is when that displayed state was first observed, not a freshness guarantee. `observed_unix_ms` and `valid_until_unix_ms` bound the snapshot, with validity no later than its earliest required evidence deadline and at most 300 s from observation. An already expired/cached snapshot is historical and cannot establish current readiness; refresh its metadata reduction or report pending when baseline validation is unavailable. An unavailable trustworthy time observation cannot establish fresh positive evidence. Mutations retain the stricter existing 60 s evidence bound and final F05 dispatch check.

Inspection, status, repair orchestration and local revocation use [management admission](../../management.md) independently of business viability. F03 approval preparation remains an admitted metadata read and does not promise that a currently unready connection can execute the proposed mutation.

## 5. Ordering, limits

| Concern | Rule |
|---|---|
| List paging | `connections.list` uses `datasource.records` paging semantics |
| Bound | maximum connections per instance is configuration (first-profile default 1,000, to be measured) |
| Status freshness | `status` carries separate since/observation/validity times under §4.1; a describe never triggers a provider call (`docs/design.md:965`, no credential resolution for metadata) |

## 6. Conformance scenarios

Positive:

- Reauthorize a connection through a fake acquisition flow; ref unchanged; continuation validity follows each cursor/session contract and its current authority, not ref stability alone.
- Two users connect the same profile on one instance; two refs; each invocation uses its own credential (fake transport records the header).
- Replace the bound token file for a `configured` connection with same-account material; ref and configuration revision may stay unchanged, but the material has a new private generation. The next invocation uses those bytes only after admitted replacement validation and publication; pending old-generation admissions are invalidated. Without validation, invocation refuses.
- Replace it with another account's token → ordinary replacement refuses; cached identity evidence for the old generation is unusable even within its age bound.

Adversarial (`docs/design.md:985`, repair without identity drift):

- Repair returns a different external identity → refused; prior binding/state is preserved subject to independently current revocation, expiry and refresh facts.
- The file changes between admission and dispatch → a pin never silently substitutes bytes. If replacement has been detected or published, old admission refuses and a new validated admission is required; if the host has not observed a change, a still-current pin can dispatch only its original validated material.
- Refresh is authorized and unresolved, or a credential is known revoked/expired → even a previously admitted pin refuses.
- Request names a connection outside admitted scope → `Forbidden`; no provider dispatch.
- Parent revoked → child `parent_degraded`; child invocation fails `route_unavailable`; fake transport sees no direct dial.
- Describe output serialized and grepped for the fixture's secret bytes and base URL → no match.

## 7. Compatibility

- Old `connector-connection v0alpha1` DTOs are not carried; a compatibility facade would map `search`/`describe` onto `connections.list`/`describe` and drop channel fields.
- [Service compatibility](../../../service/compatibility.md) is authoritative for the binding. Only unchanged implicit configured behavior is legacy-compatible. The managed `connections` descriptor flag, invocation connection selector and new errors require the extended codec and independently admitted management operations.


## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| ConnectionAuthorityPort: create/read/list by scope, update authoritative enablement/revocation/binding facts, derive status; CAS on the private publication fence plus generation publication/admission invalidation | new host module (design: host metadata store owns the active credential reference, `docs/design.md:576`) |
| `Adapter::invoke` receives a resolved connection context rather than the ambient single credential | `crates/connectors-sdk/src/lib.rs:18-31` |
| `ScopedHttp` constructed per connection, not per instance | `crates/connectors-host/src/http.rs:30-36` |
| Cursor binding includes connection ref | `crates/connectors-sdk/src/lib.rs:161` (`Cursors`) |

## 9. ESS entities

| Entity | Notes |
|---|---|
| Connection viability and operation eligibility values | Modeled in [connection_admission.yaml](../../../../ess/domains/connection_admission.yaml); the seven-state status is the §4.1 reduction, not an ESS entity lifecycle. Local revocation is terminal and enablement is orthogonal. |
| Persistent `Connection` | Proposed, not yet declared in ESS. Instance/profile/optional parent/active material relations require its persistence model; no table here asserts an implemented lifecycle or custody ownership. |
| `ExternalIdentity` | value embedded in `Connection`; shared typed kind/subject in [credentials.yaml](../../../../ess/domains/credentials.yaml), scoped by provider authority |
| `EvidenceSnapshot` | generation-bound value embedded in `Connection`, modeled in [credential_evidence.yaml](../../../../ess/domains/credential_evidence.yaml); no independent evidence-store ownership |
| Tenant / principal ownership of a connection | UNMAPPED, consistent with `ess/domains/declarations.yaml` |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Owner and transport of `connections.*` | host management dispatched by the coordinator; a selected extended operations envelope is permitted, protected completion is separate; see [management](../../management.md) |
| External identity disclosure | safe display label plus provider account id; no email unless the provider's id is the email (Atlassian API token: the user half is the email, `../connectors/providers/jira.toml`, `user_env`) — then it is `external_identity.id` and never logged |
| Maximum connections per instance | 1,000, to be measured |

Persistence ownership is consolidated in [design §31](../../../../docs/design.md#31-host-persistence-ownership-and-atomicity). ConnectionAuthorityPort owns authoritative binding facts; MaterialAdmissionPort and RefreshCoordinatorPort share its required publication/revocation ordering. This inventory does not supply a backend or execute its atomicity predicates.
