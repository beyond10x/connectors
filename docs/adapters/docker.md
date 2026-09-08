# Adapter design: Docker

- **Status:** design, not implemented. New adapter with no predecessor.
- **Old baseline:** `../connectors` at `81459ac4` has no Docker provider, crate, or spec (`rg -li docker` over `*.rs,*.toml,*.md`: 9 files, all incidental: stories, `identity-http`, SQL and Kubernetes tests, CHANGELOG). Nothing is rebuilt; this document derives the surface from the Kubernetes adapter's shape and the Docker Engine API reference, now pinned below for this semantic profile (`docs/design.md:1209`).
- **Contract index:** [contracts/README.md](../../contracts/README.md).

## 1. Scope and placement

One adapter service `connectors.docker` bound to one Docker Engine (daemon) through its API. Placement: on the host that runs the daemon (unix socket) or on a network path to a TLS-enabled daemon endpoint. First slice: read-only inventory, container logs, published-port observations; container lifecycle mutations behind approval; exec and events deferred.

Selected first API binding: **Docker Engine API v1.56**, explicitly version-prefixed with no version fallback. The [official Swagger](https://docs.docker.com/reference/api/engine/version/v1.56.yaml) is pinned by exact bytes in [provider evidence](../evidence/restart-visibility-20260908/provider-evidence.md). Listed HTTP paths and lifecycle response/parameter declarations were checked against that source. A concrete daemon/transport binding must support this version before advertising the proposed operations; no live daemon was tested.

## 2. Old surface and disposition

None. Every row below is new.

## 3. Contracts needed and why

| Contract | Profile / use | Why |
|---|---|---|
| `operations/v1alpha1` | base | describe/invoke |
| `datasource.records` | `docker-list` for containers, images, volumes, networks | provider-native inventory objects with a declared projection; Docker list endpoints are unpaged, so the adapter pages client-side with a snapshot cursor and reports `complete` truthfully |
| `datasource.logs` | `docker-container-logs` | container stdout/stderr lines with timestamps, tail, since, byte bound (`contracts/datasources/logs/v1alpha1/semantics.md`) |
| `endpoint_discovery` (exists) | `docker-published-ports` | published container ports and network attachments as endpoint observations with candidate classification; `namespace` carries the Docker network name; no dial |
| `operations` `mutation` | `container.start`, `container.stop`, `container.restart` | externally visible state changes; `natural` for start/stop desired-state semantics and `none` for restart (§4.1); all remain approval-bound |
| `auth.profile` | `docker.socket`, `docker.mtls` | daemon access is either the unix socket (peer credentials) or TLS client certificate |
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

The target is qualified by the admitted instance/connection and its configured daemon authority, not by a container ID globally. Socket peer/path or TLS endpoint/trust identity belongs to that admitted binding; a known daemon replacement or effect-relevant binding change requires new admission/semantic revision under F02/F03. A full ID observed on another daemon is not this target. The profile does not claim physical-daemon attestation or global cross-daemon deduplication.

After current host operation/input authority and approval verification, one private preflight `GET /v1.56/containers/{expected_id}/json` checks the exact full ID, requested selector and configured ID/name/label allowlist. This is an explicitly admitted bounded metadata read of that target, before the business attempt's dispatch gate, not a hidden action by `host.approval.prepare`. Use at most one inspect, no automatic retry, at most 4 MiB response, inside the original 15 s total provider / 20 s service budget; authorization failure or inability to establish the target prevents business dispatch. Recheck relevant host configuration/authority before the gate. An unprefixed 64-hex selector denotes an ID; use a leading slash to select an exact name with that spelling. Other exact names are not interpreted as ID prefixes. Name comparison is exact after removing at most one leading slash from the daemon's canonical Name and the submitted name; no whitespace/case/Unicode folding. If container is an ID it must equal expected_id; prefix IDs are invalid_input. A missing/replaced ID, name mismatch or denied membership safely refuses before business dispatch, with no retarget or fallback lookup.

Name/label allowlist membership is admitted from this exact preflight observation for this attempt. The lifecycle request is always sent to expected_id. The API does not make the inspect and lifecycle call one atomic predicate over mutable names/labels; concurrent rename or membership change after observation is not claimed to be prevented. A deployment requiring that stronger provider-atomic policy must use a separately proven binding; it cannot infer it from the allowlist or an ESS value. This limit does not permit changing the approved stable target.

Receiver configuration fixes `stop_signal` (default `SIGTERM`, a nonempty ASCII daemon-supported signal token of at most 32 bytes) and `stop_timeout_seconds` (default 10, integer 0–10) for stop/restart. Configuration admission must validate the signal against the selected daemon/platform binding; an unknown signal cannot be advertised as supported. Those resolved values and the exact target are fixed in prepared intent before approval spend/dispatch. They are not caller overrides. Changing them changes the binding/configuration revision and invalidates old approval/fingerprint meaning. Start sends neither option nor detachKeys/checkpoint/attach parameters; stop/restart send exactly the selected signal and t. No unbounded t=-1 is admitted. Preflight consumes the same budget; if the remaining budget cannot accommodate the configured stop wait, refuse before the gate rather than extend the deadline. A post-gate timeout remains unknown even if the daemon later finishes stopping or killing.

| Operation | Idempotency | Selected endpoint observation and result |
|---|---|---|
| container.start | natural | Valid 204 acknowledges start completion; valid 304 reports the endpoint's already-started check. Return `{container_id, desired_state: running, disposition: acknowledged \| already_satisfied}`. The desired state identifies start intent; neither disposition is a synchronized observation that the container is currently running. Both are applied acknowledgement of that intent. |
| container.stop | natural | Valid 204 acknowledges stop completion; valid 304 reports the endpoint's already-stopped check. Return `{container_id, desired_state: stopped, disposition: acknowledged \| already_satisfied}`. The desired state identifies stop intent; neither disposition measures current state or a number of signals/transitions. Both are applied acknowledgement of that intent. |
| container.restart | none | Valid 204 acknowledges the restart request; return `{container_id, restart_accepted: true}`. The API declares no already-restarted/deduplication outcome. A deliberate repeated invocation can restart the same container again. No healthy/ready workload or persistent future state is promised. |

These meanings require the exact selected endpoint/target and a protocol-valid daemon acknowledgement. Malformed answers, 5xx, lost responses or other insufficient effect evidence give unknown/outcome_unknown after possible dispatch. Only binding-verified refusals proving no business effect can give refused; a response status by itself is not a universal proof. Before-gate inspect/configuration/admission failures give not_attempted for a new candidate, subject to current observation admission.

Start then start without intervening state change can yield acknowledged then already_satisfied; stop then stop behaves analogously. A process exit, external actor or daemon restart policy can change state between invocations or between the provider action/check and its HTTP acknowledgement. The result therefore makes no current-state, application-health or future-liveness guarantee; no extra inspect is performed to manufacture one. Another start/stop can perform work after such a change. Natural means the bounded repeat/desired-state semantics for the same exact target and fixed parameters, not historical at-most-once execution. Restart after restart has no repeat-effect guarantee. A later inspect, 304 or successful lifecycle request cannot prove what an earlier lost request did. Each none/natural invocation is a new admitted attempt with required approval and no host key replay. There is no automatic resend, refresh-triggered retry, implicit follow-up health polling or revival of a spent approval.

## 5. Auth

| Profile | Scheme | Acquisition | Capability | Evidence |
|---|---|---|---|---|
| `docker.socket` | `socket_peer` | `static_config` (socket path) | `socket-peer` (owner-checked path, no TCP) | `verify_operation` |
| `docker.mtls` | `mtls` | `static_config` (CA, cert, key refs) | `mtls-client-identity` | `verify_operation` |

Both rows are deployment-supplied static_config activation under [acquisition §4.0](../../contracts/auth/acquisition/v1alpha1/semantics.md#40-acquisition-paths-and-required-declarations), not auth.begin/interactive entry. The socket profile validates the configured peer/transport aperture without manufacturing an OAuth token; mTLS captures a coherent certificate/key generation and uses its reviewed identity validation. Listing performs neither activation nor verification. These proposed transport profiles still require their concrete reader, validation and capability bindings before advertisement.

A daemon socket grants root-equivalent control of the host; the adapter narrows it by allowlists and by advertising only enabled operations. This is containment by configuration, not a sandbox (`docs/design.md:967`).

## 6. Configuration outline

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
