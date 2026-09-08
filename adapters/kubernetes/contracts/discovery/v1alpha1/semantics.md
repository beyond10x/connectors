# Kubernetes Service discovery/v1alpha1

**Status:** proposed native profile `kubernetes-service-targets`. Existing endpoint
discovery does not implement this new payload or authoritative absence model.
Implements [resource_discovery/v1alpha1](../../../../../contracts/discovery/resources/v1alpha1/semantics.md).

Only core/v1 Services are enumerated. Before any list, the
[Kubernetes permission binding](../../auth/v1alpha1/semantics.md) checks every
exact selected namespace or the explicitly configured all-namespaces collection.
Denied subsets are reported; all denied is refused. Lists never scan Secrets,
ConfigMaps, environment, EndpointSlice addresses, external URLs or arbitrary hosts.

The selected partition codec is `{id,kind,namespace,state}`: namespace mode has
kind=namespace and the exact configured namespace; explicit all-namespaces mode
has kind=cluster and namespace:null. Each partition is one exact Service-list
target. State uses the shared complete/capped/denied/unavailable/not_scanned enum.
The shared ordering, scope equality, coverage/publication and finite scan limits
apply. A private provider resourceVersion/continuation is collection evidence,
never the host's publication generation. The required coherent-list/exhaustion
predicate remains an explicit native authoring obligation before authoritative
absence can be advertised.

### 4.5 Fixed declarations, recognition and configured source identity

| Adapter declaration | Contract/profile fixed by the receiver | Source selection |
|---|---|---|
| Kubernetes `services.observe` | resource_discovery/v1alpha1, kubernetes-service-targets | The admitted configured Kubernetes source connection/API origin; only core/v1 Services under the configured namespace mode |

These are adapter-chosen ids; another authored id may implement the same contract only with its own exact, unambiguous descriptor/declaration binding. There is no magic resources.observe dispatcher and no caller-selectable profile. Each declaration fixes one profile and one receiver-owned source-selection rule; each admitted invocation resolves exactly one configured source binding. The first adapter examples each have one configured source connection. Missing, ambiguous or incompatible source selection refuses before provider work; an explicit Invocation.connection must match the selected admitted source under the service binding. A gateway alias resolves to the same source-qualified leaf declaration and cannot replace its contract/profile/source meaning. Duplicate or conflicting declarations are authoring/admission failures. Advertise only a supported strict reader and implemented realization; these documents do not add descriptor fields or operations to today's readers.

The selected input is closed to limit/cursor. A request profile, source URL, namespace override, mapping table or target adapter is invalid_input; it cannot retarget the declared scan. Receiver configuration owns namespace mode/filters/mapping, and its revision participates in scope/cursor equality. F08 permission preflight and F13 provider coverage remain independent. A declaration/profile/configuration change requires the corresponding new source/selection/projection identity; old cursors refuse rather than reinterpret it.

For Kubernetes, every well-formed admitted Service considered by the profile yields an observation, including an unrecognized Service with recognition:null, candidate:null, auth_requirement.kind:unknown and reachability:unknown. Invalid/incomplete provider objects follow §4.2's malformed-data rule; recognition never hides provider work from the scan limits. The observed_type remains kubernetes_service. Recognition uses the Service name and identity labels as specified below, with ASCII lowercase normalization. Missing labels contribute no token; there is no whitespace trimming, fuzzy matching or locale-dependent comparison. The old implementation consulted app.kubernetes.io/name, app, k8s-app and name (old local.rs:1170–1225 at 81459ac4). This selected Argo rule uses only the name and stable app.kubernetes.io/name label: the three broader legacy aliases do not independently establish the API marker. That is an explicit profile change, not a claim of complete old recognizer equivalence; monitoring keeps its older four-label inputs.

1. If **the Service name OR app.kubernetes.io/name whole token equals argocd-server**, report recognition {provider:argocd,confidence:inferred}. Exact Service name OR exact stable app.kubernetes.io/name label suffices; the other legacy labels cannot supply that match. This arm precedes monitoring substring recognition. It never matches argocd, argocd-repo-server, argocd-server-metrics or argocd-redis merely by their name/ordinary component labels. A Helm-prefixed name with the stable exact label matches. A falsely supplied exact label can still produce an inference: this is provider metadata, not verified application identity.
2. Otherwise apply the preserved monitoring substring arms in order grafana, alertmanager, loki, prometheus across the name and all four legacy identity-label tokens. The first matching arm supplies the inferred marker; there is no implicit additional adapter kind or arbitrary proxy. This ordering makes multiple legacy hints deterministic without claiming they prove the service type.
3. Otherwise recognition and candidate are null.

Argo recognition always has candidate:null in this profile, no mediated route and no callable Argo adapter. Recognizing a marker is not a capability advertisement. Other candidates still require the closed source-specific adapter/route/auth mapping and all explicit materialization, fixed-target, implementation and current-authority checks; a name or label cannot select a credential or bypass those checks. The first Kubernetes mediated suite remains Prometheus/Loki/Alertmanager; recognized Grafana is informational unless a separately specified compatible mapping exists. A recognition marker does not assert routability or invent separate service credentials.

Configured source identity is the host-qualified tuple (stable service instance, admitted connection ref, canonical configured HTTPS API origin, transport trust/identity-policy revision). Origin means scheme/host/effective port under the selected strict URL binding, with no userinfo, query, fragment or non-root path; ambient/kubeconfig proxies stay disabled. Equivalent accepted origin spellings share the canonical value; display names, kubeconfig context names, aliases and resolved IP addresses are not source identity. TLS trust is receiver configuration, not a value supplied in the discovery request. Changing origin or admitted trust/authority boundary requires a new independently admitted source connection; same-identity credential rotation alone does not merge or reassign it. Different configured instances/connections remain distinct even when their origins or labels coincide; no automatic cross-connection cluster deduplication is selected.

This is deliberately **configured authority identity**, not a globally unique or remotely attested physical-cluster id. The profile does not probe namespaces, nodes, kube-system UID, certificates or arbitrary endpoints to manufacture such an id. Detecting physical replacement behind an unchanged configured authority is not guaranteed by this first profile; any known replacement invalidates old continuity and requires explicit re-admission with a new source binding. Physical identity/automatic migration requires a separately specified bounded attestation and continuity contract before it can be claimed. Provider UID and target equality in §4.3 remain qualified by the selected source, never global identities. This explicit limit disposes of E32 without advertising physical-cluster continuity.


## Extraction and verification

Typed recognition and partition inputs live in
[authored ESS](../../../spec/ess/domains/discovery.yaml).
Native recognizers and exact target verification belong to this adapter; concrete
child installation/selection belongs to a composition. Recognizing an available
provider type does not create a runtime dependency on that provider's adapter.

Required native cases include exact stable Argo marker versus excluded lookalikes,
legacy monitoring precedence, unknown Services, denied namespaces, explicit
all-namespaces selection, changed configured authority, UID/target replacement and
bounded coherent-list exhaustion. Shared publication/history scenarios remain in
the shared contract. These are specification obligations, not executed results.
