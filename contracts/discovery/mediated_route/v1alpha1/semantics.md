# route.mediated_http/v1alpha1

- **Status:** proposed, not implemented.
- **Family:** discovery. Siblings: [resources](../../resources/v1alpha1/semantics.md). Consumes [auth.capability](../../../auth/capability/v1alpha1/semantics.md) (`mediated-http`) and [auth.connection](../../../auth/connection/v1alpha1/semantics.md) (`route.kind = via`).
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `route.mediated_http/v1alpha1` |
| Profiles | adapter-owned versioned parent-route bindings; no closed provider catalog |
| Provided by | the *parent* adapter as a capability |
| Consumed by | the host, which constructs the `mediated-http` capability handed to a *child* adapter ; the child adapter does not know it is mediated |

A private endpoint may be unreachable from where the client runs; the design allows an adapter placed in the reachable network or a separately admitted mediated-route capability, and states that federation of discovery results does not make private addresses reachable (`docs/design.md:483`). This contract is that capability: the parent forwards a target-relative HTTP request through its own admitted connection to one resource it observed. It is distinct from host federation, which forwards *operations* between services (`crates/connectors-host/src/federation.rs`); a mediated route carries the child's *provider* traffic.

Current native parent bindings are
[Grafana](../../../../adapters/grafana/contracts/routes/v1alpha1/semantics.md) and
[Kubernetes](../../../../adapters/kubernetes/contracts/routes/v1alpha1/semantics.md).
These are index links, not dependencies on concrete adapters.

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
{ "provides": [ { "contract": "route.mediated_http/v1alpha1", "profile": "adapter-owned-route", "targets": ["supported-child-kind"] } ] }
```

Host-side route binding (configuration created by materialization, `resource_discovery`):

```json
{ "child_connection": "conn_child_7", "parent_connection": "conn_parent_1", "observation": "obs_…", "profile": "adapter-owned-route", "observation_generation": 41, "route_revision": "route_rev_7", "target_adapter": "supported-child-kind" }
```

Parent port (Rust, private within one admitted composition; no public forwarding operation):

```text
forward(binding, method, target_relative_segments, query, headers_allowlisted, body_bounded) -> HttpResponse
```

The child receives an ordinary `HttpCapability` (`auth.capability`) whose implementation calls `forward`. Its `destination` is the *target-relative root*, not a URL:

```json
{ "connection": "conn_child_7", "profile": "child.via_parent", "destination": { "kind": "mediated", "route": "adapter-owned-route", "parent": "conn_parent_1" } }
```

Errors returned to the child: base codes plus `route_unavailable` (parent not ready, observation withdrawn, generation mismatch) and `route_refused` (suffix, method, or target outside the reviewed set). The child maps them like any dependency failure; it never retries through a different route.

## 4. Rules

- Transport only: the route changes how bytes reach the target. The child's business contracts, datasource profiles, operations and result semantics remain identical; its explicitly selected auth profile and safe route metadata differ from direct access. Conformance runs the same business suite direct and mediated (`docs/design.md:1035`, section 21.3).
- Fixed binding: materialization fixes the observation incarnation, source parent, semantic target/resource/port, route profile, access mode and host owner. An observation generation is the last successfully revalidated evidence revision, not permission to change that fixed identity. Request input cannot choose the parent, resource, prefix or port. See §4.1 for same-target revalidation.
- Path discipline: the parent prefixes its proxy route to individually encoded target-relative segments supplied by the child's capability. Absolute URLs, `..`, empty segments, and query keys outside the child's declared set are refused before dispatch.
- Methods: the profile declares allowed methods; first profiles allow `GET` only (both old adapters were read-only).
- Authority: the child's selected profile is `purpose: mediated_access`, `subject: none`, `scheme: parent`, `acquisition.flow: static_config`, capability mediated-http. It owns no provider credential, custody reference or external account. The parent capability authenticates the hop using its own current pinned generation; downstream credentials, if any, belong to the reviewed parent route/datasource binding. Child policy and route admission are required independently and never inherited from the parent. The route does not license arbitrary parent operations, copy parent grants into child scopes or expose parent secrets. Child evidence binds parent generation and the fixed route/observation revision, so parent publication/revocation or mapping changes invalidate it.
- Depth: exactly one hop. A parent connection that is itself `via` cannot provide a route (`route_refused` at materialization).
- Placement: parent and child run in one process with live private ports injected by the [authored composition executable](../../composition.md). A deployment of independent processes is not this private composition. The route is never exposed as a remote capability to another host; provider-traffic federation remains out of model.
- Redirects from the parent's provider are not followed; responses are passed through with the parent's response-size bound.
- Safe audit per forward: child/parent connection, public observation generation/id, target operation, profile and admitted destination label. Private credential-generation pins and equality/route-state coordinates remain internal correlation, never serialized with the UID, URL or credential.

### 4.1 Observation continuity and route revalidation

Three coordinates are distinct: public observation publication generation under [resource coverage](../../resources/v1alpha1/semantics.md#44-publication-retention-and-public-paging); the private parent credential generation governed by F05; and a host-private route binding revision advanced only by admitted same-target revalidation. The latter example above is host-side data, not a newly public connection field. All must agree with the currently published source/route binding at final dispatch. A normal new observation view or parent material publication invalidates old route admission/child verification; neither silently advances an old capability or permanently requires changing an otherwise identical child ref.

Same-target revalidation is an explicit host configuration/validation action under the selected static_config materialization binding. It is not a public generic `forward`, auth.begin, implicit business retry or authority carried by a discovery cursor. The host separately admits exact child, parent and fixed observation/target; requires a positively observed, unexpired row in the latest acknowledged comparable scope; asks the parent profile to prove exact fixed target/type/tenant/auth-placement equality; and establishes current parent generation and exact route permission. A stale/withdrawn/evicted row supplies no authority. If fresh observations or provider validation are needed, they run only under that separately declared admitted validation plan and finite effect/call/byte/deadline budget. Ordinary invocation requiring an unavailable revalidation step refuses route_unavailable; it cannot invent probes or broaden its budget. Route permission collected for an ordinary already-valid forward shares the child's remaining F08 budget as §4 states.

At one host metadata publication point, compare the expected route revision, current observation generation/incarnation, same fixed semantic binding, parent generation, current independent authority and non-revocation/enablement. An acknowledged success advances only the route evidence revision and invalidates all pending old route admissions and child verification. It creates no credential or new target. Unknown acknowledgement permits no forward; observe that owner or refuse. A stale revalidator cannot resurrect a locally revoked/disabled child or replace newer binding evidence. The final forward repeats the applicable current authority/permission/generation/deadline fence and uses the parent's pinned material. Revalidation success alone is not permanent permission.

Retained history after denied/capped/failed discovery is not deletion and cannot authorize a route. An enumeration denial is not proof that forwarding is denied; the separate exact forwarding permission decides it. A current definite proxy denial is route_refused even when history contains an old successful list. Missing fresh observation/equality or a degraded parent remains route_unavailable, and unavailable required proxy evidence uses unavailable; no direct fallback. Local child revoked/disabled precedence remains the auth.connection reduction. A changed type, provider object incarnation, hidden fixed target, port, parent, route mode or owner cannot use this revalidation path: a new independently admitted connection is required. Title-only rename may revalidate the same fixed target. Confirmed withdrawal terminates the old observation incarnation; reappearance gets a new id and never revives its old child.

### 4.2 Embedded configuration and historical association

The first persistent binding stores zero or one
`connectors.discovery.MediatedRouteBinding` value on its existing child Connection.
There is no route entity, independent route ID or route lifecycle. Child identity
is Connection.connection_ref; its sole canonical parent is
Connection.parent_connection_ref, a reference to the existing parent Connection.
The route value does not duplicate either as another authority. Via bindings
require both parent and route value; direct bindings have neither, and a parent
cannot itself be mediated. Many children may independently reference one direct
parent and the same historical observation.

The fixed value names the original collection_ref, scope epoch and observation_ref,
route profile, target adapter, sealed private fixed-target handle and access/owner
configuration. These fields cannot change through revalidation. They are historical
coordinates, not a mandatory live relation to ResourceObservation: retiring or
cleaning a collection/row cannot be blocked by a child. Current resolution may
find no row; that yields route_unavailable and grants no provider access. A newly
allocated scope epoch, even for the same native object, cannot match this old
coordinate or repoint the child.

Only the embedded route-evidence value may advance by §4.1 same-target CAS. It
records a private route revision, the successfully revalidated observation
generation, parent credential-generation coordinate where applicable, and its
original observation/validity times. A credentialless admitted direct parent has
no invented material-generation ID; the selected parent profile must explicitly
support that absence. Missing required material/evidence still refuses. Route
evidence loss/expiry removes current usability without deleting the fixed child
binding, creating child credentials or resetting original time. These internal
values add no fields to the public Connection or capability codec.

## 5. Limits

| Concern | Rule |
|---|---|
| Deadline | the child's provider deadline; the parent hop consumes part of it |
| Body | child response bound (4 MiB today) applies after the hop |
| Concurrency | counted against the parent's concurrency limit as well as the child's |

## 6. Conformance scenarios (`docs/design.md:989-991`)

- Same child fixture reached direct and via a fake parent → byte-identical child results and errors.
- Child capability asked for `../api/admin` or an absolute URL → `route_refused`, parent fixture sees zero requests.
- Parent connection revoked → child `parent_degraded`; child invocation `route_unavailable`; no direct dial attempted (fake DNS/transport records none).
- Fresh unchanged observation at generation 42 → old admission refuses until explicitly admitted same-target revalidation; the fixed child ref may remain. Confirmed withdrawal at 42 → old observation incarnation terminal; any reappearance needs a new observation and newly admitted child.
- Retained stale observation plus current proxy denial → route_refused when exact permission is checked, never an old-history grant. List-only denial is not substituted for proxy denial.
- Same UID/type but changed sealed target or port → refuse old route; no automatic re-resolution to a new destination.
- Materialization with a parent whose route is `via` → refused (one hop).
- Audit rows contain no UID or URL (grep against fixture values).

## 7. Compatibility

- Old `grafana_datasource_proxy_v1` and `kubernetes_service_proxy_v1` route adapters map one-to-one to the two profiles. Old Grafana `datasource_query` (`expose = false`) is not carried; children query their own API through the route.
- [Service compatibility](../../../service/compatibility.md) is authoritative for the binding. Safe capability advertising needs the extended Descriptor.provides field; forwarding remains a private same-composition port. Child-visible route errors require the extended error set. No public generic forwarding endpoint is introduced.


## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| `MediatedRoute` port (parent side) and `HttpCapability` implementation (child side) | `crates/connectors-sdk` (trait), `crates/connectors-host/src/http.rs` (binding) |
| Route binding store with generation check | host metadata store (`auth.connection`) |
| Concrete parent/child construction, linking and port injection | authored composition executable above generic host and both adapter libraries, per [composition ownership](../../composition.md); no concrete loading in server.rs |
| Conformance harness runs each datasource suite direct and mediated | `crates/connectors-conformance` |

## 9. ESS entities

| Entity | Notes |
|---|---|
| Fixed mediated route and evidence | `connectors.discovery.MediatedRouteBinding` and `MediatedRouteEvidence` are embedded values, joined to the child Connection by its owner. Parent is a Connection reference; historical observation coordinates have no live observation FK. This is not the unrelated gateway RouteBinding value in idempotency.yaml. |
| `Connection.route` | value `direct` or `via` (connection document) |
| Revalidation and placement facts | [discovery.yaml](../../../../ess/domains/discovery.yaml) types selected values, not a running validator, installed port or persistent composition owner |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Exact proxy path forms | selected from provider evidence and owned by each adapter's route binding |
| Non-GET methods through a route | refused in first profiles |
| Second hop | never; not a version question |

Persistence ownership is consolidated in [design §31](../../../../docs/design.md#31-host-persistence-ownership-and-atomicity). MediatedRoutePort owns fixed-route validation evidence; its current observation/parent/child comparisons participate in the selected binding metadata authority. This inventory does not supply a backend or execute its atomicity predicates.
