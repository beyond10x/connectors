# auth.custody/v1alpha1

- **Status:** proposed, not implemented. Extends the existing `SecretStore` port (`crates/connectors-sdk/src/lib.rs:42-44`, `read` only).
- **Family:** auth. Sibling documents: [connection](../../connection/v1alpha1/semantics.md), [profile](../../profile/v1alpha1/semantics.md), [acquisition](../../acquisition/v1alpha1/semantics.md), [capability](../../capability/v1alpha1/semantics.md), [evidence](../../evidence/v1alpha1/semantics.md).
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `auth.custody/v1alpha1` |
| Profiles | `read_only` (today: env/file-backed references resolved at dispatch), `versioned` (write, read, delete immutable versions; publication and refresh coordination held by the host metadata store) |
| Parties | host (grants a scoped store capability), store binding (in-memory, owner-only file, later Vault or database), coordinator (writer), execution boundary (reader) |

Design responsibility three of four: persist sensitive material through an injected secret-store binding (`docs/design.md:528`). The store knows nothing about provider identity, OAuth, expiry, or refresh (`docs/design.md:576`).

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| Four custody crates: `connector-secrets`, `hosted-secrets`, `hosted-vault`, `subscription-custody` | `../connectors/crates/` listing | change: one port, several bindings; subscription custody deferred |
| "Never read" means no retrieval surface: no request can select a credential source, store, Vault address, mount, role, item, or fallback; no response reveals one | `../connectors/docs/design/07-credential-custody-topologies.md:101-120`; `contracts/connector-connection/v0alpha1/README.md:7-11` | preserve |
| Prepared multi-key transaction protocol required of all backends | `docs/design.md:578` | remove from opaque custody: immutable versions and declared durability; host metadata separately provides the atomic coordination required by the selected acquisition profile |
| Current binding: `CredentialRef` env or file; files bounded, regular, owner-readable only, not symlinks; rotation by replacing the file | `crates/connectors-host/src/credentials.rs`; `contracts/service/v1alpha1/semantics.md`, Auth and configuration | preserve as `read_only` profile |
| `Secret` type with no `Debug`/`Serialize` | `crates/connectors-sdk/src/lib.rs:33-34` | preserve |

## 3. Types

Port (specializes `docs/design.md:568-572`):

```text
write_new(scope, SensitiveMaterial) -> SecretVersionRef
read(scope, SecretVersionRef)       -> SensitiveMaterial | Missing | Unavailable | Denied
delete(scope, SecretVersionRef)     -> Deleted | Missing | Unavailable | Denied
```

Host metadata (not in the store):

```json
{ "connection": "conn_…", "active_credential": "ver_…", "auth_publication_fence": 12, "superseded": ["ver_…"] }
```

```text
publish_active(connection, expected_auth_fence, new_version) -> Published(auth_fence) | Conflict(current_auth_fence)
// Baseline host operation. Rotating refresh requires the guarded transaction in
// auth.acquisition §4.1; this three-argument CAS alone is insufficient.
// Private concurrency fence, not public connection/F02 semantic revision.
```

`SensitiveMaterial` is an opaque bounded byte string per version. A credential *set* (access plus refresh token, or user plus secret half) is one version, so the pair is written and read together (`docs/design.md:576`). Interpretation of the bytes belongs to the provider auth implementation.

`scope` is host-assigned per adapter instance and connection; the store maps `(scope, version)` to its own layout. Public invocations never carry a scope or version reference (`docs/design.md:574`).

Outcomes are distinct: `Missing` (never written or deleted), `Unavailable` (backend outage), `Denied` (scope mismatch). Callers map `Unavailable` to `custody_unavailable` in connection status; `Denied` is an internal invariant failure and is logged without the reference.

## 4. Rules

- Versions are immutable: a written version is never modified; rotation writes a new version and publishes it.
- Write before publish: `write_new` completes durably before `publish_active`; a crash between the two leaves an unreferenced version that cleanup reclaims. Dispatch readers use only the current admitted generation and its pinned snapshot; coordinator validation may read a private candidate before publication. An orphan never becomes dispatch authority.
- Host publication: a baseline `publish_active` carries the expected revision; a concurrent publish loses with `Conflict` and re-reads. For rotating refresh, the host must instead fulfill [acquisition §4.1](../../acquisition/v1alpha1/semantics.md): per-generation reservation, durable single exchange authorization, fencing and guarded publication serialized with revocation. None is a secret-store lease or an operation on opaque material.
- Refresh publication additionally binds the new custody version to a new private credential generation validated under [evidence](../../evidence/v1alpha1/semantics.md). It atomically updates the active reference and invalidates old-generation dispatch admissions. Custody retention never authorizes dispatch or a second exchange. A material version can remain readable for an already-valid use without remaining refreshable.
- Failure between provider exchange and durable response registration never permits a repeat exchange. An unreferenced candidate version is not proof that refresh did or did not happen. Recovery uses the coordinator ledger; ambiguous candidates are discarded/repair is required. Secret garbage collection must not erase the ledger fact that a source was consumed.
- Reclaim: superseded versions are deleted once no valid use requires them, after a declared retention (first-profile default 24 h; refresh-token grace behavior is provider-specific and declared in the profile).
- Scoped access: a capability handed to an adapter reads only its own scope. There is no list operation over values.
- Bounded values: a version is at most 64 KiB (first-profile default; certificates with chains may need more and set it per binding).
- No private references in diagnostics: read errors, ordinary logs and metrics expose neither values nor custody scope/version/generation ids. Use separately admitted safe correlation and error codes. Access-controlled internal custody state is not a diagnostic export.
- Durability claims per binding: in-memory claims none; file binding claims fsync-before-return on the owner-only file; a database or Vault binding declares its own. A binding declares the guarantees it does not provide instead of emulating them (`docs/design.md:578`).
- Startup: the host refuses to start an adapter whose required custody binding is missing rather than falling back to another binding (`docs/design.md:816`).

## 5. Limits

| Concern | Rule |
|---|---|
| Value size | 64 KiB default |
| Versions per connection | bounded (default 8 retained including active) |
| Read latency budget | part of the provider deadline; a read exceeding it is `Unavailable` |
| Retry | reads may be retried by the caller; writes are not retried after an unknown outcome without a new version id |

## 6. Conformance scenarios (`docs/design.md:986`)

- Write, publish, read: read returns the bytes; a second write creates a new version; the old version stays readable until deleted.
- Scoped: a capability for scope A reading a version of scope B → `Denied`.
- Missing versus outage: deleted version → `Missing`; binding fault injected → `Unavailable`; the caller-facing status differs.
- Concurrent baseline publish with the same expected revision → one `Published`, one `Conflict`.
- Rotating refresh uses the acquisition failure matrix: concurrent exchanges are prevented before send, stale owners cannot publish, revocation blocks resurrection, and custody outages never reopen consumed authorization.
- Crash between write and publish (file binding): after restart the active reference is unchanged and the orphan is reclaimable.
- Serialize every error and log line produced by the suite; grep for the fixture's secret bytes → no match.
- Same suite runs against the in-memory and file bindings; provider code untouched (`docs/design.md:1014`).

## 7. Compatibility

- The `read_only` profile keeps today's env/file `CredentialRef` behavior; adapters using it change nothing.
- Migration of old credential sets is a scoped custody migration or reauthorization, never a copy through a document or command line (`docs/design.md:1116`).
- [Service compatibility](../../../service/compatibility.md) is authoritative for the binding. Private custody values and CAS ports are not public service messages. An unchanged read-only port can retain legacy behavior; versioned custody requires its own selected infrastructure binding.


## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| `SecretStore` gains `write_new`, `delete`, and a scope parameter | `crates/connectors-sdk/src/lib.rs:42-44` |
| Host metadata store with baseline `publish_active` CAS and the stronger rotating-refresh coordinator protocol in acquisition §4.1 | new host module (with `auth.connection` metadata) |
| File binding: one file per version, owner-only, fsync, atomic rename | `crates/connectors-host/src/credentials.rs` |
| In-memory binding for tests | `crates/connectors-conformance` |

## 9. ESS entities

| Entity | Notes |
|---|---|
| Immutable custody version / credential set | Proposed opaque material version, not an ESS entity. Active/superseded are host reference facts, not a declared custody lifecycle; Connection relation/cardinality/delete semantics remain UNMAPPED |
| Active credential reference | Proposed ConnectionAuthorityPort field with private publication fence, not a declared ESS Connection relation; publication couples generation, refresh ownership and revocation |
| Refresh coordination | [RefreshAttempt ESS](../../../../ess/domains/refresh.yaml), owned by acquisition/coordinator; not a custody entity or port |
| External store layout, Vault paths | UNMAPPED, binding-private |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Second binding after file | SQLite through the same port tests (old repo had `state-sqlite`); Vault later |
| Retention of superseded versions | 24 h |
| Value bound | 64 KiB, per-binding override for certificate chains |

Persistence ownership is consolidated in [design §31](../../../../docs/design.md#31-host-persistence-ownership-and-atomicity). CustodyPort owns immutable sensitive versions; ConnectionAuthorityPort/RefreshCoordinatorPort own active references, publication and consumed-source history. This inventory does not supply a backend or execute its atomicity predicates.
