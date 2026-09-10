# Local approval-key management verification — 2026-09-10

This checkpoint adds `approvals key-init`, `key-status`, `key-rotate`,
`key-recover`, `key-revoke` and `key-retire` to the generated production CLI.
The [issuer contract](../../../contracts/service/approval-issuers.md) and
[CLI guide](../../local-approval-keys.md) describe the selected behavior.
Signing seeds stay in purpose-separated qualified Secret Service custody;
SQLite migration seven retains public identity/history and current fences.
Ordinary setup remains schema three, and previous migration bytes are unchanged.

## Runtime and repository checks

The following commands ran from the repository root with:

```sh
export TMPDIR="$PWD/.local/tmp/gitlab-runtime-20260910"
export CARGO_TARGET_DIR="$TMPDIR/target"
export CARGO_BUILD_JOBS=2
```

```sh
cargo run --locked --offline -p connectors-build -- gate --msrv
cargo build --release --locked --offline -p connectors -p connectors-gitlab
CONNECTORS_TEST_CLI="$CARGO_TARGET_DIR/release/connectors" \
  cargo test --locked --offline -p connectors-host --lib local::approval_keys -- \
  --include-ignored --skip key_crash_child --test-threads=1
CONNECTORS_TEST_CLI="$CARGO_TARGET_DIR/release/connectors" \
  cargo test --locked --offline -p connectors-gitlab --test local_runtime -- \
  --ignored --test-threads=1 --nocapture
```

[The complete gate](gate-final.log) passes shared/native ESS validation and
compilation, 69 CLI structural cases, generation/drift, all workspace tests,
Clippy, adapter boundaries, Rust 1.88 all-target checking, conformance and AEP.
The ordinary host suite has 77 passes and 16 ignored auxiliary/integration entries.

[Eleven explicit key tests](test-keys-verified.log) pass in 10.15 seconds against
real private SQLite and qualified disposable Secret Service processes. Three
actual child exits distinguish staging, acknowledged secret storage and committed
publication. Tests cover CLI/keyring restart, all six production commands, missing
and locked custody, failed rotation preserving the active key, wrong recovery
material, lost synchronization acknowledgement, purpose isolation for identical
UUID tuples, competing initialization, held-use exclusion, stale revocation and
recovery, uncertain deletion and exact retirement retry. Public views omit private
locators. Ordinary production-parser tests also reject malformed, nil and
noncanonical UUIDs without creating key-state infrastructure.

[All five optimized GitLab CLI journeys](gitlab-cli-journeys.log) pass in 99.82
seconds: exact-commit CI, real-expiry revalidation, failed repair/busy stop, MR
window traversal and CLI/owner/keyring restart reuse. These use private HTTPS
fixtures and an owned keyring, not a dedicated GitLab sandbox. The harness uses
the release generic CLI and its Cargo-built debug native GitLab executable;
[their exact digests](verified-executables.sha256) are retained separately from
[the release build outputs](release-executables.sha256). Read-only copies remain
under the task's `issuer-keys/verified-bin/` directory.

## Inputs, corrections and limits

The starting source revision was 40d43a11cac1880dd9f098d47e63cbf47cbcd499.
[Source inputs](source-inputs.sha256), [tool inputs](tool-inputs.sha256) and
[versions](versions.txt) retain the tested identities. Cargo.lock adds only the
CLI's direct dependency on the already pinned UUID crate. No vendor refresh or
new cryptographic dependency occurred. The generated parser was enrolled using
an exact regenerated HEAD reference before its owning inputs were regenerated.

[Corrections](corrections.md) retain compiler, fixture, projection, ownership and
conformance failures. After the full gate, one contract paragraph clarified that
passive status creates no authority/key records while allowing existing SQLite
locking/sidecar bookkeeping. [Gate inputs](gate-source-inputs.sha256) and the
[exact post-gate difference](post-gate-input-change.diff) show that this is the only
non-evidence/planning source difference; runtime/model inputs are unchanged.

Eight immutable planning reviews are recorded through AEP. Three first-round
findings were fixed; all four second-round reviewers approve. Sonnet was unavailable,
and the inherited model was used; the fourth independent reviewer was delayed by
the three-worker limit. The later CLI projection/conformance fixes preserve the
reviewed outcome. AEP's validator continues to report empty findings arrays as
missing blocks; historical reviews have not been rewritten around that diagnostic.

This checkpoint does not issue approval proofs, qualify production time, supply
caller/subject policy, bind provider dispatch or implement native GitLab writes.
Dedicated sandbox acceptance and the C14 create/update head-guard decision remain
open. The parent goal remains GitLab, Kubernetes, PostgreSQL, MCP and remaining
providers. No Connectors publication/deployment, paid governed run, complete-product
acceptance or reproducible distribution/image build is claimed.

## Website verification

`npm run typecheck` and `npm run build` pass. After the contract wording
clarification, `cargo run --locked --offline -p connectors-build -- docs`, the
installed locked Docusaurus build, and `cargo run --locked --offline -p
connectors-build -- docs --check` pass again. The final output contains 44 contract
pages, 95 reference pages, 116 indexed pages and 474 audited public files.
[Final build output](website-build-final.log) retains the existing CSS minimizer
warnings and successful public-output audit. [Reference checking](website-reference-check.log)
finds no drift. [All fifteen authored example tests](website-example-tests.log)
pass using `cargo test --locked --offline --manifest-path
website/examples/realization/Cargo.toml --target-dir website/.cache/demo/target`.

The implementation was integrated locally using bot author and committer authority
from a clean managed Atlas checkout at ec84b91aa8b474d1ab864b11f8115af73be8bea6,
verified against remote main. No Atlas content or organization registration was
changed. Managed-worktree cleanup is verified after the local commit; its receipt
is retained in the task-owned temporary directory.
