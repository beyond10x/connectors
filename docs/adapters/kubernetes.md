# Adapter design: Kubernetes (extension of the implemented adapter)

- **Status:** design for the extension; the base adapter is implemented (`adapters/kubernetes/`, `connectors.adapter/v1`).
- **Old baseline:** `../connectors` at `81459ac4`: `crates/integration-kubernetes/`, `docs/design/10-local-kubernetes-context-and-resource-discovery.md`.
- **Contract index:** [contracts/README.md](../../contracts/README.md).

## 1. Scope and placement

Implemented today (`adapters/kubernetes/spec/adapter.json`): `resources.list` (`datasource.records/v1alpha1`, profile `kubernetes-list`), `endpoints.discover` (`endpoint_discovery/v1alpha1`), `hosts.discover` (`host_discovery/v1alpha1`); configuration `namespaces`, `resource_kinds` (pods, services, deployments, endpointslices), `discover_hosts`; bearer token via `urn:connectors:config:v1:http` credential. Placement: in-cluster or wherever the API server is reachable (`contracts/service/v1alpha1/semantics.md`, Kubernetes paragraph).

Rebuild adds what the old integration had and the first slice did not: pod logs, deployment status and rollout restart, Service target recognition, the API-server service proxy route, exec-plugin and client-certificate authentication, and per-verb permission evidence. Kubeconfig context candidates (old design 10 §1-2) become host/CLI configuration tooling, not an adapter contract.

## 2. Old surface and disposition

| Old surface | Source | Disposition |
|---|---|---|
| `kubernetes.deployment.status` | `crates/integration-kubernetes/src/workloads.rs:48` | preserve → `resources.list` with `kind: deployments` already returns status; add `deployment.get` records single item if a consumer needs one object |
| `kubernetes.deployment.rollout-restart` | `workloads.rs:49` | preserve → `operations` `mutation` (PATCH of the pod template annotation) |
| `kubernetes.pod.logs`: `namespace`, `pod`, `container`, `tail_lines` 1–1000 (200), `since_seconds` ≤ 86400, 128 KiB, namespace grant as containment | `workloads.rs:50,1120-1145` | preserve → `datasource.logs` profile `kubernetes-pod-logs` |
| `kubernetes.workloads` datasource (value-projected, excludes Secrets, env, labels, annotations, raw objects, event messages) | `workloads.rs:51`; `contracts/connector-datasource/v0alpha1/README.md:13-15` | change: `resources.list` returns provider-native objects with a declared projection; the exclusion list becomes a configured projection, not a contract |
| `kubernetes.databases` datasource | `databases.rs:46` | preserve → `endpoints.discover` classification candidates (implemented) |
| Kubeconfig context candidates, passive read, `allow_exec_auth = false` | `docs/design/10-…:18-72`; `local.rs:311-315` | move to host/CLI tooling; exec plugin becomes `auth.capability` `exec-credential-plugin` gated by configuration |
| Core/v1 Service recognizer (grafana, prometheus, loki, alertmanager), SSAR per namespace, `resource_limit`, no Secrets/ConfigMaps/env/EndpointSlice addresses | `docs/design/10-…:73-100` | preserve → `resource_discovery` profile `kubernetes-service-targets` |
| `kubernetes_service_proxy_v1`: fixed Service and port, `get` on `services/proxy` per invocation, target-relative GET via API server, no fallback | `docs/design/10-…:102-116` | preserve → `route.mediated_http` profile `kubernetes-service-proxy` |
| Argo CD recognized, observation only | `docs/design/10-…:173` | preserve as recognizer marker with no route |
| Token, token file, client certificate auth | `local.rs:1076-1079` | preserve → `http-bearer`, `mtls-client-identity` |
| Watches | `docs/design.md:939` | deferred (`events`) |
| Process execution into a pod | none in old repo (`rg exec` finds only exec-auth) | deferred (`execution`, `docs/design.md:464`) |

## 3. Contracts needed and why

| Contract | Profile / use | Why |
|---|---|---|
| `datasource.records` (exists) | `kubernetes-list` | resource inventory with `continue` tokens and `resourceVersion` |
| `endpoint_discovery` (exists) | EndpointSlice observations | database/monitoring endpoint candidates without dialing |
| `host_discovery` (exists) | nodes | host inventory |
| `datasource.logs` | `kubernetes-pod-logs` | bounded per-container log lines with timestamps; no query language |
| `operations` `mutation` | `deployment.rollout_restart` | externally visible change with approval and outcome-unknown semantics |
| `resource_discovery` | `kubernetes-service-targets` | recognized Services as candidates for Loki/Prometheus/Alertmanager/Grafana adapters, opaque locator, no dial |
| `route.mediated_http` | `kubernetes-service-proxy` (parent side) | reach an in-cluster Service from an adapter that cannot route to it, through the API server proxy, one hop, GET only |
| `auth.profile` | `kubernetes.bearer`, `kubernetes.mtls`, `kubernetes.exec_plugin` | three credential mechanisms with different effects and gating |
| `auth.capability` | `http-bearer`, `mtls-client-identity`, `exec-credential-plugin` | bearer placement; TLS client identity; helper execution only inside an admitted operation |
| `auth.evidence` | `identity_check` (SelfSubjectReview at connect), `permission_check` (SelfSubjectAccessReview per verb, group, version, resource, namespace before each list and before each proxy forward) | provider-side authorization is queryable; denied namespaces are reported, not empty |
| `auth.connection` | `configured` | one cluster binding per instance; `managed` not needed |
| `auth.custody` | `read_only` | tokens and certificates are deployment-supplied; no acquisition flow |

## 4. Operation map

| New id | Contract / profile | Effects | Old id |
|---|---|---|---|
| `resources.list` (exists) | records `kubernetes-list` | read | `kubernetes.workloads`, `deployment.status` |
| `endpoints.discover` (exists) | endpoint_discovery | none | `kubernetes.databases` |
| `hosts.discover` (exists) | host_discovery | none | — |
| `pod.logs` | logs `kubernetes-pod-logs` | read | `kubernetes.pod.logs` |
| `deployment.rollout_restart` | mutation, `idempotency: natural` (annotation timestamp), `approval: required` | external_write | `kubernetes.deployment.rollout-restart` |
| `services.observe` | resource_discovery `kubernetes-service-targets` | none | Service recognizer |
| (capability) `route.mediated_http` `kubernetes-service-proxy` | provided to the host | forwards GET | `kubernetes_service_proxy_v1` |

## 5. Auth

| Profile | Scheme | Acquisition | Capability | Evidence |
|---|---|---|---|---|
| `kubernetes.bearer` | `http_bearer` | `static_config` (token or token file) | `http-bearer` | `identity_check`, `permission_check` |
| `kubernetes.mtls` | `mtls` | `static_config` (cert + key refs) | `mtls-client-identity` | same |
| `kubernetes.exec_plugin` | `exec_plugin` | `exec_plugin`, disabled unless `auth.allow_exec_plugin = true` | `exec-credential-plugin` (runs only inside an admitted operation, bounded deadline) | same |

Configured token, certificate/key and exec output replacement follows [auth.evidence §§4.1–4.3](../../contracts/auth/evidence/v1alpha1/semantics.md#41-credential-generation-and-identity). The host captures a coherent immutable credential generation and validates its identity in an explicitly admitted activation/revalidation step. Reusing a filename, cluster connection ref or configuration revision cannot reuse old identity evidence. The connection binds the observed stable Kubernetes user subject within the admitted cluster authority; a matching display name is insufficient. An unexpected identity change refuses ordinary replacement and requires a separately authorized reassignment flow if one exists.

SelfSubjectReview for activation/replacement is an admitted auth-validation effect, with a declared budget; it is never a hidden identity call added to `resources.list`. Until that validation is available and admitted, a new candidate generation is not ready. SelfSubjectAccessReview for each operation/target and the resource request must use the same validated material. Exec runs once to capture the output used for identity validation and dispatch, rather than returning a different token on a second helper run. A client certificate and key must be captured as a coherent pair, never from independent mutable reads around admission.

The capability pins this exact generation through the final dispatch check. Successful publication of a replacement invalidates old pending admissions, and known revocation, expiry or an unresolved rotating refresh refuses even a pin. A raw file change cannot substitute bytes into a pin before the host observes it. Permission evidence never transfers across generations; same-account replacement requires fresh SSAR for the exact target. No public describe/log field exposes generation IDs, secret versions or snapshot references. This is extension semantics; the implemented bearer adapter has not gained this runtime behavior yet.

Ambient and kubeconfig HTTP proxies stay disabled (old rule, `docs/design/10-…:98-100`); the API server origin must be canonical HTTPS.

## 6. Configuration outline

Extends the implemented schema:

```json
{ "adapter": {
    "namespaces": ["a", "b"], "resource_kinds": ["pods", "services", "deployments", "endpointslices"], "discover_hosts": false,
    "logs": { "enabled": true, "max_bytes": 131072 },
    "writes": { "rollout_restart": { "enabled": false, "deployments": ["a/api"] } },
    "service_targets": { "enabled": true, "recognizers": ["grafana", "prometheus", "loki", "alertmanager", "argocd"], "resource_limit": 500 },
    "service_proxy": { "enabled": false, "targets": ["prometheus", "loki", "alertmanager"] },
    "auth": { "mechanism": "bearer | mtls | exec_plugin", "allow_exec_plugin": false } } }
```

## 7. Discovery and routes

- `services.observe` emits one observation per recognized core/v1 Service in allowed namespaces with `locator.kind = opaque` (digest of Service UID), `candidate.confidence = inferred`, `reachability = via_source_only`.
- Materialization (host): candidate + operator selection → child `auth.connection` (`route.kind = via`, parent = this instance's connection) → child adapter (Prometheus, Loki, Alertmanager) receives a `mediated-http` capability whose forward goes through `services/proxy` for the fixed namespace, Service and port. Grafana discovered in-cluster stays fail-closed: it needs its own service-account credential (`docs/design/10-…:112-116`).
- The engineering chain (`docs/design.md:998`) extends: Kubernetes observes a Prometheus Service → explicit binding → series query through the host, direct and federated.
- One mediated hop; Kubernetes → Grafana → Prometheus is refused (`docs/design/10-…:115-116`).

## 8. Specification profile

`connectors.adapter/v1` handwritten (as today). The Kubernetes OpenAPI is served per cluster and is very large; no generation from it is planned. Vendor references cited per operation: API concepts, EndpointSlices, node, pod log (`read log of the specified Pod`), SelfSubjectAccessReview, `services/proxy` (URL forms recorded when authored).

## 9. Deferred

`events` (watch, resourceVersion gaps), `execution` (pod exec with admitted workload/container, `docs/design.md:464`), tunnels beyond the API-server proxy.

## 10. Evidence required

- Fixture: log lines with timestamps and truncation; SSAR denial per namespace; exec plugin refused when disabled and never run during describe; proxy forward path discipline and refusal cases; rollout-restart approval, replay and unknown outcome.
- Credential replacement fixtures: account A → B at the same token path refuses; same-account replacement waits for admitted validation; replacement between admission and dispatch cannot substitute bytes; exec validation and use share one output; certificate/key snapshots remain coherent; stale SSAR evidence is not reused for a new generation; revoked/expired pins refuse. [F05 authored scenario compilation](../../contracts/auth/evidence/v1alpha1/verification.md) is structural evidence only; these transport fixtures remain implementation obligations.
- Live: existing `docs/live-e2e.md` extended with pod logs and a Service observation → mediated Prometheus read.
- Decoupling: adapter builds without Loki/Prometheus crates; the child adapters build without Kubernetes.
