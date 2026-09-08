# Adapter design: Docker

Exact container lifecycle intent is typed in the [adapter-owned ESS model](spec/ess/domains/mutations.yaml).
The shared mutation ledger consumes effect knowledge without importing this provider vocabulary.

- **Status:** design, not implemented. New adapter with no predecessor.
- **Old baseline:** `../connectors` at `81459ac4` has no Docker provider, crate, or spec (`rg -li docker` over `*.rs,*.toml,*.md`: 9 files, all incidental: stories, `identity-http`, SQL and Kubernetes tests, CHANGELOG). Nothing is rebuilt; this document derives the surface from the Kubernetes adapter's shape and the Docker Engine API reference, now pinned below for this semantic profile (`docs/design.md:1209`).
- **Contract index:** [contracts/README.md](../../contracts/README.md).

## 1. Scope and placement

One adapter service `connectors.docker` bound to one Docker Engine (daemon) through its API. Placement: on the host that runs the daemon (unix socket) or on a network path to a TLS-enabled daemon endpoint. First slice: read-only inventory, container logs, published-port observations; container lifecycle mutations behind approval; exec and events deferred.

Selected first API binding: **Docker Engine API v1.56**, explicitly version-prefixed with no version fallback. The [official Swagger](https://docs.docker.com/reference/api/engine/version/v1.56.yaml) is pinned by exact bytes in [provider evidence](../../docs/evidence/restart-visibility-20260908/provider-evidence.md). Listed HTTP paths and lifecycle response/parameter declarations were checked against that source. A concrete daemon/transport binding must support this version before advertising the proposed operations; no live daemon was tested.

## 2. Old surface and disposition

None. Every row below is new.

## 3. Contracts needed and why

| Contract | Profile / use | Why |
|---|---|---|
| `operations/v1alpha1` | base | describe/invoke |
| `datasource.records` | `docker-list` for containers, images, volumes, networks | provider-native inventory objects with a declared projection; the pinned v1.56 declarations expose no continuation/page-offset parameter for these four endpoints (container list does have a result `limit`). This is API-shape evidence, not proof that a daemon result is exhaustive. Any client-side snapshot paging must specify bounded collection and truthful completeness before advertisement |
| `datasource.logs` | `docker-container-logs` | container stdout/stderr lines with timestamps, tail, since, byte bound (`contracts/datasources/logs/v1alpha1/semantics.md`) |
| `endpoint_discovery` (exists) | `docker-published-ports` | published container ports and network attachments as endpoint observations with candidate classification; `namespace` carries the Docker network name; no dial |
| `operations` `mutation` | `container.start`, `container.stop`, `container.restart` | externally visible state changes; `natural` for start/stop desired-state semantics and `none` for restart (§4.1); all remain approval-bound |
| `auth.profile` | `docker.socket`, `docker.mtls` | daemon access is either the unix socket (peer credentials) or TLS client certificate |
| `auth.acquisition` | `static_config` | separately admitted deployment activation of the socket aperture or coherent TLS credential references; no interactive acquisition flow |
| `auth.capability` | `socket-peer`, `mtls-client-identity` | the host HTTP client must speak HTTP over a unix socket or with a client certificate (`crates/connectors-host/src/http.rs`, URL-based reqwest today) |
| `auth.evidence` | `verify_operation` = `_ping`/`version` | daemon reachability without effects |
| `auth.connection` | `configured` | one daemon per instance |
| `auth.custody` | `read_only` | certificate and key as file references |

Not needed: acquisition flows, discovery of resources with opaque locators, mediated routes (a daemon does not proxy to other services), sessions/media.

## 4. Operation map

| New id | Contract / profile | Effects | Pinned daemon endpoint (under /v1.56) |
|---|---|---|---|
| `containers.list` | records `docker-list` | read | `GET /containers/json` |
| `images.list` | records `docker-list` | read | `GET /images/json` |
| `volumes.list` | records `docker-list` | read | `GET /volumes` |
| `networks.list` | records `docker-list` | read | `GET /networks` |
| `container.inspect` | records single item | read | `GET /containers/{id}/json` |
| `container.logs` | logs `docker-container-logs` | read | `GET /containers/{id}/logs` |
| `ports.discover` | endpoint_discovery `docker-published-ports` | none | derived from `containers.list` and `networks.list` |
| `container.start` | mutation, natural idempotency, approval required | external_write, network | `POST /containers/{id}/start` |
| `container.stop` | mutation, natural, approval required | external_write, network | `POST /containers/{id}/stop` |
| `container.restart` | mutation, `idempotency: none`, approval required | external_write, network | `POST /containers/{id}/restart` |

Container selection retains full ID or exact name, with an explicit stable-identity expectation for writes. The closed lifecycle input is `{container, expected_id}`: `container` is either a full ID or exact name (no prefix/wildcard), and `expected_id` is the daemon's full lowercase 64-hex container ID. Name and label selection remain available through separately admitted inventory/inspect and receiver-configured allowlists; a read result supplies an expectation, never authority. A caller choosing a name first obtains its full ID through such a read. The write cannot silently follow a recreated name. No caller supplies a daemon endpoint, credentials, label-policy override or provider request options.

### 4.1 Target, intent and lifecycle results

The [native mutation binding](contracts/mutations/v1alpha1/semantics.md) owns exact
target/intent preparation, dispatch predicates and lifecycle outcome/replay rules.

## 5. Auth

| Profile | Scheme | Acquisition | Capability | Evidence |
|---|---|---|---|---|
| `docker.socket` | `socket_peer` | `static_config` (socket path) | `socket-peer` (owner-checked path, no TCP) | `verify_operation` |
| `docker.mtls` | `mtls` | `static_config` (CA, cert, key refs) | `mtls-client-identity` | `verify_operation` |

Both rows are deployment-supplied static_config activation under [acquisition §4.0](../../contracts/auth/acquisition/v1alpha1/semantics.md#40-acquisition-paths-and-required-declarations), not auth.begin/interactive entry. The socket profile validates the configured peer/transport aperture without manufacturing an OAuth token; mTLS captures a coherent certificate/key generation and uses its reviewed identity validation. Listing performs neither activation nor verification. These proposed transport profiles still require their concrete reader, validation and capability bindings before advertisement.

A daemon socket grants root-equivalent control of the host; the adapter narrows it by allowlists and by advertising only enabled operations. This is containment by configuration, not a sandbox (`docs/design.md:967`).

## 6. Configuration outline

Illustrative proposed configuration: socket, allowlist, enablement, signal and timeout values are examples, not implied defaults. The `logs.max_bytes: 131072` value is the selected default and ceiling in the [native log binding](contracts/logs/v1alpha1/semantics.md). Lifecycle timing remains bounded by the [native mutation contract](contracts/mutations/v1alpha1/semantics.md); this outline cannot enlarge those limits.

```json
{ "service": { "$ref": "urn:connectors:config:v1:service" },
  "adapter": {
    "daemon": { "socket": "/var/run/docker.sock" },
    "containers": { "allow_names": ["api-*"], "allow_labels": { "app": "api" } },
    "logs": { "enabled": true, "max_bytes": 131072 },
    "writes": { "enabled": false, "operations": ["container.restart"], "stop_signal": "SIGTERM", "stop_timeout_seconds": 10 },
    "discover_ports": true } }
```

`daemon` is either `socket` or `tls: { endpoint, ca_file, cert: credential ref, key: credential ref }`, never both.

## 7. Discovery and routes

`ports.discover` emits `EndpointObservation` items: `namespace` = network name, `service` = container name, `address` = host bind address or container IP, `port`, `transport`, `application_protocol` from image labels or port conventions marked as candidates, `source_uid` = container id, `reachability` = `host_local` or `network_local`. No dial, no credential.

## 8. Specification profile

`connectors.adapter/v1` handwritten. The pinned Docker v1.56 source is Swagger 2.0; the v2 generation profile accepts OpenAPI 3.0/3.1 GET only (`spec-kinds/adapter/v2/semantics.md:13-15`), so generation waits for either a converted source or a wider profile.

## 9. Deferred

`execution` (container exec, `docs/design.md:462`), `events` (`GET /events` stream), image pull/build (supply-chain trust decision), compose/stack management, log follow (a `sessions` stream).

## 10. Evidence required

- Fixture: a fake daemon over a unix socket; list paging snapshot; log timestamps and truncation; allowlist refusal before dispatch; mutation approval and replay; socket permission failure → `Unavailable`, not `Forbidden`.
- Live: a local daemon with one allowed container: list, logs, discover ports; restart with approval; verify no effect when disabled.
- Decoupling: builds without any other adapter; host gains unix-socket transport tested independently of Docker.

Lifecycle operation visibility follows the [extended matrix](../../contracts/service/v1alpha2/semantics.md#321-visibility-lookup-and-refusal-precedence-e12): implemented-but-disabled writes are hidden from describe and give forbidden only after admitted current-revision private lookup. Missing caller/result authority wins before key/existence disclosure. A stale description is not dispatch permission; dependency readiness and a one-shot approval remain invocation checks.
