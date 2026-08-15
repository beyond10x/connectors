# Design 08: discovery observations and mediated Connections

**Status:** accepted Connector model; personal-local Grafana and Kubernetes Service mediation
implemented; first explicit Agent Endpoint-Grant projection implemented · **Date:** 2026-08-15

**Inputs:** [Design 01](01-domain-model.md) · [Design 02](02-architecture.md) ·
[Design 07](07-credential-custody-topologies.md) ·
[Grafana data-source HTTP API](https://grafana.com/docs/grafana/latest/developer-resources/api-reference/http-api/api-legacy/data_source/) ·
[Grafana data sources](https://grafana.com/docs/grafana/latest/datasources/) ·
[Grafana Alertmanager data source](https://grafana.com/docs/grafana/latest/datasources/alertmanager/)

This design covers a provider that can reveal other provider instances and mediate access to them.
Grafana is the first case: its data-source API lists configured backends and its data-source proxy
can carry Prometheus-, Loki-, or Alertmanager-shaped requests. The model is generic; none of its
core nouns is Grafana-specific.

## 1. Vocabulary and boundaries

| Term | Meaning | Authority? |
|---|---|---|
| **Origin** | Scheme, host, and port selected by operator/deployment policy. | No; only a destination fact. |
| **Discovery observation** | A bounded, evidenced fact emitted by one declared normalizer over one admitted read response. | No. |
| **Connection candidate** | A normalized possible target Provider contract plus a proposed fixed route. | No; still unusable. |
| **Route** | How a Connection executes: directly, or through one parent Connection and a closed adapter. | No; it cannot replace a Grant. |
| **Connection** | A durable authorized instance of one Provider, with one fixed route and lifecycle. | Only together with a current independent Grant. |
| **Agent Endpoint** | The Harness projection of an admitted Connection under an Agent Endpoint Grant. | Agent-owned projection, not a Connector route. |
| **Datasource** | Worker-facing context selected after Agent admission. Use **Grafana data source** only for Grafana's vendor object. | No authority by itself. |
| **Federation** | B10x central-to-satellite topology. | Separate from mediated routing. |

“Host discovery” retires for this use. A discovered Grafana data source can name a logical cluster,
proxy, service, or hosted backend rather than one host. “Endpoint discovery” is also too early:
Endpoint is the Agent projection after a Connection exists.

**2026-08-15 clarification:** [Design 10](10-local-kubernetes-context-and-resource-discovery.md)
also permits a Connection candidate to come from trusted local configuration before a source
Connection exists. It has the same no-authority lifecycle but no proposed mediated route yet:
activation creates the direct source Connection. Observations remain facts behind an existing
Connection. The two origins share the candidate noun and never share the transition that produced
it.

## 2. The flow is deliberately not collapsed

```text
Grafana Connection + Grant
       │ invoke declared bounded read
       ▼
discovery observations ── unknown type ──► remain unsupported observations
       │ closed type mapping
       ▼
Connection candidates
       │ policy-backed materialization (guided connect or explicit control method)
       ▼
mediated Connections governed by target Provider contracts + independent Grants
       │ direct or fixed via-Connection route
       ▼
target Provider operations
```

Each arrow is a policy or lifecycle boundary. Listing a Grafana data source does not create a
Prometheus Connection. Creating that child Connection does not create a Connector Grant. An Agent
seeing the child Connection does not receive an Endpoint Grant automatically.

## 3. Provider declaration

Discovery is metadata over an ordinary operation, not a sixth callable member kind. The source
Provider declares:

- a stable discovery id and service;
- one operation in that service;
- one closed response-normalizer driver;
- exact observed-type → target-Provider + route-adapter mappings.

The loader requires the named operation to be a unary HTTP read with explicit `read` and `network`
effects. Mapping types are unique and use a bounded token grammar. Both driver and adapter are
closed enums. There is no selector, JSONPath, URL, arbitrary proxy suffix, credential, or fallback
field a caller can fill.

The target Provider need not live in the same source catalog because deployed catalogs are
composable. Materialization is nevertheless fail-closed: a candidate cannot become callable unless
the deployed catalog contains the target Provider and the selected placement installs a compatible
route adapter.

Grafana declares exactly these first mappings:

| Grafana `type` | Target Provider | Route adapter |
|---|---|---|
| `prometheus` | `prometheus` | `grafana_datasource_proxy_v1` |
| `loki` | `loki` | `grafana_datasource_proxy_v1` |
| `alertmanager` | `alertmanager` | `grafana_datasource_proxy_v1` |

Grafana documents `GET /api/datasources` as the instance-wide data-source list and
`/api/datasources/proxy/uid/:uid/*` as the proxy to the actual data source. Alertmanager's built-in
data-source provisioning type is `alertmanager`; Prometheus and Loki are likewise native Grafana
data-source types. Those vendor facts ground this mapping, but vendor URLs and UIDs remain internal
to the normalizer and route binding.

## 4. Observation and candidate invariants

An observation carries only safe normalized metadata: observation identity, declaration identity,
source Connection, observed type, bounded title, evidence generation and digest, plus a hidden
resource binding. Recognized observations can yield a candidate. Unknown types remain observable
but yield no candidate; they never fall through to generic HTTP.

The opaque resource binding is Connector-owned state. It may resolve internally to a Grafana UID,
but the UID, configured data-source URL, proxy path, secure JSON data, auth headers, and parent
credential are absent from catalog, Connection responses, Harness metadata, audit payloads, and
operation arguments.

Evidence is generation-bound. Refresh replaces the current observation set atomically. A missing
resource, changed vendor type, changed parent generation, or withdrawn mapping makes any dependent
candidate stale and degrades an already-materialized child Connection until reconciliation proves
the same binding again.

## 5. Connection route invariants

A Connection has exactly one route for its lifetime:

```text
direct

via_connection {
  parent_connection
  resource_binding       # opaque and Connector-only
  adapter                # closed enum
}
```

The public value-free projection omits `resource_binding`; it reports only `direct`, or the parent
Connection and adapter identity. A Connection cannot route through itself. Initial implementation
supports one mediated hop; nested mediated parents are refused until cycle detection, compound
audit, and revocation ordering have their own accepted design.

The child is governed by the target Provider contract. A Prometheus child therefore publishes the
Prometheus operation catalog and Prometheus semantics whether its route is direct or via Grafana.
“Target Provider” names the API contract; it does not imply a direct backend transport, another
credential, or another login. The Connection is “mediated” precisely because its fixed route stays
through the parent Grafana Connection.
The adapter changes request transport only. Planning strips the target Provider's direct origin and
produces a target-relative mediated HTTP plan; an unavailable adapter refuses before dispatch, so
there is no accidental direct-egress fallback.

Parent and child share execution placement. The route adapter uses the parent's admitted origin,
credential custody, destination aperture, and opaque resource binding. The invocation still needs
the child's own current Connector Grant. The parent Grant is not inherited, and the child Grant
does not license arbitrary Grafana operations or proxy paths.

Audit joins both identities: child Connection and Grant, parent Connection generation, discovery
evidence generation and digest, target Provider operation, route adapter, and final admitted
destination subject. Audit never records the hidden binding or credentials.

## 6. Grafana execution boundary

The first runtime adapter must:

1. invoke the declared Grafana list operation under the parent Connection's normal admission;
2. normalize only list entries whose exact `type` has a catalog mapping;
3. seal the Grafana UID as the observation's opaque resource binding;
4. on child invocation, re-resolve that binding and prefix only the target operation's reviewed
   target-relative path with Grafana's data-source proxy route;
5. refuse arbitrary suffixes, absolute URLs, redirects outside the parent aperture, unknown plugin
   types, missing target Providers, stale evidence, unavailable adapters, cross-placement parents,
   and nested mediated parents;
6. degrade dependants on parent revocation, authorization loss, discovery withdrawal, mapping/type
   change, or adapter failure.

Grafana operations remain on the Grafana Connection. Prometheus, Loki, and Alertmanager operations
remain on their mediated child Connections. A generic `grafana.proxy` operation is
explicitly out of model.

## 7. Harness projection

Connectors owns observations, candidates, Connections, route execution, credential custody, and
Connector Grants. The Agent/Harness owns Endpoint discovery, Endpoint Grants, Datasources, and tool
admission. The integration adapter therefore projects only released value-free facts:

```text
Connector Connection/candidate      Agent/Harness
────────────────────────────────    ─────────────────────────────
target Provider                  -> ProviderRef
Connection/candidate ref         -> EndpointRef (opaque)
Connection placement             -> BoundaryRef
safe label + normalized kind     -> EndpointDescriptor
evidence generation + digest     -> EvidenceRef / observation generation
Connection lifecycle             -> ConnectionState
admitted target operation set    -> Datasource + OperationDescription
```

The Agent never receives the parent credential, resource binding, Grafana UID, backend URL, or
proxy path. Its passive discovery reads stored observations through the value-free
`ConnectorConnection.observation_search` method. Any active refresh remains a normal Connector
operation and requires Agent discovery authority *and* Connector-side admission; the Agent's
`DiscoveryGrant` does not become a Connector Grant.

The first integrated Agent adapter sends the current owner snapshot and opaque
operation/Connection references over the owner-only local socket; it has no credential field or
provider transport. `zwirn connect kubernetes` persists only the selected Connection references.
On startup, Zwirn refreshes their current callable descriptions and feeds value-free Endpoint,
Datasource, owner-fact, and Operation observations plus session-scoped Endpoint Grants through the
normal capability compiler. Description leases remain inside the adapter and are refreshed again
immediately before invocation. This is not a claim that Connector observations became Agent
authority: an observation is absent from the Agent catalog until a person materializes its
Connection and the Harness independently grants that Endpoint.

## 8. Implemented personal-local slice

The `connectors` daemon now composes Grafana beside SIP and Slack. `connectors connect grafana`
submits one service-account token to a one-use owner-only completion socket, verifies it by listing
Grafana data sources, stores the token only in daemon memory, and materializes recognized data
sources for which the value-free configuration names an independent target Grant. Unknown types
remain unsupported observations.

The daemon exposes Grafana dashboard operations and target Provider operations for Prometheus,
Loki, and Alertmanager. Target invocation first proves a mediated plan, then rewrites the reviewed
target-relative request beneath exactly
`/api/datasources/proxy/uid/{sealed-binding}` on the configured Grafana origin. The executor permits
HTTPS JSON only, follows no redirects, uses no ambient proxy, and places only the parent Grafana
credential.

The first alpha deliberately does not persist that credential. Restarting the daemon removes it and
requires `connectors connect grafana` again. OS-keychain, wired secret-provider, and satellite-local
write-only custody remain the persistence/topology implementations specified by Design 07; no
plaintext file fallback is allowed while they are absent.
