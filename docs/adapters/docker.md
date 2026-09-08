# Adapter design: Docker

- **Status:** design, not implemented. New adapter with no predecessor.
- **Old baseline:** `../connectors` at `81459ac4` has no Docker provider, crate, or spec (`rg -li docker` over `*.rs,*.toml,*.md`: 9 files, all incidental: stories, `identity-http`, SQL and Kubernetes tests, CHANGELOG). Nothing is rebuilt; this document derives the surface from the Kubernetes adapter's shape and the Docker Engine API reference, which must be pinned at authoring (`docs/design.md:1209`).
- **Contract index:** [contracts/README.md](../../contracts/README.md).

## 1. Scope and placement

One adapter service `connectors.docker` bound to one Docker Engine (daemon) through its API. Placement: on the host that runs the daemon (unix socket) or on a network path to a TLS-enabled daemon endpoint. First slice: read-only inventory, container logs, published-port observations; container lifecycle mutations behind approval; exec and events deferred.

Vendor reference to pin: Docker Engine API (`https://docs.docker.com/reference/api/engine/`), exact version selected at authoring. Endpoint names below are the expected ones from that reference and are unverified until pinned.

## 2. Old surface and disposition

None. Every row below is new.

## 3. Contracts needed and why

| Contract | Profile / use | Why |
|---|---|---|
| `operations/v1alpha1` | base | describe/invoke |
| `datasource.records` | `docker-list` for containers, images, volumes, networks | provider-native inventory objects with a declared projection; Docker list endpoints are unpaged, so the adapter pages client-side with a snapshot cursor and reports `complete` truthfully |
| `datasource.logs` | `docker-container-logs` | container stdout/stderr lines with timestamps, tail, since, byte bound (`contracts/datasources/logs/v1alpha1/semantics.md`) |
| `endpoint_discovery` (exists) | `docker-published-ports` | published container ports and network attachments as endpoint observations with candidate classification; `namespace` carries the Docker network name; no dial |
| `operations` `mutation` | `container.start`, `container.stop`, `container.restart` | externally visible state changes; `idempotency: natural` (start of a running container is a no-op on the daemon side) but still approval-bound |
| `auth.profile` | `docker.socket`, `docker.mtls` | daemon access is either the unix socket (peer credentials) or TLS client certificate |
| `auth.capability` | `socket-peer`, `mtls-client-identity` | the host HTTP client must speak HTTP over a unix socket or with a client certificate (`crates/connectors-host/src/http.rs`, URL-based reqwest today) |
| `auth.evidence` | `verify_operation` = `_ping`/`version` | daemon reachability without effects |
| `auth.connection` | `configured` | one daemon per instance |
| `auth.custody` | `read_only` | certificate and key as file references |

Not needed: acquisition flows, discovery of resources with opaque locators, mediated routes (a daemon does not proxy to other services), sessions/media.

## 4. Operation map

| New id | Contract / profile | Effects | Expected daemon endpoint (verify) |
|---|---|---|---|
| `containers.list` | records `docker-list` | read | `GET /containers/json` |
| `images.list` | records `docker-list` | read | `GET /images/json` |
| `volumes.list` | records `docker-list` | read | `GET /volumes` |
| `networks.list` | records `docker-list` | read | `GET /networks` |
| `container.inspect` | records single item | read | `GET /containers/{id}/json` |
| `container.logs` | logs `docker-container-logs` | read | `GET /containers/{id}/logs` |
| `ports.discover` | endpoint_discovery `docker-published-ports` | none | derived from `containers.list` and `networks.list` |
| `container.start` | mutation, natural idempotency, approval required | external_write | `POST /containers/{id}/start` |
| `container.stop` | mutation, natural, approval required | external_write | `POST /containers/{id}/stop` |
| `container.restart` | mutation, `idempotency: none`, approval required | external_write | `POST /containers/{id}/restart` |

Container selection is by id or name and must match a configured allowlist (ids, names, or label selectors) before dispatch.

## 5. Auth

| Profile | Scheme | Acquisition | Capability | Evidence |
|---|---|---|---|---|
| `docker.socket` | `socket_peer` | `static_config` (socket path) | `socket-peer` (owner-checked path, no TCP) | `verify_operation` |
| `docker.mtls` | `mtls` | `static_config` (CA, cert, key refs) | `mtls-client-identity` | `verify_operation` |

A daemon socket grants root-equivalent control of the host; the adapter narrows it by allowlists and by advertising only enabled operations. This is containment by configuration, not a sandbox (`docs/design.md:967`).

## 6. Configuration outline

```json
{ "service": { "$ref": "urn:connectors:config:v1:service" },
  "adapter": {
    "daemon": { "socket": "/var/run/docker.sock" },
    "containers": { "allow_names": ["api-*"], "allow_labels": { "app": "api" } },
    "logs": { "enabled": true, "max_bytes": 131072 },
    "writes": { "enabled": false, "operations": ["container.restart"] },
    "discover_ports": true } }
```

`daemon` is either `socket` or `tls: { endpoint, ca_file, cert: credential ref, key: credential ref }`, never both.

## 7. Discovery and routes

`ports.discover` emits `EndpointObservation` items: `namespace` = network name, `service` = container name, `address` = host bind address or container IP, `port`, `transport`, `application_protocol` from image labels or port conventions marked as candidates, `source_uid` = container id, `reachability` = `host_local` or `network_local`. No dial, no credential.

## 8. Specification profile

`connectors.adapter/v1` handwritten. Docker publishes its Engine API description as a Swagger 2.0 document (to verify at pinning); the v2 generation profile accepts OpenAPI 3.0/3.1 GET only (`spec-kinds/adapter/v2/semantics.md:13-15`), so generation waits for either a converted source or a wider profile.

## 9. Deferred

`execution` (container exec, `docs/design.md:462`), `events` (`GET /events` stream), image pull/build (supply-chain trust decision), compose/stack management, log follow (a `sessions` stream).

## 10. Evidence required

- Fixture: a fake daemon over a unix socket; list paging snapshot; log timestamps and truncation; allowlist refusal before dispatch; mutation approval and replay; socket permission failure → `Unavailable`, not `Forbidden`.
- Live: a local daemon with one allowed container: list, logs, discover ports; restart with approval; verify no effect when disabled.
- Decoupling: builds without any other adapter; host gains unix-socket transport tested independently of Docker.
