# Local execution audit verification — 2026-09-10

Source parent: `04a7e22110d3e5b36d3f39b178d32fd7ee283c0e` on local main.
The verification tree is identified by [source-inputs.sha256](source-inputs.sha256):
475 non-planning, non-receipt source files. It implements the private host
[execution audit binding](../../local-execution-audit.md), without enabling
public audited dispatch or provider writes.

## Implemented boundary

Migration five adds retained audit aggregates to the existing private WAL/FULL
authority. Only admitted audit anchoring installs it; ordinary setup retains
version three. The mutation port recognizes versions four and five. Previous
migration bytes, authority UUID, connections and attempts remain intact.

The owner allocates public refs, derives the contract's exact private identity,
checks bounded safe fields and same-instance/connection references, and returns a
non-Clone live admission receipt only after definite commit acknowledgement.
Early refusal and internal observation cannot produce that receipt. Final append
has its own transaction and acknowledgement; exact observation retries recover
the original fact, while any changed field or UUID conflicts. Capacity never
evicts records, and no delete or automatic expiry API exists.

The port accepts trusted host facts. It does not verify caller authority,
approval proofs, credentials, current provider permission or disclosure policy.
Audit timestamps describe observations and are not a qualified authorization or
expiry clock. The complete dispatch coordinator must supply those controls.

## Commands and results

Cargo commands use two jobs and task-owned storage:

```sh
export TMPDIR="$PWD/.local/tmp/gitlab-runtime-20260910"
export CARGO_TARGET_DIR="$TMPDIR/target"
export CARGO_BUILD_JOBS=2
cargo run --locked --offline -p connectors-build -- gate --msrv
```

[gate-final.log](gate-final.log) passes the required full gate: shared and native
ESS validation/compilation, parser and descriptor drift, reproducible generation,
workspace tests, Clippy, conformance, dependency boundaries and all-target Rust
1.88 checks. The host suite reports 58 passes and five ignored entry points;
the new audit port has 12 executed tests and one auxiliary subprocess entry point
that the parent invokes four times.

Audit fixtures cover exact identity and instance isolation, rollback and lost
acknowledgements, eight competing final appends, eight concurrent anchors against
capacity four, immutable outcomes and exact retry, safe field and aggregate byte
bounds, malformed/unavailable metadata, stale or mismatched receipt refusal,
retained connection/attempt references and migration continuity. The retention
fixture directly simulates completed mutation-index retirement; the mutation
owner's existing expiry tests also pass. It does not claim public cleanup or
provider execution.

Four child processes exit abruptly immediately before and after anchor/final
commit. Recovery observes absent, anchored, anchored and finalized state,
respectively. A simulated effect file is created only after receipt confirmation;
final append recovery leaves it unchanged. These are actual process exits and
SQLite recovery, with simulated business work.

`npm run typecheck` and `npm run build` pass. The build indexes 114 pages and
audits 468 public files. The reference check passes with 43 contract / 93 total
reference pages and no drift. See [website-build.log](website-build.log),
[website-typecheck.log](website-typecheck.log) and [docs-check.log](docs-check.log).

The guide's optimized build and five production CLI journeys pass:

```sh
cargo build --release --locked --offline -p connectors -p connectors-gitlab
CONNECTORS_TEST_CLI="$CARGO_TARGET_DIR/release/connectors" \
  cargo test --locked --offline -p connectors-gitlab --test local_runtime -- \
  --ignored --test-threads=1 --nocapture
```

[cli-release-journeys.log](cli-release-journeys.log) records all five passes in
85.32 seconds: exact-commit CI, real-expiry credential revalidation, failed repair
and busy stop, MR update-window traversal, and CLI/owner/keyring restart reuse.
The fixture uses the optimized generic CLI and the harness's debug GitLab
executable, against disposable private HTTPS and qualified Secret Service
services. Dedicated GitLab sandbox acceptance is still missing.

The tested binaries are retained read-only under the task's
`execution-audit/verified-bin/`, with [verified-executables.sha256](verified-executables.sha256).
Generic CLI SHA-256 is
`690b70bc7ac10969c513fddf6ac058c131d645f9aa4cadc2689eb3a6e66d3cda`;
native fixture executable SHA-256 is
`bf70c37e22cc3121d5f7e6490953fdd9032a13304108a83ef64901868afdec6b`.

## Corrections, review and remaining work

The first focused audit tests passed. The first gate refused infrastructure
vocabulary in the shared ESS status summary; the summary now names the logical
host port, with SQLite details retained in binding documentation. The next gate
passed workspace tests but Clippy refused a large acknowledgement enum. Boxing
the retained record preserves the non-Clone receipt and passed targeted Clippy
and the final full gate. All failed-run logs remain alongside the final results.

Four independent planning critics approved the five-child partial decomposition
in one round, with zero findings and no second round needed. Exact immutable
records are in AEP. Sonnet was unavailable, so the inherited model was used;
three slots required the fourth review to follow without shared findings.
The validator's missing machine-readable scope notice was corrected through AEP.
Its existing review-format notices also count explicit empty findings blocks;
the recorded approvals were not rewritten to suppress them. This was an
interactive implementation with one planning writer and no approval bypasses.

Rust/Cargo 1.98.1 built the source, with Rust 1.88 checked by the gate. ESS 0.20.0
remains pinned at `6f7ef46163e758f3401945d1a946e0fc80ebc003`; input/executable
digests are in [tool-inputs.sha256](tool-inputs.sha256). Cargo.lock adds only the
host's direct reference to the already-pinned base64 0.22.1 dependency. Provider
vendor inputs and generated adapter bundles are unchanged. No image rebuild,
publication, deployment or complete distribution-reproducibility claim is made.

Next are local approval issuance/verification/spending, qualified clock and
connection-bound dispatch composition, then native MR validation and governed
writes. C14 create/update head-guard semantics and dedicated GitLab access remain
explicit blockers for their own acceptance. Full GitLab still precedes Kubernetes,
PostgreSQL, MCP and remaining providers; the overall initiative remains open.
