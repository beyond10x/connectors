# Local mutation ledger verification — 2026-09-10

Source parent: `dd3df88c1ed62e7c4d8ee740426ebd80674754fb` on local Connectors main.
Verification covers the working-tree increment identified by
[`source-inputs.sha256`](source-inputs.sha256), excluding planning records and
timestamped receipts. The implementation is the private host SQLite port described
in [the binding](../../local-mutation-ledger.md); it advertises no CLI write.

## Implementation and scope

Migration 4 atomically retains attempts and exact keyed reservations, references
the existing connection authority, and preserves immutable terminal results.
Only admitted mutation preparation installs it. Ordinary setup and read management
retain schema 3 and can inspect recognized schema 4. The first three migration
bytes and authority identities are preserved.

Preparation and gate acknowledgements return separate non-Clone handles only on
definite success. The gate checks the original process, authority and current
published connection/configuration/revision/fence. Lookup and recovery cannot
reconstruct a handle. Pending and unknown reservations do not expire; known
retirement compares the exact key and reservation generation under trusted clock
bounds. Unavailable or regressed clock evidence cannot shorten retention. The
clock port deliberately has no unqualified SystemTime default.

The tests use actual private SQLite files, synthetic registry coordinates and a
deterministic clock. They do not claim caller policy, approval spending, audit,
provider-bound dispatch or production clock qualification. Those remain the next
shared GitLab controls. The C14 create/update SHA race is recorded separately in
`decision-blocker:gitlab-mr-create-update-head-guard`; dedicated GitLab sandbox
acceptance remains open.

## Commands and results

All Cargo commands use:

```sh
export TMPDIR="$PWD/.local/tmp/gitlab-runtime-20260910"
export CARGO_TARGET_DIR="$TMPDIR/target"
export CARGO_BUILD_JOBS=2
```

The final required command is:

```sh
cargo run --locked --offline -p connectors-build -- gate --msrv
```

[`gate-qualified-migration.log`](gate-qualified-migration.log) passes: shared ESS
393 declarations; independently compiled adapter models; parser, generation/drift,
conformance, workspace tests, Clippy, dependency boundaries and Rust 1.88. The
ledger contributes 14 executed tests plus one subprocess entry point exercised by
the parent test. Four real child processes exit abruptly after preparation,
gate, simulated effect and terminal settlement. Recovery preserves Aborted,
Indeterminate, Indeterminate and Completed respectively without a second effect.
Other cases include eight concurrent duplicate callers, 24 abort/gate races,
lost acknowledgements and atomic rollback, all fingerprint coordinates, namespace
null/origin distinctions, safe payload and record capacity, immutable first facts,
clock bounds/regression/outage, stale retirement, unkeyed attempts and schema-3
upgrade continuity. These are storage-port and fixture facts, not native writes.

`npm run build`, `npm run typecheck` and
`cargo run --locked --offline -p connectors-build -- docs --check` pass. Their
logs retain 114 indexed website pages, 468 audited public files and 43 contract /
93 reference pages without drift. The mutation status now distinguishes SQLite
fixture evidence from pending CLI writes.

The optimized CLI build and its five production CLI journeys pass:

```sh
cargo build --release --locked --offline -p connectors --bin connectors
CONNECTORS_TEST_CLI="$CARGO_TARGET_DIR/release/connectors" \
  cargo test --locked --offline -p connectors-gitlab --test local_runtime -- \
  --ignored --test-threads=1 --nocapture
```

[`cli-release-journeys.log`](cli-release-journeys.log) records five passes in
86.42 seconds. This run uses the optimized generic CLI and the test harness's
debug GitLab executable. The exact binaries are retained read-only under the
task's `mutation-ledger/verified-bin/` directory, with
[`verified-executables.sha256`](verified-executables.sha256). The CLI is
21,875,808 bytes with SHA-256
`9f7556b7509f0b69ac401bc215d8fd23dc03a7f3c785ff95ad29c2900e4dc33b`;
the GitLab executable digest is
`76a1abebe6382e0fe6946bd669ba1df974ce64fd8674188b89ad1f0da967a689`.
These are disposable local HTTPS/Secret Service journeys, not dedicated GitLab
sandbox evidence.

The guide's combined optimized build also passes:
`cargo build --release --locked --offline -p connectors -p connectors-gitlab`;
[`guide-build.log`](guide-build.log) records it. Its generic CLI bytes match the
retained tested binary exactly. The guide wording is the only runtime-input
manifest entry changed after the final gate; it does not affect that Rust/ESS
gate or the selected website projection. The final manifest records all 470
non-planning, non-receipt source files.

## Failures and follow-up

The initial ESS clock value used undeclared `Int64`; it was corrected to the
existing `Integer` vocabulary before planning decomposition, and validation passed.
The initial Rust test helper attempted SQLite conversion directly into `usize`;
it now reads `u32`. The next fixture run was refused because its temporary state
directory lacked explicit 0700 permissions; the fixture was corrected without
weakening filesystem admission. The first Clippy run requested a simpler type for
the fingerprint variation table; fixed-size function pointers replace boxed
closures. The first full gate found a previous migration test expecting version 3
after unconditional migration to 4. An intermediate corrected gate passed, then
review of that upgrade boundary led to the final explicit mutation-only migration;
the original ordinary-upgrade expectation remains 3. All logs are retained.

The initial unoptimized CLI rerun passed two journeys and timed out during owner
startup in three. An isolated comparison using the unchanged previously verified
CLI (`24133fc3a301d3856f79c560d83fae8c404fd5c4b6c67e3e4a7ac110661f5f69`)
reproduced the same first-connect timeout in 11.83 seconds. It is therefore not
specific to the new ledger code. Both debug binaries are approximately 95 MB;
the existing owner spawn path hashes and captures its executable within a
ten-second budget. Executable-capture cost is a supported explanation, not an
instrumented measurement of the original failing step. No runtime deadline was
increased. The [local CLI guide](../../local-gitlab-cli.md) now selects an optimized
build. The original debug failures and the baseline comparison remain in
[`cli-journeys.log`](cli-journeys.log) and
[`cli-baseline-comparison.log`](cli-baseline-comparison.log).

## Inputs, planning and limits

Rust/Cargo 1.98.1 build the current source; the gate checks Rust 1.88.0. ESS 0.20.0
is pinned at `6f7ef46163e758f3401945d1a946e0fc80ebc003` with executable and input
digests in [`tool-inputs.sha256`](tool-inputs.sha256). Cargo.lock and provider vendor
inputs are unchanged. AEP reports protocol 0.55.0 and worktree CLI 0.4.0.

The four `aep-plan:plan-critic-*` roles reviewed the partial decomposition in two
independent rounds. The unavailable Sonnet pin was replaced by the inherited
model; the fourth review was scheduled after a slot became free while preserving
independence. One pre-existing acceptance-wording finding was fixed; all four
second-round verdicts approve. Exact immutable records and the one fixed outcome
are in AEP. Its existing no-findings-block notices also count explicit empty
approval blocks; no historical review was rewritten to suppress them.

The full initiative remains open in the required order: GitLab, Kubernetes,
PostgreSQL, MCP, remaining providers. No Connectors source/image publication or
cloud deployment occurred. No image is rebuilt for this private-port increment;
the earlier MR image remains evidence for its own recorded inputs. Complete
distribution reproducibility and dedicated-provider runtime acceptance remain
required at their owning milestones.
