# Local approval-policy implementation checkpoint

The policy metadata and lease port is implemented under the active
`story:guarded-gitlab-merge`. It retains policy identity, successor revisions,
issuer/instance binding, exact selection snapshots and a bounded operation set.
Shared process-bound leases exclude publication until current uses finish.
Migration eight leaves migrations one through seven unchanged; setup remains at
version three. Passive status creates no policy schema, lock or service.

This port requires its trusted host caller to supply already admitted write
operations. It does not authenticate the caller, classify provider operations,
issue approval evidence or dispatch a business effect. The proposed
[coordinator](../../../contracts/service/local-mutations.md),
[private protocol](../../../contracts/cli/v1alpha1/private-mutations.md),
[write generation](../../../spec-kinds/adapter/v3/semantics.md) and
[GitLab merge](../../../adapters/gitlab/contracts/guarded-merge.md) still need
their production implementation. No GitLab write or full provider completion is
advertised by this checkpoint.

## Verification

Commands ran from the repository root unless marked website. Bounded runs used
`TMPDIR=$PWD/.local/tmp/gitlab-runtime-20260910`,
`CARGO_TARGET_DIR=$TMPDIR/target`, `CARGO_BUILD_JOBS=2`, and the unchanged ESS pin
in `crates/connectors-spec/toolchain.json`. Logs are retained alongside this file.

| Command | Observed result |
|---|---|
| Pinned `ess specify validate --path ess` | 23 files valid; full gate compiles 429 declarations. |
| `cargo test --locked --offline -p connectors-host --lib local::approval_policy -- --nocapture` | 12 tests pass in 2.82 seconds; the ignored subprocess helper is explicitly invoked by the process tests. |
| `cargo run --locked --offline -p connectors-build -- gate --msrv` | Pass: 103 host tests, native/shared ESS, CLI and adapter generation, workspace tests/Clippy, dependency boundaries, Rust 1.88, conformance and AEP. |
| `cargo run --locked --offline -p connectors-build -- docs` and `docs --check` | 46 selected contract pages, 99 total reference pages; no drift. |
| Website `npm run typecheck`, bounded `npm run build`, `npm run test:examples` | Pass; 120 indexed pages, 487 public files audited; 15 example tests pass. |
| `cargo build --locked --offline --release -p connectors -p connectors-gitlab` | Pass; tested executable digests are retained in `optimized-binaries.sha256.gz`. |
| `CONNECTORS_TEST_CLI=$CARGO_TARGET_DIR/release/connectors cargo test --locked --offline -p connectors-host --lib local::approval_keys -- --include-ignored --skip key_crash_child --test-threads=1` | All 11 key-management tests pass in 19.49 seconds. |
| `CONNECTORS_TEST_CLI=$CARGO_TARGET_DIR/release/connectors cargo test --locked --offline -p connectors-gitlab --test local_runtime -- --ignored --test-threads=1 --nocapture` | All six production CLI journeys pass in 104.57 seconds against disposable GitLab HTTPS and qualified Secret Service fixtures. |

The policy suite uses real SQLite and exact child processes. It covers passive
inspection, missing issuer, restart identity/snapshot continuity, stale CAS,
operation and selection mismatch, malformed/oversized publication, failed SQL
writes, unknown publication acknowledgement, immutable retention triggers,
concurrent first publication, shared-use exclusion across processes, killed-use
cleanup, exit after commit before acknowledgement, forked-process refusal,
revision exhaustion and exclusion of private authority/custody coordinates from
public views. A lost acknowledgement retains the committed successor revision;
repeating the previous CAS refuses instead of creating another publication.

Conformance synthesis retains the existing 22 explicit refusals for states that
have no scenario constructor; its 315 scenarios (34 authored) are not a runtime
proof of the new coordinator. The required gate accepts and reports that bounded
result. Dedicated sandbox acceptance, full CLI write recovery, and distribution
reproducibility remain unsatisfied under the active story.

## Corrections and review

The first policy test run refused all ten initial fixtures because their fresh
temporary directories were not mode 0700. The fixtures now explicitly establish
that prerequisite; the production ownership check was not relaxed. The final
suite adds fork and exhausted-revision cases. The initial website command omitted
the task-specific Cargo/TMP environment; it was rerun with the required settings.
Both outputs are retained, without deleting unrelated pre-existing build caches.

The four planning perspectives approve the decomposition. Sonnet was unavailable;
reviewers used the inherited model and the fourth lane was delayed by slots.
An initial acceptance reading reused the scope reviewer's context. That record
is archived, preserved, and replaced by a fresh-context acceptance review that
read no other findings. AEP first refused promotion for a missing existing-vision
edge, then a direct draft-to-active move. Adding that edge allowed the legal
draft-to-proposed-to-active transitions. These observations do not close runtime
acceptance or supply provider access.

## Source and ownership

The source base is `7e95b9c20e04e5d30f1056ffe326d02b3a6670a4`.
`source-inputs.sha256.gz` records the exact runtime, contract, native-input,
generator, model and Cargo bytes checked. No dependency pin changed. The original
six SQL migration files retain their recorded bytes; migration one remains the
unchanged embedded SQL in metadata.rs. Timestamped receipts are separate from
the deterministic source/generation inputs.

The clean Atlas authority checkout matches remote main
`1e9ea6546fcecbc87335d9f407f17790c296cc4e`; its bot-wrapper SHA-256 is
`8645c7100bda1e1f5a8285aa93f7feed045779ddf503d440dc2df15615c1e6ee`.
Root is the only implementation/planning writer. Its temporary authority tree is
managed through `worktree` and must be retired with exact-ID recovery proof.
The unrelated AGENTS.md release edit is preserved separately. No Connectors
publication, provider sandbox access, cloud deployment or Atlas registration is
part of this checkpoint.

The next runtime work is the explicit private version-two and v3 generation
bindings, followed by their production policy/issuance and one-shot merge
coordinator. Full GitLab remains before Kubernetes, PostgreSQL, MCP and the
remaining providers. The existing sandbox and create/update atomic-head blockers
remain open.
