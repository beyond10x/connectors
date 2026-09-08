# Connectors v2

Independent Rust adapter services for **Kubernetes discovery, GitLab, and SQL**,
consumed directly or through a provider-independent federation host.

This is a local implementation of the first data/discovery profile. It is not registered in Atlas.
The initial end-to-end implementation is tracked in
[the three-adapter story](.engineering/planning/story/three-adapters-e2e.md).
The subsequent [GitLab generation story](.engineering/planning/story/gitlab-spec-service.md)
adds specification-driven runtime generation and local container packaging.
See [generation and packaging](docs/gitlab-generation.md) for that workflow.
See [the full design](docs/design.md) and
[the executable first-slice contract](contracts/service/v1alpha1/semantics.md).
Run [the live acceptance recipe](docs/live-e2e.md) and read
[the recorded verification and limits](docs/verification.md).

## Build and test

Rust 1.88.0 is the checked minimum. The full test/generation gate uses the ESS pin in
[`crates/connectors-spec/toolchain.json`](crates/connectors-spec/toolchain.json)
and the rustfmt recorded in the GitLab generated manifest (currently Rust 1.98.1).
`--msrv` additionally checks all targets on installed Rust 1.88.0. The Rust gate runs
formatting, descriptor drift, offline builds/tests/Clippy, library dependency
boundaries, the [shared ESS provider boundary](adapters/README.md),
independent adapter-model compilation and AEP validation. It uses a task-owned temporary directory under
`.local/tmp`. `CARGO_TARGET_DIR` selects the build output base; the MSRV check uses
its `msrv/` subdirectory (default `target/msrv`). This local repository has no
configured CI or publication target.

```sh
mkdir -p .local/tmp
TMPDIR="$PWD/.local/tmp" CARGO_BUILD_JOBS=2 cargo run --locked -p connectors-build -- gate --msrv
```

Normal builds consume checked-in generated files and need neither ESS nor networked
vendor-source refresh. GitLab uses the full v2 generation pipeline:

```sh
cargo run --locked -p connectors-spec -- \
  --generate \
  --specification adapters/gitlab/spec/adapter.json \
  --output adapters/gitlab/generated
```

Repeat with `--check` to verify the complete bundle. Generation and its tests need
the pinned ESS release and the recorded rustfmt version. The shared resolver checks
`--ess`, then `CONNECTORS_ESS`, then `.local/toolchains/ess/<version>/bin/ess`, then
each `ess` on PATH for the exact pinned version. Explicit paths must match; default
lookup skips other versions. It never installs a tool or changes PATH. See the
[toolchain setup](docs/gitlab-generation.md#generate-and-check).
Kubernetes and SQL retain v1 descriptor
generation: omit `--generate` and select their `generated/descriptor.json` output.

Each adapter's default `service` feature adds its standalone executable and host
wiring. To embed only its contract implementation, disable default features:

```sh
cargo build --locked -p connectors-kubernetes --lib --no-default-features
```

The same command works for `connectors-gitlab` and `connectors-sql`. These libraries
depend on shared contracts/SDK and their protocol dependencies, with no host,
client, or sibling adapter dependency. The generic client and federation host have
no adapter dependencies. A new provider supplies an `Adapter` implementation and
configuration; it requires no provider switch in the shared client or host.

## Run services

Copy the matching file from `examples/`, choose a unique instance/listener, configure
the permitted resources, and bind private credential files or environment references.
Credential files must be regular files owned by the running user with mode `0600`.
Credentials never belong in operation input. Config changes require a restart.
Credential file replacement is resolved on subsequent requests, including gateway
downstream credentials. Environment references reflect the process environment;
changing a parent shell's environment cannot update an already running service.

```sh
target/debug/connectors-gitlab --config gitlab.yaml
target/debug/connectors-kubernetes --config kubernetes.yaml
target/debug/connectors-sql --config sql.yaml
target/debug/connectors serve --config federation.yaml
```

The shared CLI discovers the current contract before invoking it:

```sh
target/debug/connectors describe \
  --endpoint http://127.0.0.1:7101/ --allow-plaintext --token-file service.secret
target/debug/connectors invoke \
  --endpoint http://127.0.0.1:7101/ --allow-plaintext --token-file service.secret \
  --operation project.get --input examples/requests/gitlab-project.json
```

Through the example federation host, the same operation is `gitlab__project.get`.
Source prefixes distinguish instances; the gateway does not interpret provider data.
The first profile supports one federation hop. It never retries a provider request.

## Supported surfaces

| Adapter | Operations | External binding |
|---|---|---|
| GitLab | `project.get`, `issues.list`, `file.get` | GitLab API v4; configured project allowlist; token or public reads |
| Kubernetes | `resources.list`, `endpoints.discover`, optionally `hosts.discover` | Kubernetes API; namespace/kind restrictions; bearer token and CA |
| SQL | `schema.list`, `query.read` | PostgreSQL; explicit role/database; read-only transactions and deadlines |

SQL uses the `postgresql-native-text` profile: column names and native type names
accompany rows of strings or JSON null. Exact numeric, array, timestamp, and JSON
representations are preserved without numeric coercion. Parameters are text or null
and PostgreSQL resolves their types. Use explicit SQL casts where inference needs
help. The database role's grants govern accessible schemas and functions. Each request
opens a fresh connection and read-only transaction, initialized with `search_path = public, pg_catalog`;
qualify tables in other schemas (for example `reporting.sales`). There is no pool in
this bounded local profile. Fresh connections resolve the current credential each
time and discard session state; connection setup therefore contributes to latency.

Endpoint discovery reports observations with provenance, readiness and reachability.
It does not create a connection or dial the discovered address. Explicitly configure
an SQL binding and its credential after selecting an authorized reachable endpoint.

Service listeners speak HTTP; use loopback locally or a trusted TLS-terminating ingress
for remote deployments. The 32-operation limit and 20-second execution deadline
apply after request admission and reading; configure connection and request-read
limits at the ingress. Upstream TLS certificate validation is enabled by default.
For HTTP providers and SQL, `ca_file` replaces public roots with that PEM bundle.
Federation downstream clients currently use built-in public roots and have no
private-CA setting; local downstreams use explicitly permitted loopback HTTP.
`allow_plaintext` is an explicit local-test configuration, not a TLS verification bypass.

SQL keeps its execution deadline independent of caller-controlled database settings.
Timeouts and dropped invocations trigger a bounded cancellation/cleanup attempt;
the request budget includes cleanup. Remote termination cannot be guaranteed after
a network partition or process loss. See the [service contract](contracts/service/v1alpha1/semantics.md)
for the exact budgets and the [SQL regression harness](adapters/sql/examples/live_deadlines.rs)
for live timeout, caller-drop and TLS checks.

No adapter in this slice advertises OAuth acquisition, writes, durable events, process
execution, or media sessions. Those remain separate contracts in the full design.
