# Private prepare/commit transport checkpoint

The private mutation transport is implemented under the active
`story:guarded-gitlab-merge`. The [owning contract](../../../contracts/cli/v1alpha1/private-mutations.md)
defines explicit local configuration version two and its separately selected
private protocol. Production GitLab still exposes its read-only projection.
Approval issuance, owner mutation IPC, the complete host coordinator and a callable
approved GitLab merge remain required work.

`connectors-local/1` retains its implicit private version one and selection digest;
its decoder refuses the new field. Version two requires an explicit supported
private selection for every adapter, including when choosing private version one.
Loading never rewrites configuration. Readiness and cached-bootstrap admission
check the selected protocol's effect restrictions before credential entry.

A native preparation holds the immutable request and command-local credential.
The host's live handle exclusively borrows the exact child and consumes itself on
commit or cancellation. The original monotonic budget and absolute deadline span
both phases. Substitution, other requests, duplicate commits, EOF and expiry
cannot cause a new write. Cancellation acknowledges only after destruction;
dropping a host handle terminates and reaps its owned child. Post-commit failure
returns unknown effect without retry. Known applied evidence survives failure to
validate or disclose a safe result. Failure documents contain closed codes only.

This port does not itself verify approval proofs or persist mutation attempts.
Its trusted coordinator caller must hold the separately acknowledged policy,
approval, audit, attempt and connection-dispatch authority. Those integration
obligations remain open; these tests do not prove restart quarantine for a CLI
write or dedicated provider acceptance.

## Verification

Runs used Linux x86_64, task-owned
`TMPDIR=$PWD/.local/tmp/gitlab-runtime-20260910`,
`CARGO_TARGET_DIR=$TMPDIR/target` and `CARGO_BUILD_JOBS=2`. Cargo runs were serialized.
The successful full gate additionally used `RUST_TEST_THREADS=2`; tests retain
their own explicit concurrent contenders. The source baseline was
`fb4fbb3edbfddc0d369224de1d5bacd8ba18b8da` plus the source inputs recorded here.

| Command | Observed result |
|---|---|
| `cargo test --locked --offline -p connectors-host` | 113 host unit tests passed, 18 existing ignored cases; all 24 integration tests and the one-use compile-fail example passed. |
| `cargo clippy --locked --offline -p connectors-host -p connectors-gitlab --all-targets -- -D warnings` | Passed after the internal enum corrections described below. |
| `cargo run --locked --offline -p connectors-build -- gate --msrv` with two test threads | Passed: shared ESS (23 files, 429 declarations), six independent native models, CLI/adapter generation and drift, workspace tests, formatting, Clippy, boundaries, Rust 1.88 all-target checks, conformance and AEP. |
| `CONNECTORS_TEST_CLI=$CARGO_TARGET_DIR/debug/connectors cargo test --locked --offline -p connectors-gitlab --test local_runtime -- --ignored --test-threads=1 --nocapture` | All six disposable production CLI journeys passed in 222.99 seconds: CI/traces, real-expiry revalidation, failed repair/busy stop, MR collection, changed-head validation and CLI/owner/keyring restart reuse. |
| Website `npm run typecheck`, `npm run build`; `cargo run --locked --offline -p connectors-build -- docs --check` | Passed. Generated 46 contract pages and 99 total references, built Rust/WASM examples, indexed 120 pages, audited 487 public files and confirmed no reference drift. |

The private tests use the production fd-3 server and exact-child launcher with
fictional credentials. Independent peers write literal JSON to exercise wrong
readiness versions/challenges/projections, malformed preparation IDs and replies,
closed result documents and schema failures. Cases cover one send, no send before
commit, exact cancellation, substitution, duplicate commit, lost replies, expiry,
EOF, read-only legacy sessions and old-codec rejection. A compile-fail test
prevents consuming a prepared invocation twice. The CLI journeys use disposable
HTTPS, D-Bus and qualified GNOME Keyring processes, not a provider sandbox.

Conformance retains 315 scenarios (34 authored) and 22 existing synthesis
refusals. AEP retains 231 artifacts and its 139 existing review-format notices.
Neither replaces the pending production mutation acceptance. The full unedited
planning validation output is [aep-validation.log](aep-validation.log).

## Failures and source identity

The first Clippy check refused repetitive internal enum variant names and an
unnecessarily large reply union. Internal variants now have explicit unchanged
wire names, and the mutation reply decoder admits only its selected variants.
No lint was suppressed.

The first full gate failed in the unchanged audit-capacity concurrency test with
`MetadataUnavailable`; the standalone host run had passed it. An exact isolated
recheck passed in 0.80 seconds. The full gate subsequently passed with two test
threads. The test still launches eight concurrent contenders; no assertion,
storage timeout or production audit code changed. Both gate runs and the isolated
recheck are retained. This does not establish the precise cause of the first
failure or claim that unbounded parallel execution is reliable.

`source-inputs.sha256.gz` identifies 385 source and dependency inputs; the final
hash check passed after verification. `runtime-artifacts-after-cli.sha256.gz`
identifies the CLI and native adapter after Cargo built the tested target, plus
the qualified keyring and D-Bus executables. The earlier pre-test binary snapshot
is retained separately. These verification binaries do not establish reproducible
distribution acceptance. Dependency pins and generated adapter/CLI outputs are
unchanged. Rust 1.98.1/rustfmt 1.9.0 built the ordinary checks, Rust 1.88 checked
all targets, and Node 22.23.2/npm 12.0.2 built the website.

Atlas authority was verified from its clean primary checkout and remote main at
`1e9ea6546fcecbc87335d9f407f17790c296cc4e`; the bot-wrapper SHA-256 was
`8645c7100bda1e1f5a8285aa93f7feed045779ddf503d440dc2df15615c1e6ee`.
The installed Connectors 0.7.0 authority lookup returned `connector-unreachable`,
which was reported before using Git's read-only remote check. Root was the sole
implementation/planning writer on primary main. No linked tree was created.

The initiative and guarded-merge story remain active. Next integrate production
CLI policy/preparation/issuance and owner mutation IPC with the existing ledgers,
then one native SHA-guarded GitLab merge and its lost-response/restart acceptance.
Dedicated GitLab sandbox access and the create/update atomic-head decision remain
open. Full GitLab precedes Kubernetes with Helm, PostgreSQL, MCP and the remaining
providers. This checkpoint adds no release or publication claim.
