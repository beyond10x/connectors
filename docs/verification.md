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
descriptions were captured only at gateway startup in this original evidence.
The subsequent [review remediation](review-response-2026-09-08.md) adds atomic
refresh on a leaf stale-description refusal and deliberate caller resubmission. Credential-file
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

## GitLab specification-to-service completion, 2026-09-08

The follow-up story is `story:gitlab-spec-service`. Local commit
`75f1c7275d7b227d5a2e4a3e95bfc5c9b2de262a` preserves the previous workspace before
this implementation. [Generation and packaging](gitlab-generation.md) explains
reproduction; [the evidence directory](evidence/gitlab-spec-service-2026-09-08/)
contains the exact build, source, test, runtime and shutdown observations.

Observed verification:

- `cargo fmt --all -- --check` and
  `cargo clippy --workspace --all-targets --locked -- -D warnings`: exit 0.
- `cargo test --workspace --locked`: **24 passed, 0 failed, 0 ignored**, including
  generation reproducibility/drift/ownership/refusal cases, GitLab request/policy/
  pagination/response fixtures, and a missing-binding compile-fail doctest.
- `cargo build --workspace --locked --offline`: exit 0. Ordinary source builds
  consume checked-in generated Rust; no ESS invocation or source refresh occurs.
- Each of GitLab, Kubernetes and SQL built with `--lib --no-default-features`.
  Normal dependency trees exclude host/client/sibling adapters from those
  libraries and exclude all adapter implementations from the generic CLI.
- `connectors-build check`, repository ESS validation, generated ESS validation/
  compilation/synthesis, ESS build compilation, BuildKit projection, Docker image
  build and exact-image realization validation/compilation passed.
- The final Rust live acceptance runner passed **nine scenario groups** against
  the packaged GitLab service, a new k3s fixture, and PostgreSQL 17.11. All eight
  operations and selected refusal cases worked directly and through federation.
  The actual Kubernetes EndpointSlice observation was selected to configure SQL;
  the observation itself did not dial or supply a credential.
- The running GitLab container used the recorded image and executable digest,
  a read-only root filesystem, and separate read-only `/config` and `/secrets`
  mounts. The configured upstream was public GitLab over verified HTTPS.
- SIGTERM produced exit 0 for the GitLab container and all three host processes
  (gateway, Kubernetes adapter, SQL adapter). All three owned containers were
  removed. The PostgreSQL fixture exited 0; the k3s fixture exited 2 on Docker stop
  after acceptance had passed. `shutdown.json` preserves this distinction.

The retained local image is `connectors-v2-gitlab:spec-local`, identity
`sha256:220b27a8a4f91cd69618c0324dc7243cd446cd7f97454f8cd35bb4cfe8b9b247`.
Its executable SHA-256 is
`382a3e1401941248d282fdff10a97b4ede957960c37c3b4ab90a3dd3b56032c6`.
`build-evidence.json` binds the binary, source snapshot, rootfs manifest, toolchain,
and generation manifest. Code/Cargo inputs were compared back to that source snapshot after acceptance.
A final generator change normalizes ESS Rust through rustfmt. All 13 affected
generation/GitLab tests and Clippy were rerun, the image was rebuilt, and the three
GitLab acceptance groups were rerun against that exact final image. The original
nine-group run and its earlier image checks remain recorded under `prior-build/`. Subsequent evidence and planning updates describe the
run; they are not claimed to have existed inside the earlier build snapshot.

Limits and encountered refusals:

- ESS **0.9.2** cannot directly import the vendor's OpenAPI 3.0 document. The complete
  refusal is retained in `adapters/gitlab/generated/ess-import.json`. The Connectors
  frontend imports the supported selected request mappings and ESS synthesizes
  the separately declared local request types. Vendor response schemas remain
  uncorrected source evidence; handwritten response obligations are tested.
- ESS's image graph requires a repository label even for local execution. The
  recorded label names the local image; no registry or publication was configured.
  Its omitted-empty-`secrets` reader mismatch is handled by the documented separate
  projection-input adaptation, retaining the original canonical build IR.
- The system temporary filesystem exhausted its quota during an early generator
  run and a later independent SQL build. Both passed using the task-owned `TMPDIR`.
- GitLab private credentials are covered by fixtures, not a private live account.
  The PostgreSQL live fixture uses explicit plaintext; its TLS path remains covered
  by the existing implementation and prior stated limits. The image is a native
  Linux x86_64 development build with host runtime library hashes, not a production
  release or a claim of reproducible images across build hosts.

No Atlas, original Connectors checkout, remote repository, registry or consumer
state was changed. The final image is retained locally. Owned probe images and
reproducible rootfs copies were removed; reports and the source workspace remain.

## Full review remediation — 2026-09-08

The subsequent [review response](review-response-2026-09-08.md) records all 17
findings. Its [gate log](evidence/review-2026-09-08/gate.log) shows 35 passing tests,
zero failures/ignored tests, formatting, warning-denying Clippy, regenerated
schema checks, offline builds, dependency boundaries and Rust 1.88 checks for all
targets. The [audit](evidence/review-2026-09-08/audit.json) has no vulnerabilities
or warnings. It adds local PostgreSQL protocol and TLS fixtures without claiming
a repeat of the earlier live services or image build.

## ESS executable resolution — 2026-09-08

The repository now owns its pin in `crates/connectors-spec/toolchain.json`.
The resolver was first verified with the unchanged 0.9.2 pin. Both commands below
exited 0 and printed `gate: all checks passed`, including the Rust 1.88 check:

```sh
env -u CONNECTORS_ESS PATH="/home/timo/.cargo/bin:/home/timo/.local/bin:$PATH" TMPDIR="$PWD/.local/tmp" CARGO_BUILD_JOBS=2 cargo run --locked -p connectors-build -- gate --msrv
env -u CONNECTORS_ESS PATH="/home/timo/.local/bin:/home/timo/.cargo/bin:$PATH" TMPDIR="$PWD/.local/tmp" CARGO_BUILD_JOBS=2 cargo run --locked -p connectors-build -- gate --msrv
```

The corresponding [cargo-first](evidence/ess-toolchain-2026-09-08/gate-pin-cargo-first.log)
and [local-first](evidence/ess-toolchain-2026-09-08/gate-pin-local-first.log) logs
contain 39 passing tests each. With the same cargo-first environment,
`cargo test --locked -p connectors-spec --test generation` passed
[5/5](evidence/ess-toolchain-2026-09-08/generation-pin-cargo-first.log).

With `CONNECTORS_ESS` unset and `PATH=/home/timo/.cargo/bin:/usr/bin:/bin`,
`target/debug/connectors-build gate --msrv` and the standalone generation tests
both refused the absent 0.9.2 binary, naming the pin and searched locations:
[gate refusal](evidence/ess-toolchain-2026-09-08/missing-pin.log),
[generation refusal](evidence/ess-toolchain-2026-09-08/generation-missing-pin.log).
No installed executable was renamed. Isolated resolver tests also cover matching
and mismatching explicit flags/environment paths, cache selection, both PATH
orders, and agreement between the committed manifest and the repository pin.
