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
| `resource_discovery` | `kubernetes-service-targets` | Service observations with inferred markers; potential Loki/Prometheus/Alertmanager candidates, informational Grafana/Argo markers, opaque locator, no dial |
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
| `deployment.rollout_restart` | mutation, `idempotency: keyed` (receiver F02), `approval: required` | external_write, network | `kubernetes.deployment.rollout-restart` |
| `services.observe` | resource_discovery `kubernetes-service-targets` | none | Service recognizer |
| (capability) `route.mediated_http` `kubernetes-service-proxy` | provided to the host | forwards GET | `kubernetes_service_proxy_v1` |

### 4.1 Rollout restart intent and replay

`deployment.rollout_restart` selects **receiver-keyed idempotency** under [operations §§5.1–5.2](../../contracts/operations/v1alpha1/semantics.md#51-key-namespace-fingerprint-and-replay-admission), retaining Kubernetes optimistic concurrency as an additional provider guard. Its closed input requires namespace, name, uid and resource_version, all nonempty exact strings within the ordinary request bound. Namespace/name must be valid single Kubernetes path components; UID is the Deployment identity, not Invocation.request_id or the host key. The admitted instance/connection fixes the configured cluster authority. No input can choose an API origin, refresh a credential identity, or substitute a new object under the same name.

This profile selects the built-in apps/v1 Deployment conditional-update binding evidenced at Kubernetes v1.35.0. Its resource_version input must be the canonical ASCII decimal representation of an integer in **1..18446744073709551615**: 1–20 digits, first digit 1–9, remaining digits 0–9, within that unsigned 64-bit bound. Zero, every all-zero alias, leading zeros on positive values, signs, whitespace, radix prefixes, non-ASCII digits and overflow are invalid_input at admitted operation-input validation, before key inspection or business dispatch. The host rejects these spellings rather than normalizing them. This rule is specific to the selected conditional Deployment binding, not a universal lexical rule for all Kubernetes resource versions or APIs. A different provider interpretation requires a separately established binding before advertisement.

The canonical approval/fingerprint binds that exact input and semantic connection/configuration/operation revisions. Mutation preflight checks the exact permission target `(patch, apps, v1, deployments, namespace, name, "")` under auth.evidence's mutation evidence rules. It does not fetch a new Deployment or refresh UID/resourceVersion. Accepted resourceVersion bytes are copied unchanged. The pinned provider compares parsed unsigned values; canonical positive admission makes that comparison identify the same admitted version without its zero/unconditional-update branch or alternate spellings. Numeric range validation does not choose a newer version or rewrite intent. No arithmetic, trimming, refetch or rebase may substitute a version. This is the chosen precondition contract, not a claim that current Kubernetes forbids all scoped version ordering.

For a candidate new attempt only, provider preparation fixes one timestamp marker from the host's current nonnegative epoch milliseconds, serialized as a canonical decimal string, and one exact patch body. This preserves the predecessor's decimal timestamp annotation, while making generation/fixity explicit. The marker is generated once for that candidate and associated with its immutable prepared intent; it never changes after the attempt is anchored. Approval authorizes the declared single timestamp-patch operation, including this receiver-controlled generation rule; the marker is not caller input, an authority token, a deduplication key or a unique-rollout promise. A generation-rule change is an operation-semantic revision. A concurrent loser discards its prepared candidate; matching host replay/wait never generates another marker or body. Recovery never sends an anchored request again.

Send one strategic merge PATCH to `/apis/apps/v1/namespaces/{namespace}/deployments/{name}` with `Content-Type: application/strategic-merge-patch+json`, metadata.uid and metadata.resourceVersion copied exactly, and only `spec.template.metadata.annotations["kubectl.kubernetes.io/restartedAt"]` set to the fixed marker. No server-side apply/force, alternate object, automatic version refetch/rebase or follow-up rollout polling is implicit. Permission checks and PATCH share the original 15 s provider / 20 s service budget and 4 MiB response bound.

A definitive successful API acknowledgement must contain the exact Deployment namespace/name/UID, a nonempty resulting resourceVersion and the requested annotation value. Return `{namespace, name, uid, resource_version, patch_accepted: true}` with applied. This proves the accepted patch intent only, not rollout convergence, healthy Pods, or even a fresh template value if an equal marker was already present. A timestamp is not guaranteed unique. Unexpected identity/value, malformed/oversized response, transport loss or generic provider error after possible dispatch is unknown/outcome_unknown. For a binding-verified precondition refusal that proves this PATCH was not applied, use refused with a safe invalid_input error; preserve safe not_found/forbidden for their verified refusal cases. Do not copy the predecessor's broad 409/422 mapping as universal no-effect proof. A code alone or an unrelated later conflict does not settle an earlier attempt.

| Request history | Required meaning |
|---|---|
| Same live host key and same full fingerprint | observe original pending/terminal/quarantined reservation under current result admission; no new marker, approval spend or PATCH |
| Same live key, changed namespace/name/UID/version or semantic binding | idempotency_conflict; no stored-input/axis disclosure or new PATCH |
| Original PATCH succeeded and advanced the version; a deliberate new key uses original UID/version | API precondition can refuse this later PATCH; that refusal alone cannot determine whether the original or another actor advanced the version |
| Original reply lost; current version still matches original | an independently admitted new key could apply a PATCH; do not assume all repeats must conflict or must cause another rollout |
| Same name now identifies another UID | exact UID guard prevents silently restarting the replacement; do not remove the guard or refetch a successor |
| Caller intentionally reads a new version, forms new canonical input and obtains fresh admission/approval/key | new mutation intent, not a replay or reconciliation of the earlier unknown attempt |
| Later read matches marker or reports healthy Pods | present state is not proof of which exact earlier request executed; no automatic settlement/retry |
| Known-result key expires, or an unknown reservation remains quarantined | §5.1 retention and fresh-admission rules apply; expiry does not unspend approvals or reopen unknown reservations |

[Provider evidence](../evidence/restart-visibility-20260908/provider-evidence.md) pins the old implementation at 81459ac4, official Kubernetes conditional update documentation, immutable-UID validation, unsigned version parsing, Deployment's unconditional-update strategy and the strategic PATCH/store path. The evidence supports the selected positive-version precondition mechanism; it does not execute a provider race or prove all server/admission-plugin behavior. The concrete binding must establish its exact success/no-effect evidence before advertisement. Existing AttemptRecord/KeyReservation lifecycles and the rule against reclassifying a settled Indeterminate attempt remain unchanged.

## 5. Auth

| Profile | Scheme | Acquisition | Capability | Evidence |
|---|---|---|---|---|
| `kubernetes.bearer` | `http_bearer` | `static_config` (token or token file) | `http-bearer` | `identity_check`, `permission_check` |
| `kubernetes.mtls` | `mtls` | `static_config` (cert + key refs) | `mtls-client-identity` | same |
| `kubernetes.exec_plugin` | `exec_plugin` | `exec_plugin`, disabled unless `auth.allow_exec_plugin = true` | `exec-credential-plugin` (runs only inside an admitted operation, bounded deadline) | same |

The bearer and mTLS static_config paths use separately admitted deployment activation, not auth.begin or interactive credential entry; references remain in configuration and custody supplies immutable captures. The exec row records a reserved future acquisition binding: the enable flag is necessary but insufficient, and cannot advertise runtime support until a separately reviewed flow/reader/implementation exists ([acquisition matrix](../../contracts/auth/acquisition/v1alpha1/semantics.md#40-acquisition-paths-and-required-declarations)).

Namespace fan-out follows [auth.evidence §4.4](../../contracts/auth/evidence/v1alpha1/semantics.md#44-exact-authorization-targets-and-fan-out-budget-f08): at most 64 distinct exact targets and 64 authorization calls per invocation (lower policy limits allowed), at most one call per uncached target, with a shared deadline and no automatic check retry. Two namespaces and four configured kinds require eight targets. Fresh evidence saves calls only for the identical checking binding/generation and ResourceAttributes. Host policy admits the whole input first; all required checks finish before resource requests. More targets than allowed refuses invalid_input; insufficient call budget, timeout or unknown permission refuses unavailable without resource reads. A denied subset is skipped with explicit target coverage and complete:false under a selected coverage-capable payload; all denied is forbidden. An allowed empty namespace is reported as allowed, never conflated with denied. The current strict Page cannot carry that coverage and therefore cannot advertise this proposed partial-read behavior. Discovery cannot treat denied/unavailable partitions as complete or infer their unseen resources were withdrawn; an independently complete, comparable partition may prove absence only within its exact scope even in a partial overall view; [discovery coverage §§4.1–4.4](../../contracts/discovery/resources/v1alpha1/semantics.md#41-exact-coverage-scope) supplies the exact scope, provider completeness, retention and publication rules.

The SSAR evaluation itself uses a separately admitted private POST capability restricted to the exact review endpoint/body. Business GET capability and mutation approval rules are not widened. Proxy authorization uses its own exact services/proxy tuple and shares the child's remaining budget; listing Services is no substitute.

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

- `services.observe` emits one observation per admitted well-formed core/v1 Service under the configured namespace mode, with a source-qualified opaque locator. Recognized rows carry recognition.confidence=inferred; unrecognized rows have recognition:null and candidate:null. Candidates remain nullable and supply no authority; reachability is unknown unless the selected mapping establishes a permitted source-only route.
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

A Service discovery partition is the exact admitted namespace/core-v1-Services collection (or explicitly admitted cluster-wide collection). Complete permission coverage does not prove all provider pages were exhausted. Explicitly configured all-namespaces mode uses exactly `(list,"","v1","services","","","")` under F08, distinct from finite per-namespace mode; missing selection or denied namespace cannot enable it. Both modes share the same scan ceilings. Capped/failed/denied partitions preserve trustworthy current positives and retain nonterminal history as stale without proving disappearance; previously withdrawn incarnations remain terminal until bounded eviction; a narrower namespace selection cannot withdraw from the old scope. Same namespace/name with a new UID or changed fixed port/semantic target cannot repoint an existing child. [Resource discovery §4.5](../../contracts/discovery/resources/v1alpha1/semantics.md#45-fixed-declarations-recognition-and-configured-source-identity) owns the exact declaration, recognition and configured source-identity boundary. services.observe fixes kubernetes-service-targets and the admitted configured source; no request profile or origin retargets it. Argo matches the whole Service name OR stable app.kubernetes.io/name label after declared ASCII lowercase normalization, before monitoring hints; it always has candidate:null and no callable route. Other old label aliases do not independently identify Argo in this selected profile. Different configured instance/connection/API-origin bindings stay distinct; physical-cluster attestation/deduplication is explicitly unselected.

Mediated parent and child libraries are linked and injected by an explicitly authored [composition executable](../../contracts/discovery/composition.md), not by generic host/server code or either sibling library. Independent processes on the same machine do not supply this private port. Direct standalone Kubernetes remains independent; discovering an endpoint never installs or starts a target adapter.
