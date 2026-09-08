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
  "status": { "state": "ready", "since_unix_ms": 0, "next_action": null },
  "revision": "config or management revision this connection was admitted under",
  "created_unix_ms": 0
}
```

Route variant:

```json
{ "route": { "kind": "via", "parent": "conn_grafana_1", "profile": "grafana-datasource-proxy" } }
```

Status states and the safe `next_action` each may carry:

| `state` | Meaning | `next_action` |
|---|---|---|
| `ready` | evidence says usable (`auth.evidence`) | none |
| `reauthorization_required` | credential invalid, revoked, or refresh uncertain | `reconnect` |
| `insufficient_scope` | valid credential, missing granted scope for enabled operations | `reconnect` with widened request |
| `custody_unavailable` | secret store unreachable | none (operator) |
| `parent_degraded` | `via` route whose parent is not `ready` or whose binding is stale | none (reconcile) |
| `disabled` | present in configuration, not enabled | none |
| `revoked` | terminal; retained for audit and cursor invalidation | none |

Management operations (profile `managed`; all are ordinary `operations/v1alpha1` operations on the adapter, admitted by the host):

| Operation | Effect |
|---|---|
| `connections.list` | safe summaries for the caller's admitted scope |
| `connections.describe` | one connection, safe status and evidence summary |
| `connections.revoke` | mutation: marks revoked, triggers provider revocation via `auth.acquisition` where the profile supports it, invalidates cursors and sessions per their contracts |

Creation and repair are not operations here; they are `auth.acquisition` flows whose completion yields a connection.

Errors: base `ErrorCode` plus `connection_not_ready` (state is not `ready`; message carries the state name only) and `route_unavailable` (parent not ready).

## 4. Rules

- Identity: the connection ref is stable across reauthorization, refresh, token rotation, compatible adapter upgrade, and process restart. A repair whose returned external identity differs from the bound one is refused unless an explicitly authorized reassignment flow says otherwise (`docs/design.md:547`).
- Selection: an invocation selects a connection by ref or, for the `configured` profile, implicitly. A request cannot select a connection outside the caller's admitted scope; the receiver resolves the scope from verified caller context, never from a request field (`docs/design.md:383`).
- Separation of facts: implemented, enabled, ready, authorized remain separately answerable (`docs/design.md:350`). `status.state` answers only "ready"; authorization is per invocation.
- Value freedom: no response, log, or error carries a credential, secret reference, provider base URL of a private deployment, proxy path, or parent resource binding.
- Route: fixed at creation. One mediated hop. A connection cannot be its own parent. Parent revocation or degradation moves every child to `parent_degraded`; children never fall back to a direct route (`docs/design/08:146-148` in the old repo: no accidental direct-egress fallback).
- Multiplicity: one adapter instance holds many connections; one auth profile may back many connections (many users); one external identity may back at most one non-revoked connection per instance and profile.
- Concurrency: status transitions are serialized per connection; a repair completing while a revoke is in flight yields `revoked` (revocation wins).

## 5. Ordering, limits

| Concern | Rule |
|---|---|
| List paging | `connections.list` uses `datasource.records` paging semantics |
| Bound | maximum connections per instance is configuration (first-profile default 1,000, to be measured) |
| Status freshness | `status` carries `since_unix_ms`; a describe never triggers a provider call (`docs/design.md:965`, no credential resolution for metadata) |

## 6. Conformance scenarios

Positive:

- Reauthorize a connection through a fake acquisition flow; ref unchanged; cursors bound to it remain valid.
- Two users connect the same profile on one instance; two refs; each invocation uses its own credential (fake transport records the header).
- Replace the bound token file for a `configured` connection; ref and revision unchanged; next invocation uses the new bytes.

Adversarial (`docs/design.md:985`, repair without identity drift):

- Repair returns a different external identity → refused; connection stays `reauthorization_required`.
- Request names a connection outside admitted scope → `Forbidden`; no provider dispatch.
- Parent revoked → child `parent_degraded`; child invocation fails `route_unavailable`; fake transport sees no direct dial.
- Describe output serialized and grepped for the fixture's secret bytes and base URL → no match.

## 7. Compatibility

- The `configured` profile is what every current adapter does; making it explicit changes no wire bytes. Descriptors gain an optional `connections` capability flag when `managed` is provided.
- Old `connector-connection v0alpha1` DTOs are not carried; a compatibility facade would map `search`/`describe` onto `connections.list`/`describe` and drop channel fields.

## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| Connection metadata store port: create, read, update status, list by scope; CAS on revision | new host module (design: host metadata store owns the active credential reference, `docs/design.md:576`) |
| `Adapter::invoke` receives a resolved connection context rather than the ambient single credential | `crates/connectors-sdk/src/lib.rs:18-31` |
| `ScopedHttp` constructed per connection, not per instance | `crates/connectors-host/src/http.rs:30-36` |
| Cursor binding includes connection ref | `crates/connectors-sdk/src/lib.rs:161` (`Cursors`) |

## 9. ESS entities

| Entity | Notes |
|---|---|
| `Connection` (identity: connection ref) with lifecycle `pending → ready ↔ reauthorization_required/insufficient_scope/custody_unavailable/parent_degraded → revoked` | relations: `instance → ServiceConfiguration` (one), `profile → AuthProfile` (one), `parent → Connection` (zero or one), `active_credential → CredentialSet` (zero or one, see custody) |
| `ExternalIdentity` | value embedded in `Connection` |
| Tenant / principal ownership of a connection | UNMAPPED, consistent with `ess/domains/declarations.yaml` |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Whether `connections.*` are adapter operations or host-only management routes | adapter operations, host-admitted, so federation can forward them like any operation |
| External identity disclosure | safe display label plus provider account id; no email unless the provider's id is the email (Atlassian API token: the user half is the email, `../connectors/providers/jira.toml`, `user_env`) — then it is `external_identity.id` and never logged |
| Maximum connections per instance | 1,000, to be measured |
