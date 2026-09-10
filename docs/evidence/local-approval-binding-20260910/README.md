# Local approval binding verification — 2026-09-10

Source parent: `631caabffa46702e5dddeb02a1a3aa7d0547e05c` on local main.
The final tree is identified by [source-inputs.sha256](source-inputs.sha256):
480 non-planning, non-receipt source files. The implementation is the
[private local approval binding](../../local-approval-binding.md), with no
advertised approval CLI, provider writes or delegated ingress.

## Implemented boundary

Strict canonical JWS verification binds all mandatory-null subject coordinates,
fixed Ed25519/type, configured issuer/audience/key, current policy and bounded
time. Signing uses protected in-memory key material and fresh random references;
it is not persistent issuer onboarding. The configured key value and optional
complete attempt subject were modeled and validated before decomposition.

SQLite migration six retains complete approval subjects for keyed and unkeyed
preparations, exact issuer/reference uniqueness, one redemption per attempt and
immutable tombstones. Migrations one through five retain their original bytes.
Only approval-aware preparation upgrades the port; normal setup stays at three.
The consumer policy guard stays held through spend commit/acknowledgement, with
clock checks before and after commit. Late expiry retains the spend but yields
no receipt. An original live preparation allows one spend call, so failed or
ambiguous acknowledgement cannot be retried into a newly reconstructed grant.

The separate approved dispatch gate consumes both original live handles and
rechecks durable association and current connection fences. Ordinary dispatch
refuses required and event-claim modes. Recovery and abort do not refund a spend
or resend. Audit, prepare, spend, gate and settlement remain separate decisions.

## Commands and results

Cargo uses two jobs and task-owned storage:

```sh
export TMPDIR="$PWD/.local/tmp/gitlab-runtime-20260910"
export CARGO_TARGET_DIR="$TMPDIR/target"
export CARGO_BUILD_JOBS=2
cargo run --locked --offline -p connectors-build -- gate --msrv
```

[gate-final.log](gate-final.log) passes shared/native ESS validation and compile,
parser/descriptor drift, reproducible generation, workspace tests, Clippy,
dependency boundaries, conformance and all-target Rust 1.88 checks. The host
suite reports 75 passes and six ignored auxiliary/explicit integration entries.
[approval-tests.log](approval-tests.log) records 17 focused passes, plus one
ignored subprocess entry that the parent invokes four times.

Coverage includes the RFC 8032 vector; exact fixed-algorithm proof framing;
nullable/Unicode identity and every subject coordinate; malformed JSON/base64
and integer bounds; issuer/audience/key substitution; current revocation and
authority changes; unavailable/expired time; simultaneous reference/attempt
spends and substituted proofs; full unkeyed capture and old-record refusal;
capacity, immutable tombstones, rollback and lost acknowledgement; guard lifetime
through committed visibility; corrupt storage classification; original receipt
association, existing-key observation and separately fenced dispatch.

Four child processes exit abruptly before spend commit, after spend commit,
after the dispatch gate and after a simulated effect. Recovery observes zero or
one retained spend and fences Prepared to Aborted or Dispatching to Indeterminate.
Repeated recovery leaves the simulated effect unchanged. These are real process
and SQLite tests, not provider writes or production clock/policy qualification.

Website type checking, production build and reference drift checks pass:
[typecheck](website-typecheck.log), [build](website-build.log),
[reference](docs-check.log). The build indexes 114 pages and audits 468 public
files; reference projection has 43 contract and 93 total reference pages.
All 15 authored example tests also pass in [website-example-tests.log](website-example-tests.log).
The existing CSS minimizer font-size warnings remain non-fatal.

The website's separate example workspace needed two constructors updated for
the new optional field. The store forwards that field; its fictional approval
counter supplies None, with no invented verified subject. This three-line change
followed the full repository gate and was checked by the example tests, Rust/WASM
regeneration and website build. [gate-source-inputs.sha256](gate-source-inputs.sha256)
differs from the final manifest only for that excluded example file; the full
gate's workspace implementation/model inputs are unchanged.

The optimized guide build and all five production CLI journeys pass:

```sh
cargo build --release --locked --offline -p connectors -p connectors-gitlab
CONNECTORS_TEST_CLI="$CARGO_TARGET_DIR/release/connectors" \
  cargo test --locked --offline -p connectors-gitlab --test local_runtime -- \
  --ignored --test-threads=1 --nocapture
```

[cli-release-journeys.log](cli-release-journeys.log) records five passes in 85.17
seconds: exact-commit CI, real-expiry credential revalidation, failed repair/busy
stop, MR update-window traversal, and CLI/owner/keyring restart reuse. They use the
optimized generic CLI and harness-built debug GitLab executable against disposable
private HTTPS and qualified Secret Service fixtures. Dedicated GitLab sandbox
acceptance remains missing. The tested executables are retained read-only in the
task's `approval-binding/verified-bin/`; their exact identities are recorded in
[verified-executables.sha256](verified-executables.sha256).

## Corrections and planning review

[focused-corrections.txt](focused-corrections.txt) records the initial fixture
syntax/removal mistakes. The first full gate passed tests then refused an enlarged
preparation enum; boxing the private capture fixed Clippy without changing receipt
ownership. [gate-first.log](gate-first.log) retains that refusal. A second gate
passed; final review then corrected storage errors being reported as approval
refusals and added two corrupt-schema tests. The final full gate passed again.
The first website attempt exposed the constructor omissions; its failure is
retained in [website-build-first.log](website-build-first.log).

Four independent planning critics reviewed the six-child partial decomposition:
`aep-plan:plan-critic-acceptance`, `aep-plan:plan-critic-design`,
`aep-plan:plan-critic-scope` and `aep-plan:plan-critic-parallel-safety`.
The design critic's sole first-round finding required an explicit dependency on
the audit schema. It was fixed through AEP and recorded as fixed. All four second
rounds approved. All eight exact verdicts are immutable review-result records.
Sonnet was unavailable, so inherited models were used; the fourth critic followed
the first three because of the concurrency limit, without shared findings.

This was interactive implementation with one planning writer and no approval
bypasses. AEP validates 196 artifacts. Its 114 review-format notices include
explicit empty findings blocks; those historical records were preserved.
[planning-validation.log](planning-validation.log) retains the verbatim output.

Rust/Cargo 1.98.1 built the source; the gate checks Rust 1.88. ESS 0.20.0 remains
pinned at `6f7ef46163e758f3401945d1a946e0fc80ebc003`. Tool inputs are recorded in
[tool-inputs.sha256](tool-inputs.sha256). Cargo.lock adds only a direct host
reference to the already-pinned ring 0.17.14. No provider vendor refresh, source
publication, image rebuild, deployment or full distribution reproducibility is
claimed by this checkpoint.

## Remaining work

Persistent protected issuer-key custody/publication/rotation, actual local
approval preparation/issuance CLI, authenticated caller policy, production clock
qualification, generated mutation ingress and the complete connection-bound
provider coordinator remain required. Native GitLab validation/writes follow.
C14 create/update head-guard semantics still await an answer; no weaker race
guarantee has been selected. Dedicated GitLab sandbox acceptance remains open.
Full GitLab precedes Kubernetes, PostgreSQL, MCP and the remaining providers.
