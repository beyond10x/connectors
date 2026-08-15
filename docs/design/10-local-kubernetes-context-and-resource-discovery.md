# Design 10: local Kubernetes context and resource discovery

**Status:** accepted; personal-local context activation and bounded Service discovery implemented ·
**Date:** 2026-08-15

**Inputs:** [Design 07](07-credential-custody-topologies.md) ·
[Design 08](08-discovery-observations-and-mediated-connections.md) ·
[Kubernetes kubeconfig contexts](https://kubernetes.io/docs/concepts/configuration/organize-cluster-access-kubeconfig/) ·
[SelfSubjectReview](https://kubernetes.io/docs/reference/access-authn-authz/authentication/) ·
[SelfSubjectAccessReview](https://kubernetes.io/docs/reference/access-authn-authz/authorization/) ·
[Kubernetes Services](https://kubernetes.io/docs/concepts/services-networking/service/)

This design lets a personal-local Zwirn installation find the Kubernetes contexts already available
to its user and, after an explicit selection, find supported services such as Grafana. It does not
give an Agent a kubeconfig, enumerate arbitrary cluster objects, or turn observation into authority.

## 1. Two kinds of discovery must not share a lifecycle

```text
standard merged kubeconfig
        │ passive local metadata read; no provider request or auth helper
        ▼
direct-Connection candidates (contexts)
        │ explicit user activation
        │ resolve private auth + SelfSubjectReview + SelfSubjectAccessReview
        ▼
direct Kubernetes Connection
        │ bounded admitted read
        ▼
resource observations (Services)
        │ closed recognition
        ▼
Grafana / Prometheus / Loki / Alertmanager target candidates
        │ target Grant + installed closed route adapter
        ▼
target Connections
```

A kubeconfig context is a candidate for a *source Connection*: no Connection exists yet. A Grafana
Service is an observation *behind an existing Kubernetes Connection*. Calling both things “host
discovery” loses the source boundary and incorrectly suggests that a context or Service grants use.

The generic `ConnectorConnection.candidate_search` and `candidate_activate` methods cover the first
transition. Existing `observation_search` and `materialize` cover the second. A candidate response
contains only an opaque ref, Integration, safe local label, lifecycle, evidence digest, and optional
activated Connection. Cluster server, kubeconfig user, credential mechanism, token, certificate,
client key, exec command, and file path stay Connector-owned.

## 2. Passive means no cluster or credential-helper contact

The trusted personal-local Connector reads the same merged standard kubeconfig selection as normal
Kubernetes clients (`KUBECONFIG`, otherwise the default user config). During startup it parses
context, cluster, user, namespace, and server binding only to form private evidence. It does not:

- contact an API server;
- execute a kubeconfig auth plugin;
- follow a provider login flow;
- publish server, user, auth, or file details; or
- create a Connection merely because a context exists.

Kubernetes warns that a malicious kubeconfig can lead to command execution or file exposure. The
Connector therefore treats exec-based and legacy auth-provider authentication as a separate
effect: `allow_exec_auth = false` by default, and activation refuses such a context unless the user
enabled it in the owner-controlled configuration. Even when enabled, a helper can run only after
`candidate_activate`, never while listing candidates.

Activation re-reads the context and compares its private evidence. A changed cluster, user,
namespace, or server returns `stale_authority`; it cannot silently retarget the passive candidate.
The Connector then resolves authentication, asks the API server for SelfSubjectReview, and refuses
if the authenticated username cannot be established.

## 3. Resource discovery is allowlisted and bounded

The first slice discovers core/v1 Services only. Before each list it submits
SelfSubjectAccessReview for the exact verb, group, version, resource, and namespace scope. An empty
configured namespace list means cluster-wide only when that review permits it. A non-empty list is
checked namespace by namespace; denied namespaces are skipped. At most `resource_limit` objects are
accepted across the scan.

Only Service name and labels are used for the initial closed recognizer:

| Recognized marker | Target Provider |
|---|---|
| `grafana` | `grafana` |
| `prometheus` | `prometheus` |
| `loki` | `loki` |
| `alertmanager` | `alertmanager` |

The public observation contains namespace/name as a human label, normalized type
`kubernetes_service`, target Provider, generation, and digest. The private binding is the Service
identity. Secrets, ConfigMap values, Pod/container environment, workload specs, annotations,
EndpointSlice addresses, external URLs, and network scan results are not read or returned. Future
recognizers for operator CRDs must be separately declared and permission-checked; they do not
justify a generic dynamic-object sweep.

The selected API-server origin must be canonical HTTPS without embedded user information or a URL
tail. Ambient and kubeconfig HTTP proxies are disabled in this slice; a proxy is a second
destination/credential boundary and requires its own reviewed route instead of silently inheriting
process environment.

## 4. Route and topology boundary

A Kubernetes Service is a stable resource identity, but it is not itself an HTTP route from a
personal-local process. Materializing it requires a closed adapter with a fixed Service binding.
For a local Connector the intended adapter is Kubernetes-mediated transport (API-server Service
proxy or a reconciled Service-to-Pod port-forward); request input may not choose namespaces, Service
names, Pods, ports, or proxy suffixes. For an in-cluster satellite, a deployment-owned direct
service DNS route can be selected instead.

This release implements context activation and observation, and refuses `materialize` for these
observations with `unavailable` because that route adapter is not installed yet. It does not label a
partially wired Grafana target callable. Grafana reached directly in a satellite may continue to
discover its Prometheus/Loki/Alertmanager data sources through the existing one-hop Grafana adapter.
The initial one-mediated-hop invariant remains: personal local must not build
Kubernetes → Grafana → Prometheus as two opaque nested mediated routes.

## 5. Harness integration

The local Harness adapter uses the owner-only Connector socket and never opens kubeconfig itself:

```text
Harness settings UI       Connector                         Kubernetes
      │ candidate_search      │                                  │
      ├──────────────────────►│ passive local detection          │
      │ context labels        │                                  │
      ◄───────────────────────┤                                  │
 user selects one             │                                  │
      │ candidate_activate    │ auth + identity + RBAC + list    │
      ├──────────────────────►├─────────────────────────────────►│
      │ Connection + stored observations                         │
      ◄───────────────────────┤                                  │
      │ observation_search    │ no provider I/O                  │
      ├──────────────────────►│                                  │
      │ supported services    │                                  │
      ◄───────────────────────┤                                  │
```

Candidate enumeration belongs in user settings/setup, not the Agent tool surface. The chooser may
filter by Integration and label and stores only opaque candidate/Connection references. Activation
must be a visible user action because it can contact a cluster, refresh cloud credentials, or run an
explicitly permitted auth helper. Resource observations can then feed the normal Explorer filters.
An observation becomes an Agent Endpoint only after target materialization, a current target
Connector Grant, and the Harness's separate Endpoint Grant.

## 6. Personal-local configuration and UX

The configuration contains policy only: source Grant, initiation, optional namespace allowlist,
independent target Grants, auth-exec opt-in, and object limit. It contains no kubeconfig path,
credential value, server URL, context name, or resource binding.

`connectors connect kubernetes` lists detected contexts when selection is ambiguous and performs no
cluster request. `connectors connect kubernetes --context NAME` activates the exact label, verifies
identity/RBAC, creates the Kubernetes Connection, and prints the stored supported Service
observations. Generic clients can use `connection candidates`, `connection activate`, and
`connection observations` over the same credential-free protocol.
