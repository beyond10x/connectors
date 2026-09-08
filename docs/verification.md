# Three adapters: verification record

Verified locally on **2026-09-08** against the executable scope in
[the v1alpha1 contract](../contracts/service/v1alpha1/semantics.md). The three
selected adapters are Kubernetes including discovery, GitLab, and SQL. All eight
advertised operations ran through independent adapter processes and the generic
federation host. This is evidence for this first profile, not for every capability
in the longer design.

## Delivered implementation

| Surface | Responsibility |
|---|---|
| `connectors-core` | Versioned invocation/description/error types, strict JSON decoding, canonical digests |
| `connectors-contracts` | Pages, provenance, endpoint observations, relational results |
| `connectors-sdk` | Abstract adapter, credential, secret-store and authenticated-HTTP ports; schema/cursor helpers |
| `connectors-client` | Provider-independent authenticated HTTP discovery/invocation |
| `connectors-host` | Concrete credential stores, scoped HTTP, admission, logging, limits, serving and federation |
| `connectors-spec` | Connectors-owned adapter-kind validation and deterministic descriptor compilation |
| `connectors-gitlab` | Scoped project metadata, issue pagination and repository file reads |
| `connectors-kubernetes` | Scoped resource lists, EndpointSlice observations, optional node/host discovery |
| `connectors-sql` | PostgreSQL schema discovery and bounded parameterized reads |
| `connectors` | Generic describe/invoke CLI and `serve` federation entry point |
| `connectors-conformance` | Live direct/federated acceptance executable, using generic client calls |

Provider libraries build with `--lib --no-default-features` without host, client,
or sibling provider dependencies. Their default `service` feature supplies binary
wiring. The CLI and host depend on no provider implementation. Concrete file and
memory secret stores satisfy the same injected contract; no provider operation
opens a secret file or owns a logging/storage framework. SQL owns its PostgreSQL
driver and receives an abstract password credential.

## Automated checks

All commands below exited **0**. Rust used for this run was
`rustc 1.98.1 (48a229cea 2026-09-01)` on Linux. This does not establish the declared
minimum Rust version on a separate toolchain.

```sh
cargo fmt --all --check
cargo build --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --locked -p connectors-conformance -p connectors
cargo clippy --locked -p connectors-conformance --all-targets -- -D warnings
```

The workspace run contains **14 passing tests**, with no failed or ignored tests.
[The captured test log](evidence/2026-09-08-workspace-tests.log) distinguishes tests
from empty unit/doc-test targets. It covers strict/duplicate JSON and response
envelopes; canonical descriptors and drift; unsupported spec declarations; service
authentication, schema/version/revision refusal before dispatch; generic federation
success, credential rotation and downstream failure; interchangeable secret stores,
file permissions and symlink refusal; redirect containment, bounded responses and
safe rate-limit errors; provider scope, path encoding, pagination and discovery;
and cursor integrity/context binding. These tests include real HTTP fixtures, not
claims about live vendor availability.

Independent library builds also exited 0:

```sh
cargo build --locked -p connectors-gitlab --lib --no-default-features
cargo build --locked -p connectors-kubernetes --lib --no-default-features
cargo build --locked -p connectors-sql --lib --no-default-features
```

`cargo tree --locked --edges normal --no-default-features -p <adapter>` confirmed
the library boundaries above. `cargo tree --locked --edges normal -p connectors`
confirmed that the generic CLI/host has no adapter dependency.

Each compiler check exited 0 and printed the matching handler obligation count:

```text
gitlab: 3 implemented-binding obligations; descriptor matches
kubernetes: 3 implemented-binding obligations; descriptor matches
sql: 2 implemented-binding obligations; descriptor matches
```

Reproduce with `target/debug/connectors-spec --specification
adapters/<name>/spec/adapter.json --output adapters/<name>/generated/descriptor.json
--check`. Normal Cargo builds consume these repository artifacts and do not fetch
vendor specifications or invoke ESS.

`ess validate --path ess` printed:

```text
connectors v1 — 2 file(s), valid
```

The planning store is validated with `aep plan artifact validate`; its final
implementation/evidence state is recorded in
[story:three-adapters-e2e](../.engineering/planning/story/three-adapters-e2e.md).

The shared sccache initially failed on a temporary-directory quota. Subsequent
successful build/check commands used `RUSTC_WRAPPER=`,
`TMPDIR=/home/timo/.cache/connectors-v2-build-tmp`, and `CARGO_BUILD_JOBS=2`.
This is an environment workaround, not a skipped check or a repository dependency.

## Live upstream verification

The final acceptance command exited **0**, with **nine passing scenario groups**:

```sh
target/debug/connectors-conformance --token-file .local/e2e/service.secret \
  --allow-plaintext
```

[The captured JSON result](evidence/2026-09-08-live-acceptance.json) records discovery
and authentication refusal for every leaf, then successful direct and federated
operation/refusal scenarios for all three providers. The runner source is
[connectors-conformance](../crates/connectors-conformance/src/main.rs).

| Upstream actually exercised | Live proof |
|---|---|
| `https://gitlab.com/api/v4/`, public `gitlab-org/gitlab` project | Project identity, two issue pages, base64 repository file content; out-of-scope project refused |
| Disposable `rancher/k3s:v1.31.5-k3s1` API server | Namespace-scoped services; ready PostgreSQL EndpointSlice with source resourceVersion; node/host observations; out-of-scope namespace refused |
| Disposable `postgres:17-alpine`, reporting PostgreSQL **17.11** | Reader-role schema access, parameter binding, exact decimal, SQL NULL, array and JSON text, empty-result metadata, explicit truncation, oversized-row refusal, mutation/multiple-statement refusal, unchanged fixture row count, database deadline |

Kubernetes used an actual scoped service-account token and verified cluster CA.
GitLab used verified HTTPS with public reads. PostgreSQL used a generated reader
password and explicit plaintext on the isolated local fixture connection. Service
HTTP listeners were on loopback; every call required service admission. GitLab
private-token placement was exercised against HTTP fixtures, not a private live
GitLab account. PostgreSQL TLS configuration is implemented but was not exercised
against a TLS-enabled database in this run.

The integration chain was also executed explicitly: the Kubernetes adapter returned
the fixture PostgreSQL endpoint at `172.17.0.10:5432`; that observed address was
selected into the SQL service configuration with a separately bound reader
credential; `sql__query.read` then returned the fixture rows through the gateway.
No discovery result created a credential or connection by itself.

The final runner checked a five-million-byte SQL value and received `capacity`.
The adapter bounds a row in PostgreSQL before transport, fetches the portal one row
at a time, and bounds the accumulated JSON result. `pg_sleep(30)` returned `timeout`
at the database's ten-second statement deadline on both routes. A first expanded
run exposed a wrong boolean assertion in the test (`t` versus PostgreSQL's
`::text` value `true`); the assertion was corrected after inspecting the actual
typed result, and the full live runner then passed. Array and JSON values remained
intact throughout.

The final four service processes each handled SIGTERM and exited **0**. The prior
processes also handled Ctrl-C/SIGINT and exited 0. The two exact task-owned Docker
fixtures were stopped and removed after verification. The unsuccessful k3d setup
attempt had already rolled itself back; the live proof used direct Docker k3s.
The [reproduction recipe](live-e2e.md) includes fixture resources, role grants,
credential/config creation, the discovery-to-binding step, acceptance and cleanup.

## Evidence scope and remaining design work

[Build hashes](evidence/2026-09-08-build.sha256) identify the tested local binaries
and dependency lock; [source hashes](evidence/2026-09-08-source.sha256) identify the
runtime, contract and declaration files. Build hashes are observations of these
local artifacts, not a claim of cross-machine binary reproducibility.

The first profile has static configured instances, restart-based configuration
activation, expiring in-memory cursors, and one federation hop. Downstream
descriptions are captured at gateway startup; a source revision change requires
refreshing that configured snapshot by restarting the gateway. Credential-file
rotation is separate and works on subsequent calls without restarting. No cache
or dynamic plugin download/build installation path is advertised.

The PostgreSQL contract supports subquery-compatible single-statement reads, not
every PostgreSQL utility statement or other SQL dialect. Non-NULL values use
PostgreSQL `::text` with native column types. Read-only transactions and restricted
database grants are required; functions available to the role remain part of the
database administrator's trust boundary. The limit does not eliminate query cost
or turn arbitrary privileged functions into safe reads.

There is no production tenant control plane, OAuth installation/refresh service,
durable delivery/cache, media/session adapter, OpenAPI-to-executable ESS lowering,
OCI image synthesis, or registry deployment in this slice. The Connectors custom
spec kind and descriptor compiler are implemented; ESS validates the separate
declaration model. Unsettled multi-tenant assignment cardinality stays explicitly
`UNMAPPED`. The work and all planning/evidence remain local. Atlas and the old
Connectors checkout were not integrated or changed by this implementation.
