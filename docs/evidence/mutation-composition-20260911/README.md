# Mutation integration checkpoint

Work under `story:guarded-gitlab-merge`, based on
`af543efe3a75beb3bd624bb61639ef51ece24c08`. GitLab still advertises eleven reads.
This checkpoint does not supply owner mutation IPC, a complete dispatch
coordinator, native merge handling or a callable CLI write.

## Changes and scope

The mutation, audit and approval-spend stores previously accepted databases only
through version seven. Local approval policy publication installs version eight,
so the individually tested ports could not run together afterward. Their explicit
version checks now accept eight. Migration bytes, migration targets, WAL/FULL
settings and all separate acknowledgement boundaries are unchanged.

A combined test installs the actual policy migration and calls audit admission,
approval-aware preparation, proof spend, audit confirmation and the approved
dispatch gate on that authority. It reconstructs the stores after an unobserved
outcome and recovers the attempt as indeterminate; the same key still names that
attempt and its proof remains spent. Its signing key, policy and clock are test
fixtures. Store reconstruction is not evidence of a production owner restart or
native provider effect.

`Evidence::from_document` consumes a protected issuance document, with a 20 KiB
outer bound and the existing compact-proof limit. Literal string values borrow
from the zeroizing source during decoding; compact proof bytes are copied only
into their protected owner. Closed decoding refuses extra, duplicate, missing,
non-string, escaped and oversized inputs. A valid document round-trips and verifies
under the existing proof verifier. This API does not perform caller admission or
spend and is not yet called by operation invocation.

`ScopedHttp::into_write` gives trusted composition a separate, consuming PUT
capability over captured HTTP/TLS/credential configuration. It shares request
construction with GET, preserves individually encoded path segments and query
values, marks credential headers sensitive, bounds response headers/body, and
retains the existing disabled redirect/protocol-retry configuration. Native
adapters remain responsible for interpreting effect knowledge. A transport error
or HTTP status alone is no dispatch permission or proof of a safe retry.

## Verification

Linux x86_64; task-owned
`TMPDIR=$PWD/.local/tmp/gitlab-runtime-20260910`,
`CARGO_TARGET_DIR=$TMPDIR/target`, `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=2`.
Cargo commands run serially. The explicit CLI journeys select
`CONNECTORS_TEST_CLI=$CARGO_TARGET_DIR/debug/connectors`.

| Command | Observed result |
|---|---|
| `cargo test --locked --offline -p connectors-host --test http_write --test http` and corrected `--test http_write` | Three existing HTTP read tests and four write tests pass. The first write run had the fixture assertion correction below. |
| Exact host unit filters `local::approvals::tests::policy_schema_supports_audit_spend_dispatch_and_restart_observation` and `local::approvals::tests::protected_document_decoding_is_closed_bounded_and_verifiable` | Both pass; repeated as part of the full workspace gate. |
| `cargo run --locked --offline -p connectors-build -- gate --msrv` | Pass: shared and six independent native ESS roots; CLI/adapter generation and drift; workspace tests, formatting, Clippy and boundaries; Rust 1.88 all-target checks; conformance and AEP. Host unit suite: 124 passed, 19 explicitly ignored. New HTTP capability reuse compile-fail example also passes. |
| `cargo test --locked --offline -p connectors-gitlab --test local_runtime -- --ignored --test-threads=1 --nocapture` | All six production CLI journeys pass in 206.80 seconds: CI/traces, real-expiry revalidation, failed repair/busy stop, MR windows, pinned-head validation and CLI/owner/keyring restart reuse. |
| `cargo run --locked --offline -p connectors-build -- docs --check` | Pass; selected public documentation artifacts remain current. |
| `aep plan artifact validate` after the evidence update | Valid: 231 artifacts and 139 existing review-format notices. Full unedited output is retained in [aep-validation.log](aep-validation.log). |

The HTTP fixture receives a complete PUT and then closes without a reply. The
client returns its safe transport error; the server observes no second connection
and credentials are resolved once. Other cases cover exact JSON/path/query values,
a 307 pointing to another listener without following it, a 503 with `Retry-After`
without another send, oversized response refusal and invalid path segments before
credential resolution. These are disposable local HTTP fixtures, not GitLab
sandbox acceptance.

The initial lost-response fixture expected an empty query to omit `?`. The existing
URL builder preserves an explicit empty query; the actual request target is
`/api/v4/items/7?`. The fixture now asserts that complete target and unchanged PUT
method. No production retry/target behavior or effect-count assertion was weakened.
The initial failure is retained as a session-output excerpt.

Conformance retains 315 scenarios, 34 authored, and 22 existing synthesis refusals.
No model, generator, website presentation or selected public documentation input
changed. The two local implementation guides were checked against current source
and their local links; unrelated website build evidence is not rerun or extended.

The source manifest records 386 implementation/model/contract/dependency inputs.
Dependency pins are unchanged: Cargo lock SHA-256
`2e5536b428d4193128f6edd1ced2529a253ce5c3c870899e8f78b8ef91c04801`, ESS source
`6f7ef46163e758f3401945d1a946e0fc80ebc003`. Ordinary Rust/Cargo are 1.98.1;
minimum-version checks use 1.88.0. Runtime identities are verification artifacts,
not proof of reproducible distribution.

Atlas authority was verified from clean primary and remote main at
`1e9ea6546fcecbc87335d9f407f17790c296cc4e`. The installed Connectors integration
returned `connector-unreachable`; this was reported before Git's read-only remote
check. Root is the only implementation/planning writer, directly on primary main
under the repository-specific single-agent rule. No linked tree, release, external
publication, Atlas registration or paid governed run is part of this checkpoint.

The story and initiative remain active. Next connect protected proof consumption
and version-two owner IPC to current result-access admission, audit, attempt,
approval and connection dispatch; then join the generated native SHA-guarded merge.
Dedicated GitLab sandbox access, create/update atomic-head semantics, complete
GitLab acceptance and reproducible distribution remain open. The order stays
GitLab, Kubernetes including Helm, PostgreSQL, MCP and remaining providers.
