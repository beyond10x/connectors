# route.mediated_http/v1alpha1

- **Status:** proposed, not implemented.
- **Family:** discovery. Siblings: [resources](../../resources/v1alpha1/semantics.md). Consumes [auth.capability](../../../auth/capability/v1alpha1/semantics.md) (`mediated-http`) and [auth.connection](../../../auth/connection/v1alpha1/semantics.md) (`route.kind = via`).
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `route.mediated_http/v1alpha1` |
| Profiles | `grafana-datasource-proxy`, `kubernetes-service-proxy` |
| Provided by | the *parent* adapter (Grafana, Kubernetes) as a capability |
| Consumed by | the host, which constructs the `mediated-http` capability handed to a *child* adapter (Loki, Prometheus, Alertmanager); the child adapter does not know it is mediated |

A private endpoint may be unreachable from where the client runs; the design allows an adapter placed in the reachable network or a separately admitted mediated-route capability, and states that federation of discovery results does not make private addresses reachable (`docs/design.md:483`). This contract is that capability: the parent forwards a target-relative HTTP request through its own admitted connection to one resource it observed. It is distinct from host federation, which forwards *operations* between services (`crates/connectors-host/src/federation.rs`); a mediated route carries the child's *provider* traffic.

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| Route `via_connection { parent_connection, resource_binding, adapter }`; child governed by the target provider contract; adapter changes transport only; unavailable adapter refuses before dispatch, no direct-egress fallback | `../connectors/docs/design/08-discovery-observations-and-mediated-connections.md:122-149` | preserve |
| Parent and child share placement; route uses parent's admitted origin, credential custody, destination aperture, opaque binding; child needs its own grant; parent grant not inherited; child grant does not license arbitrary Grafana operations or proxy paths | same, lines 150-160 | preserve |
| Grafana execution boundary: seal the UID as the opaque binding; on child invocation re-resolve it and prefix only the reviewed target-relative path with Grafana's data-source proxy route; refuse arbitrary suffixes, absolute URLs, redirects outside the aperture, unknown plugin types, missing targets, stale evidence, cross-placement parents, nested parents; degrade dependants on parent revocation or mapping change; generic `grafana.proxy` out of model | same, lines 161-176 | preserve |
| Kubernetes `kubernetes_service_proxy_v1`: fixed Service and port; input cannot choose namespace, Service, Pod, port, or suffix; resolves only reviewed Prometheus/Loki/Alertmanager operations; requires `get` on `services/proxy` for that namespace and Service at invocation; sends target-relative GET through the API server; no fallback | `../connectors/docs/design/10-local-kubernetes-context-and-resource-discovery.md:102-116` | preserve |
| One mediated hop; personal-local does not build Kubernetes → Grafana → Prometheus | same, line 113-116; old design 08 line 136-138 | preserve |
| Audit joins child connection and grant, parent generation, evidence generation and digest, target operation, route adapter, final admitted destination subject; never the hidden binding or credentials | old design 08, lines 155-160 | preserve |

## 3. Types

Parent-side capability declaration (in the parent adapter specification):

```json
{ "provides": [ { "contract": "route.mediated_http/v1alpha1", "profile": "grafana-datasource-proxy", "targets": ["prometheus", "loki", "alertmanager"] } ] }
```

Host-side route binding (configuration created by materialization, `resource_discovery`):

```json
{ "child_connection": "conn_prom_7", "parent_connection": "conn_grafana_1", "observation": "obs_…", "profile": "grafana-datasource-proxy", "generation": 41, "target_adapter": "prometheus" }
```

Parent port (Rust, host-internal; the wire is the parent's `operations` surface):

```text
forward(binding, method, target_relative_segments, query, headers_allowlisted, body_bounded) -> HttpResponse
```

The child receives an ordinary `HttpCapability` (`auth.capability`) whose implementation calls `forward`. Its `destination` is the *target-relative root*, not a URL:

```json
{ "connection": "conn_prom_7", "profile": "prometheus.none", "destination": { "kind": "mediated", "route": "grafana-datasource-proxy", "parent": "conn_grafana_1" } }
```

Errors returned to the child: base codes plus `route_unavailable` (parent not ready, observation withdrawn, generation mismatch) and `route_refused` (suffix, method, or target outside the reviewed set). The child maps them like any dependency failure; it never retries through a different route.

## 4. Rules

- Transport only: the route changes how bytes reach the target. The child's contract, profiles, operations, descriptor, and result semantics are identical to a direct connection. Conformance runs the same suite direct and mediated (`docs/design.md:1035`, section 21.3).
- Fixed binding: the target is fixed at materialization (observation id + generation). Request input cannot choose the parent, the resource, a path prefix, or a port.
- Path discipline: the parent prefixes its proxy route (Grafana data-source proxy for the sealed UID; Kubernetes API-server `services/proxy` for the fixed namespace, Service, port) to individually encoded target-relative segments supplied by the child's capability. Absolute URLs, `..`, empty segments, and query keys outside the child's declared set are refused before dispatch.
- Methods: the profile declares allowed methods; first profiles allow `GET` only (both old adapters were read-only).
- Authority: the parent's credential authenticates the hop. The child connection still requires its own admission and evidence; the parent's admission is not inherited. The route does not license any parent operation.
- Kubernetes profile: before each forward, `permission_check` (`auth.evidence`) for `get` on `services/proxy` in the exact namespace and Service; a denial is `route_refused`.
- Grafana profile: the parent re-resolves the sealed UID for the current generation; a changed type or missing source is `route_unavailable` and degrades the child.
- Depth: exactly one hop. A parent connection that is itself `via` cannot provide a route (`route_refused` at materialization).
- Placement: parent and child run in the same host process or composition; a route is never exposed as a remote capability to another host (that would be federation of provider traffic; out of model).
- Redirects from the parent's provider are not followed; responses are passed through with the parent's response-size bound.
- Audit per forward: child connection, parent connection and generation, observation id, target operation, profile, final destination subject (safe label); never the UID, URL, or credential.

## 5. Limits

| Concern | Rule |
|---|---|
| Deadline | the child's provider deadline; the parent hop consumes part of it |
| Body | child response bound (4 MiB today) applies after the hop |
| Concurrency | counted against the parent's concurrency limit as well as the child's |

## 6. Conformance scenarios (`docs/design.md:989-991`)

- Same Prometheus fixture reached direct and via a fake Grafana parent → byte-identical child results and errors.
- Child capability asked for `../api/admin` or an absolute URL → `route_refused`, parent fixture sees zero requests.
- Parent connection revoked → child `parent_degraded`; child invocation `route_unavailable`; no direct dial attempted (fake DNS/transport records none).
- Observation withdrawn in generation 42 → route stale; refusal until re-materialized.
- Kubernetes fake denies `get services/proxy` → `route_refused` before dispatch.
- Materialization with a parent whose route is `via` → refused (one hop).
- Audit rows contain no UID or URL (grep against fixture values).

## 7. Compatibility

- Old `grafana_datasource_proxy_v1` and `kubernetes_service_proxy_v1` route adapters map one-to-one to the two profiles. Old Grafana `datasource_query` (`expose = false`) is not carried; children query their own API through the route.

## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| `MediatedRoute` port (parent side) and `HttpCapability` implementation (child side) | `crates/connectors-sdk` (trait), `crates/connectors-host/src/http.rs` (binding) |
| Route binding store with generation check | host metadata store (`auth.connection`) |
| Composition rule: child adapter placed in the same host as the parent | `crates/connectors-host/src/server.rs` composition loading |
| Conformance harness runs each datasource suite direct and mediated | `crates/connectors-conformance` |

## 9. ESS entities

| Entity | Notes |
|---|---|
| `RouteBinding` (identity: child connection ref), lifecycle `bound → degraded → retired` | relations: `parent → Connection` (one), `observation → ResourceObservation` (one), `profile` value |
| `Connection.route` | value `direct` or `via` (connection document) |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Exact proxy path forms | taken from the vendor references at adapter authoring (Grafana data-source proxy; Kubernetes `services/proxy`), recorded in the adapter documents, not here |
| Non-GET methods through a route | refused in first profiles |
| Second hop | never; not a version question |
