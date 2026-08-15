# Design 07: credential custody follows execution placement

**Status:** draft for cross-repository decision; implementation open · **Date:** 2026-08-14

**Inputs:** [Design 01](01-domain-model.md) · [Design 02](02-architecture.md) ·
[Design 06](06-personal-slack-socket-mode.md) ·
[`b10x/architecture` RFC 0004](https://github.com/b10x/architecture/blob/main/rfcs/0004-satellite-federation.md) ·
[`b10x/architecture` proposed RFC 0013](https://github.com/b10x/architecture/blob/main/rfcs/0013-credential-custody-follows-connector-placement.md)

This design fixes how a Connection can use an entered secret, an external secret provider, or no
reusable secret across personal, central, and satellite deployments. The result stays one generic
Connection contract. A provider such as Grafana does not choose the topology, and a caller cannot
choose it at invocation time.

## 1. Placement and custody are separate decisions

A Connection answers which authorized provider instance exists. Its execution placement answers
which Connectors process performs the operation. Its credential binding answers how that process
satisfies the Provider's declared credential requirements.

```text
Connection + Grant + declared operation
                 │
                 ▼
       execution placement
       local | central | satellite
                 │
                 ▼
        credential binding
 workload identity | managed store | external secret provider
                 │
                 ▼
      fixed destination aperture
```

Credential locality follows execution locality. A satellite operation resolves authority at the
satellite. A personal-local operation resolves it on the developer's machine. Central Connectors
does not receive a satellite or personal credential merely because it coordinates the request.

## 2. Credential store and secret provider

`connector-secrets::SecretStore` remains the internal port used by planning and dispatch. A
deployment may bind that port in three ways:

| Binding | What exists | Where it lives |
|---|---|---|
| `workload_identity` | no reusable vendor secret in Connector custody; the platform authenticates the workload or an auth proxy maps it to a provider identity | execution environment |
| `managed_store` | an entered or acquired credential sealed by a Connector-owned backend | OS keychain for personal-local; deployment-managed encrypted store for central or satellite |
| `secret_provider` | an opaque binding to a credential held by an external system such as 1Password | secret remains in that provider; the executing Connector holds only its constrained binding and provider-session custody |

The public term is **credential source**. `SecretStore` and secret-provider adapter are
implementation terms. They are not new model-callable provider operations.

A secret provider may be configured through the ordinary Integration and Connect Session
experience: for example, a user connects 1Password, then selects one approved item field as the
credential source for a Grafana Connection. That reuse stops at the control plane. Runtime secret
resolution is a dedicated, non-projectable credential-source port; the catalog must never expose
an operation such as arbitrary `secret.read` to a harness or model.

Bootstrap must terminate rather than recurse forever. A local 1Password binding can rely on the
already-unlocked desktop/OS broker and user presence. A deployed secret provider uses workload
identity or one deployment-sealed bootstrap credential. That bootstrap authority has its own
destination and scope aperture and is never borrowed by a provider operation.

## 3. “Never read” means no retrieval surface

An entered secret must be resolvable inside the executing Connector long enough to authenticate a
declared provider request. “Never read” therefore cannot mean that no trusted runtime ever sees the
value. It means:

- no client, harness, model, Connection response, catalog response, audit record, error, log,
  argument, generic environment, or relational row can retrieve it;
- the management surface supports acquire, bind, rotate, revoke, and delete, but has no
  credential-value `get`, export, list-value, or echo operation;
- resolution happens only after principal, Connection, Grant, operation, current generation, and
  destination checks, into guarded memory registered for redaction before dispatch;
- a secret-provider locator is itself hidden behind an opaque binding reference because vault,
  item, or field names may disclose sensitive metadata.

Connection search and describe may report only a non-secret custody summary such as
`workload identity`, `local keychain`, `external provider`, or `deployment store`, plus health and
reauthorization state.

## 4. Supported topologies

### Personal-local, pasted once

```text
human -> one-use local completion socket -> local Connector -> OS keychain
harness -> opaque Connection + Grant ----> local Connector -> provider
```

The first-party completion UI prompts with echo disabled and writes directly to an owner-only
Connector completion socket. The harness receives neither that endpoint nor the value. The local
Connector seals the value in the OS keychain; no plaintext credential file or environment variable
is the release fallback.

### Personal-local, external credential source

```text
human -> connect 1Password / approve item binding
                         │
                         ▼
harness -> local Connector -> constrained secret-provider adapter -> provider
             │                         │
             └ opaque binding only     └ credential stays in 1Password
```

The developer owns the secret-provider session. User-presence and unlock policy remain provider
facts. Connectors may cache only a bounded provider session when the provider permits it; it never
copies the selected provider credential into ordinary local configuration.

### Remote central or satellite, stored at execution

```text
human completion client
  -> encrypt to target-generated one-use key
  -> central control plane carries ciphertext
  -> outward satellite control connection
  -> satellite consumes once and commits to its encrypted store

harness -> central Connectors -> signed operation -> satellite -> private provider
```

This preserves RFC 0004: the satellite still has no inbound management listener and federation is
not a generic tunnel. A Connect Session is assigned to one execution placement. The target creates
the ephemeral completion key; the trusted human-facing client encrypts the submission to it;
central can route and audit the opaque envelope but cannot decrypt it. The satellite accepts one
valid completion while its control lease and signed generation are current, commits the value and
value-free Connection metadata atomically, destroys the completion key, and returns only terminal
status plus the durable Connection reference.

The same target-sealed flow can bind an external secret-provider reference rather than store a
value. Rotation replaces the sealed value or binding in place; it does not change Connection id.

### Satellite with workload identity

```text
harness -> central Connectors -> signed operation -> satellite workload
                                                  -> auth proxy / service identity
                                                  -> stable private service
```

There is no reusable provider credential to enter. The workload or proxy identity is restricted to
the Integration's destination and provider role. In Kubernetes, the destination is a stable
Service such as `grafana.monitoring.svc.cluster.local`, never a pod address. NetworkPolicy and the
deployment destination aperture admit only the required service and port; post-resolution checks
remain mandatory. A network-restricted anonymous Viewer may prove a development path, but an
accountable workload identity is the release preference.

### OAuth2 acquisition

OAuth2 is an acquisition method, not a storage topology. The human browser authorizes once; the
Connector assigned to the Connection owns PKCE/state, performs the token exchange, stores or binds
the resulting tokens at that execution placement, refreshes them, and exposes only terminal
Connection status.

For a satellite without an inbound listener, central may terminate the public callback and forward
the one-session authorization response, but the target retains the PKCE verifier and performs the
token exchange. A provider that cannot support that declared custody chain is refused for that
placement rather than silently falling back to central token custody. Provider browser SSO is not
assumed to be an API credential.

## 5. One product journey

The product can present four choices without exposing the protocol:

| Choice | Result |
|---|---|
| **Use organization connection** | Select a tenant-shared Connection at its configured central or satellite placement. No credential prompt appears when workload identity is available. |
| **Paste a credential securely** | The trusted completion surface stores it in the executing Connector's keychain or encrypted deployment store. |
| **Use my credential provider** | Connect or select an approved secret provider, then bind one credential without copying it into the harness. |
| **Sign in with the provider** | Complete OAuth2 when the Provider declares a compatible flow for the selected placement. |

All four finish with the same user-visible object: a named Connection. The ordinary harness sees an
opaque Connection reference, available operations, and current admission—not the acquisition
mechanism, completion endpoint, secret binding, or destination.

## 6. Developer and web-product journeys are different

The same Connection contract must not force the same setup experience onto fundamentally different
users:

| Concern | Developer harness | Hosted web product |
|---|---|---|
| Ordinary owner | one developer and their local Connector | organization operator or an end user represented by the product |
| Natural execution | personal-local, with explicit organization/satellite Connections when needed | central or satellite; personal-local only through an explicitly paired local Connector |
| Natural credential source | OS keychain or the developer's credential provider | deployment-managed encrypted store, workload identity, or organization secret provider |
| Natural acquisition | protected local prompt, credential-provider approval, loopback/device OAuth | hosted Connect Session, browser OAuth, or target-sealed remote entry |
| Connection scope | usually principal-owned | tenant-shared for organization services or principal-owned for delegated user access |

A browser product cannot silently reach a developer's keychain or local 1Password session. Doing so
requires a separately installed and deliberately paired personal Connector, which remains the
execution and custody placement for that Connection. Conversely, a developer harness does not gain
organization secrets because it can address a central or satellite Connector; the remote
Connection and Grant remain independent gates.

The UX may therefore differ substantially while search, describe, invoke, event, lifecycle, and
Grant semantics remain portable.

## 7. Deployment configuration owns available stores

Credential-source selection belongs to Connectors-owned deployment configuration. If a larger
product has one common configuration document, this section remains an owned Connectors subtree;
the harness or web product may reference a Connection policy but cannot define a secret backend or
carry a secret value.

The target `platform.toml` shape is:

```toml
[[credential_sources]]
id = "personal-keychain"
driver = "os-keychain"
placements = ["personal"]
connection_scopes = ["principal"]

[[credential_sources]]
id = "organization-vault"
driver = "secret-provider"
provider_connection = "connection:organization-vault"
placements = ["central", "satellite:grafana-dev"]
connection_scopes = ["tenant"]

[[credential_sources]]
id = "cluster-workload"
driver = "workload-identity"
placements = ["satellite:grafana-dev"]
connection_scopes = ["tenant"]
```

These are non-secret policy bindings. A `managed-store` driver receives its encryption/key
configuration through deployment-owned sealed inputs, never inline values. A `secret-provider`
source names an opaque provider Connection, not a vault item; item/field binding happens inside a
Connect Session and remains hidden. The dependency graph must terminate in workload identity or a
managed sealed store and must be acyclic.

Each Integration declares the credential-source drivers and acquisition flows it permits. The
deployment configuration narrows those declarations to named available sources and placements.
Connection setup chooses from that intersection; an invocation request cannot override it.

There is no cross-source fallback. An unavailable keychain, secret provider, deployment store, or
workload identity degrades the affected Connection and asks for repair. Changing the configured
default does not migrate, copy, or rebind existing credentials automatically.

Satellite configuration receives only the subset bound into its signed deployment generation. A
source added to central configuration is not usable at a satellite until a higher signed generation
admits its driver, placement, scope, and destination aperture.

## 8. Required implementation boundaries

- The Integration declares allowed credential-source kinds and placement compatibility. A caller
  cannot invent a store, provider, destination, or acquisition flow.
- Connect Session creation binds tenant, principal or tenant-shared scope, Integration, execution
  placement, allowed acquisition kind, expiry, generation, and one completion action.
- Local completion is owner-only. Remote completion is target-encrypted before crossing central.
- The credential management API is write/rotate/revoke/delete-only with value-free status; there is
  no value retrieval endpoint.
- Secret-provider authorization and secret resolution are separate capabilities. Provider/model
  operation Grants cannot invoke the latter.
- The satellite rechecks its current control lease, signed generation, tenant, placement,
  Connection, Grant, credential binding, and destination before resolution or provider I/O.
- Failure never changes credential source or placement. Repair creates a new Connect Session and
  reauthorizes the stable Connection in place.
- Tests use sentinels to prove credentials and derived forms are absent from responses, logs,
  audits, argv, environment, relational state, crash artifacts, and central relay plaintext.

## 9. Delivery slices

The hosted managed-store foundation is implemented: Cloud deploys internal TLS Vault KV v2, binds
the Connectors ServiceAccount through Kubernetes auth to one tenant prefix, and the hosted
Connector composes that store with fail-closed startup plus in-memory token refresh. This does not
complete external secret-provider Connections, remote target-sealed entry, or the browser product
flow below.

1. Replace the development `FileStore` release path with an OS-keychain backend while preserving
   the existing prepared transaction and crash-recovery contract.
2. Add credential-source bindings and one constrained mock secret-provider adapter; prove no
   generic secret operation reaches the operation catalog.
3. Extend Connect Session completion with placement binding and target-sealed remote envelopes.
4. Compose the accepted satellite federation contract with deployment-managed storage and one
   workload-identity Integration.
5. Add provider-declared OAuth custody compatibility and exercise local and satellite token
   exchange without changing the Connection contract.
