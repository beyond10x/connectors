# Local approval CLI checkpoint

The generated production CLI now implements `approvals policy-status`,
`policy-set`, `prepare` and `issue` under the active `story:guarded-gitlab-merge`.
The [guide](../../local-approvals.md) and
[owning contract](../../../contracts/service/local-mutations.md) describe their
boundaries. Production GitLab still advertises eleven reads. This checkpoint
does not implement proof consumption, owner mutation IPC, the final combined
dispatch coordinator or a native GitLab merge.

Policy publication admits only configured, supported private-protocol-two writes
from the selected cached descriptor, retaining exact configuration, executable
and clock selections. First publication requires absence; later publication uses
the existing revision CAS. Empty operations revoke new policy uses. Inspection
is passive and does not install a migration.

Preparation validates original JSON and the native write schema, including
formats. It checks the complete retained connection binding and scopes through
a passive read transaction. Expired baseline evidence can identify a target;
known invalid or revoked material cannot. Independent SQLite observers prove
that preparation and refusals do not update the registry clock or commit data.
No custody, clock network, provider, service startup, reservation or spend occurs.

Issuance independently reconstructs the subject and compares the explicit
approved digest. Fresh authenticated clock acquisition precedes policy/key
leases, and the subject/policy are resolved again afterward. Exact current policy
and active-key leases surround signing and protected file publication. The seed
and compact proof remain in protected buffers; ordinary output contains only a
reference, digest and definite publication disposition. File and directory sync
precede success; existing destinations are never overwritten. A failed check
after publication reports uncertain acknowledgement and preserves the file.

## Verification

Linux x86_64 runs use task-owned
`TMPDIR=$PWD/.local/tmp/gitlab-runtime-20260910`,
`CARGO_TARGET_DIR=$TMPDIR/target`, `CARGO_BUILD_JOBS=2` and
`RUST_TEST_THREADS=2`. Cargo runs are serialized. The exact built CLI is selected
with `CONNECTORS_TEST_CLI=$CARGO_TARGET_DIR/debug/connectors`.

| Command | Observed result |
|---|---|
| `cargo run --locked --offline -p connectors-build -- cli` | Ten generated CLI artifacts; corrected canonical model and structural fixtures validate. The subsequent gate validates 81 CLI structural cases and the existing acquisition/page obligations. |
| `cargo clippy --locked --offline -p connectors-host -p connectors -p connectors-build -p connectors-conformance --all-targets -- -D warnings` | Passed after sharing the existing clock fixture module. |
| `cargo run --locked --offline -p connectors-build -- gate --msrv` | Passed: shared ESS and six independent native roots, CLI/adapter generation and drift, all workspace tests, formatting, Clippy, library boundaries, Rust 1.88 all-target checks, conformance and AEP. Host unit tests: 122 passed, 19 explicitly ignored; the new production CLI journey was separately invoked below. |
| `cargo test --locked --offline -p connectors-host --lib local::owner::approval_issuance::tests::production_cli_approval_issuance_and_restart -- --ignored --exact --nocapture` | Passed on the final CLI in 1.32 seconds, using qualified disposable GNOME custody and an independently encoded synthetic clock response. |
| `cargo test --locked --offline -p connectors-gitlab --test local_runtime -- --ignored --test-threads=1 --nocapture` | All six production CLI journeys passed in 256.70 seconds, including CI/traces, real-expiry revalidation, failed repair/busy stop, MR windows/validation and CLI/owner/keyring restart reuse. |
| Website `npm run typecheck`, `npm run build`; `cargo run --locked --offline -p connectors-build -- docs --check` | Passed. Generated 46 contract pages and 99 references, built Rust/WASM examples, indexed 120 pages, audited 487 public files and confirmed no reference drift. |

Conformance retains 315 scenarios (34 authored) and 22 existing synthesis
refusals. AEP retains 231 artifacts and its 139 existing review-format notices.
The full unedited planning output is [aep-validation.log](aep-validation.log).
These checks do not replace the pending native mutation or provider sandbox
acceptance. The runtime commands above exercise their actual production handlers.

The approval CLI journey initializes a real qualified signing key through the
CLI, sets policy, prepares and issues, and verifies the protected proof after the
issuing process exits. It hashes the printed canonical subject independently,
checks explicit null coordinates and mode-0600 publication, refuses overwrite
without a new clock query, reuses custody after keyring restart, and refuses new
issuance while custody is locked. Its cached write descriptor and retained
provider connection are fixture metadata; no provider executable exists. It
proves the approval CLI boundary, not a callable native business write.

Deterministic host tests cover stale description/schema, missing permissions,
duplicate JSON keys, invalid dates, target-envelope size, expired total budget,
policy CAS/revocation, changed clock/connection bindings, passive observations,
known invalid material, and proof publication failure. A clock responder updates
policy exclusively during acquisition; issuance refuses the changed subject,
proving that network acquisition holds no policy/key/metadata lease. A separate
fault test fails the acknowledgement check after actual durable file publication
and proves that the retained file cannot be replaced by a later attempt.

## Corrections and limits

The first generation refused a file-only document source because pinned
`ess-cli/1` requires inline, file and stdin carriers together. The authored
policy-set input now names a file path; the trusted handler performs bounded
file acquisition. No extra input carrier or generator override was added.

The canonical preparation fixture exposed a real model/projection mismatch:
optional object fields project as omission, but canonical subjects require
explicit null members. The delegation owner now uses named nullable values and
a canonical authority-scope projection. Other idempotency scope projections keep
their original meaning. Positive null and negative missing-route schema fixtures
are retained; the existing proof codec's canonical byte format is unchanged.
The CLI fixture checker now prints the failing instance paths and schema reasons.

Initial compilation corrected a mistaken fallible clock-digest call and an owned
descriptor borrow. Early tests incorrectly retained a metadata lifecycle lease
across another open, and constructed deletion rows that violated SQLite checks;
the independent observer and fixture rows were corrected. The CLI test initially
expected empty stderr even for structured failures; it now checks the actual
success/error channels and forbids proof members in both. The first gate caught
the old 23-command inventory; it now covers all 27 commands. A later Clippy run
caught duplicate inclusion of the independent clock fixture; tests reuse its
existing module, with no lint suppression. Failed and corrected logs are retained.

The source baseline is `bab9751daab36b043f0d5112c3c88199781166a7`.
The source manifest records 412 source/dependency inputs. Runtime identities
record the exact CLI/native adapter used by the explicit journeys and the
qualified keyring/D-Bus executables. These are verification binaries, not proof
of complete distribution reproducibility. Cargo, ESS and website dependency pins
are unchanged; the ESS source pin is `6f7ef46163e758f3401945d1a946e0fc80ebc003`.
The later workspace gate rebuilt development executables with its workspace
feature selection. Their differing digests are retained separately in
`runtime-artifacts-after-gate.sha256.gz`; the explicit production journeys refer
to `runtime-artifacts-cli.sha256.gz`. No equivalence of those payloads is claimed.

Atlas authority was checked from its clean primary checkout and remote main at
`1e9ea6546fcecbc87335d9f407f17790c296cc4e`; the private wrapper SHA-256 is
`8645c7100bda1e1f5a8285aa93f7feed045779ddf503d440dc2df15615c1e6ee`.
The installed Connector returned `connector-unreachable`, reported before using
Git's read-only remote check. Root is the sole implementation/planning writer,
directly on primary main under the repository-specific single-agent rule. No
linked worktree was created and no source release or publication is claimed.

The story and initiative remain active. Next join protected proof consumption
and owner mutation IPC to the current approval/audit/attempt/connection gates,
then implement one SHA-guarded GitLab merge and its lost-response/restart
acceptance. Dedicated GitLab sandbox access, create/update atomic-head semantics,
full distribution reproducibility and the rest of the GitLab batch remain open.
The order remains full GitLab, Kubernetes with Helm, PostgreSQL, MCP and remaining
providers. The existing paid-driver blocker is unchanged; no governed run is
claimed or required here.
