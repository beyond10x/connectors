# resource_discovery/v1alpha1

- **Status:** proposed, not implemented.
- **Family:** discovery. Siblings: [mediated_route](../../mediated_route/v1alpha1/semantics.md); `endpoint_discovery/v1alpha1` and `host_discovery/v1alpha1` in [service v1alpha1](../../../service/v1alpha1/semantics.md).
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `resource_discovery/v1alpha1` |
| Profiles | `grafana-datasources`, `kubernetes-service-targets` |
| Relation to `endpoint_discovery` | same observation discipline (no dial, no credential, candidates not grants); differs in that the observed resource has an opaque locator rather than an address and port, and in that a recognized observation names a target adapter that could be reached *through* the source |

Three discoveries are distinct: service, contract, resource (`docs/design.md:319-323`). This contract is resource discovery for provider objects whose address is either hidden by the provider (a Grafana data source's backend origin is never exposed) or not routable from the observer (a Kubernetes Service). An observation carries a stable source identity, an observed resource reference, a type/profile, owner scope, an address *or opaque locator*, reachability context, observation time/revision, and known authentication requirements without values (`docs/design.md:468`).

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| Grafana `[[discoveries]]`: `grafana-data-sources` from `grafana-datasources-list`; mappings `observed_type` prometheus/loki/alertmanager → `target_provider` with `route_adapter = grafana_datasource_proxy_v1` | `../connectors/providers/grafana.toml`, discoveries block | preserve as profile `grafana-datasources` with a `targets` mapping table |
| Observation carries only safe normalized metadata: identity, declaration identity, source Connection, observed type, bounded title, evidence generation and digest, plus a hidden resource binding; unknown types observable but yield no candidate | `../connectors/docs/design/08-discovery-observations-and-mediated-connections.md:105-121` | preserve |
| Evidence is generation-bound; refresh replaces the set atomically; missing resource, changed type, changed parent generation, or withdrawn mapping makes candidates stale and degrades materialized children | same, lines 116-121 | preserve |
| Kubernetes core/v1 Service recognizer by name/labels: `grafana`, `prometheus`, `loki`, `alertmanager` → target provider; type `kubernetes_service`; label namespace/name; private binding is the Service identity; SSAR before each list; at most `resource_limit` objects; no Secrets, ConfigMaps, env, EndpointSlice addresses, external URLs, scans | `../connectors/docs/design/10-local-kubernetes-context-and-resource-discovery.md:73-100` | preserve as profile `kubernetes-service-targets` |
| Argo CD recognized, stops at the observation | same document, amendment 2026-08-20 | preserve the principle: a recognizer may name a target with no route |
| Hosted Grafana: closed data-source allowlist by provider type plus UID SHA-256; missing, renamed, or type-changed sources become degraded and never widen to an untyped proxy | old design 08, amendment 2026-08-17 | preserve as configuration of the profile |
| Discovery never dials, authenticates to, or materializes the observed endpoint | `contracts/service/v1alpha1/semantics.md`, Kubernetes paragraph | preserve |

## 3. Types

Operation `resources.observe` (paged like `datasource.records`):

```json
{ "profile": "grafana-datasources", "limit": 100, "cursor": null }
```

```json
{
  "items": [
    {
      "id": "obs_…",
      "source_connection": "conn_grafana_1",
      "observed_type": "prometheus",
      "title": "prod-prometheus",
      "locator": { "kind": "opaque", "digest": "sha256:…" },
      "candidate": { "target_adapter": "prometheus", "route_profile": "grafana-datasource-proxy", "confidence": "declared" },
      "auth_requirement": { "kind": "inherited_from_source" },
      "reachability": "via_source_only",
      "generation": 41,
      "observed_at_unix_ms": 0
    }
  ],
  "next_cursor": null,
  "complete": true,
  "generation": 41,
  "provenance": { "instance": "…", "resource": "grafana:datasources", "observed_at_unix_ms": 0, "source_revision": "41" }
}
```

| Field | Rule |
|---|---|
| `locator.kind` | `opaque` (digest of the provider identity; the identity itself stays private), `address` (reserved; `endpoint_discovery` covers it today) |
| `candidate` | present only for recognized `observed_type` values from the profile's mapping table; `confidence` is `declared` (provider states the type) or `inferred` (name/label heuristics, Kubernetes) |
| `auth_requirement.kind` | `inherited_from_source` (mediated route uses the source connection's credential), `separate` (a direct connection needs its own), `unknown` |
| `reachability` | `via_source_only`, `direct_possible`, `unknown` |
| `generation` | monotonic per source connection; the whole set shares one generation |

Withdrawal requires a complete authoritative observation over the represented scope: the [authorization-budget rule](../../../auth/evidence/v1alpha1/semantics.md#44-exact-authorization-targets-and-fan-out-budget-f08) forbids publishing a denied/unavailable/incomplete scan as a complete replacing generation or withdrawing unseen resources from it. Finer coverage/generation semantics remain with the discovery-coverage story. For a complete replacing generation, an absent observation is withdrawn; `resources.observe` at an older generation returns `StaleCursor`.

Errors: base codes; `Forbidden` when the profile is not enabled in configuration.

## 4. Rules

- No effect: observing never dials the observed resource, never resolves a credential for it, never downloads or executes anything (`docs/design.md:483`).
- Source authentication is a candidate credential-placement fact, never inherited host authority. inherited_from_source may select only the explicit via_parent profile after materialization validates the supported fixed parent route and child/parent admissions; it grants no arbitrary target authentication or direct fallback. Separate/unknown requirements do not become anonymous because no credential was found.
- Candidates are not connections: a candidate says "an adapter of kind X could be bound through this source". Materialization is an explicit, host-owned configuration step (`docs/design.md:472-481`) that creates an `auth.connection` with `route.kind = via` and a `route.mediated_http` binding.
- Recognition is closed: the mapping table is part of the adapter specification; an unknown type is observable with `candidate: null`; nothing falls through to a generic proxy.
- Identity stability: `id` is stable across generations while the private binding (Grafana UID, Kubernetes Service UID) is unchanged; a changed type or a new provider identity yields a new `id`, and every child bound to the old one is degraded (`auth.connection` state `parent_degraded`).
- Atomic refresh: a new generation replaces the set; readers never see a mix.
- Private data: provider UIDs, backend URLs, proxy paths, secure JSON, headers, and parent credentials are absent from observations, descriptors, logs, and audit (old design 08 §4 rule preserved).
- Kubernetes profile: only core/v1 Services; SSAR per namespace before list; denied namespaces skipped and reported; recognition by name and labels only; `confidence: inferred`.
- Grafana profile: recognition from the provider's `type` field; `confidence: declared`; a configured allowlist (type plus digest) narrows which observations may become candidates.

## 5. Limits

| Concern | Rule |
|---|---|
| Observations per generation | bounded by configuration (`resource_limit`, first-profile default 500, the old Grafana projection bound) |
| Page | 1–100 as `datasource.records` |
| Refresh | on demand and on a configured interval; no watch |

## 6. Conformance scenarios (`docs/design.md:989`)

- Fixture lists 4 Grafana data sources of types prometheus, loki, alertmanager, `unknownplugin` → 4 observations, 3 candidates, 1 `candidate: null`; fixture sees exactly one request; no request to any data-source backend.
- Fixture renames a data source and changes its type between generations → old `id` withdrawn, new `id` issued, generation incremented.
- Kubernetes fixture denies SSAR in namespace `b` → observations from `a` only, `b` reported denied.
- Serialize all outputs and logs; grep for the fixture's UID and backend URL → no match.
- Cursor from generation 40 used at generation 41 → `StaleCursor`.
- No materialization occurs from observation alone: the fake host's connection store is unchanged after observe.

## 7. Compatibility
- [Service compatibility](../../../service/compatibility.md) is authoritative for the binding. Discovery observations and page generation/coverage use a selected new payload schema. Existing endpoint discovery and the closed Page reader do not gain fields automatically.


## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| `ResourceObservation` type | `crates/connectors-contracts/src/lib.rs` |
| Host materialization step: candidate + operator configuration → `auth.connection` (`via`) + mediated route binding; refuses when the profile is not `via_source_only`-capable | new host module |
| Private binding store (observation id → provider identity) scoped to the source adapter | adapter-transient state backed by the host metadata store |

## 9. ESS entities

| Entity | Notes |
|---|---|
| `ResourceObservation` (identity: observation id), lifecycle `observed → withdrawn` | relations: `source → Connection` (one), `candidate_target → AdapterSpecification` (zero or one) |
| `Materialization` | not an entity here; it is the creation of a `Connection` with a parent |
| Multiplicity: one resource may have several observations (from different sources); one observation may back several child connections | recorded as cardinality `many` on `Connection.parent` reverse; ownership UNMAPPED (`docs/design.md:487`) |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Whether `endpoint_discovery` becomes a profile of this contract | later; both stay until a third discovery profile exists |
| Kubernetes recognizer markers | the old four plus `argocd` as observation-only |
| Refresh interval | configuration; default 300 s |
