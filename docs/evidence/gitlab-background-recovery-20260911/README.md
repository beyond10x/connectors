# Background GitLab mutation recovery evidence

Local development increment under active `story:guarded-gitlab-merge`, based on
`0bc49610d5c5a1030f8ab05e782a5e63e7ebcb43`. This receipt and implementation belong
to the same source commit. The provider batch and parent initiative remain open.

## Binding and scope

The running owner now recovers abandoned attempts without waiting for another
invocation of the original business key. It waits five seconds between scans,
uses the existing instance/state index, visits at most eight instances and
returns at most 64 pending references for one instance. The private transient
cursor rotates across retained records, including attempts without a caller key.
The scan neither installs a mutation schema nor reads result payloads. It closes
metadata before acquiring configured trusted time for Prepared entries.

Recovery runs between jobs on the existing instance worker. Only one maintenance
batch may be queued for an instance. With no live worker, the supervisor holds
its worker-creation lock while applying the exact retained attempt/instance
fences. A busy/idle hint does not establish ownership. The owner's existing
lifetime lock excludes another owner process, and shutdown stops and joins both
maintenance and instance workers. The batch checks a 250 ms budget between
attempts; an individual transaction retains its existing bounded metadata wait.

Prepared settlement requires the currently selected qualified clock. Missing or
changed time leaves it pending. Dispatching becomes Indeterminate with a
quarantined key and no expiry, without requiring time. A failed acknowledgement
is followed by one authoritative observation, without an immediate mutation
retry. A later scheduled pass requires fresh positive pending evidence.

Housekeeping can fence an unkeyed attempt or a target whose connection is revoked
or whose adapter was removed from configuration. It grants no result access,
reads no proof or credential, starts no worker or adapter child, lifts no
suppression, and creates no provider-send authority. Current admission continues
to govern result disclosure. Key expiry, history reclamation and original-audit
reconciliation are outside this increment.

No persisted schema, migration, native business semantics, dependency pin,
generated projection or public/private wire format changes. The existing
AttemptRecord model and retained instance/connection references own the semantics;
the new scan cursor and recovery batch are private transient implementation values.

## Runtime evidence

The complete ignored GitLab runtime suite passes all 12 tests in **460.12 seconds**:
11 production CLI journeys and one inert subprocess fixture driver. It includes
all earlier read, credential persistence, repair, stop, revocation, exact-key
recovery and guarded-merge cases, plus these two new journeys:

- A real CLI merge applies one provider PUT while its response is withheld. A
  separate unkeyed preparation on the same instance makes a background pass
  request time; both attempts remain pending while the native exchange is live.
  The fixture kills the exact socket-derived owner through its pidfd and verifies
  owner/native-child exit. A separately admitted read starts a new owner, then
  the adapter and custody are stopped. Without resubmitting the original key,
  background recovery quarantines the dispatch while time is unavailable and
  settles the unkeyed preparation after time returns. Suppression persists;
  there is exactly one PUT and no further provider call. Later original-key
  observation reports the retained unknown result.
- A CLI journey revokes the old connection and removes its adapter from current
  configuration. A separately configured native instance starts the new owner
  through production CLI connect, so no old instance worker exists. After that
  child and custody stop, the background pass leaves the old unkeyed preparation
  pending without time and settles it when time returns. Restoring configuration
  does not restore the revoked connection's result-access grant: a later request
  is refused without a mutation observation or provider call.

The seeded preparation is produced by a separate durable-port subprocess that
exits with status 73 after acknowledged preparation. It uses
`Approval::NotRequired`, without native preflight, proof spend or a dispatch gate.
It does not establish an approved native crash before dispatch. The live fixture
uses an independent clock-request witness and bounded queue-handoff interval;
the deterministic supervisor unit test separately proves that a live worker with
an idle hint receives one coalesced recovery batch rather than being bypassed.

The three new ledger tests cover passive scanning on schema three without a
migration, pagination across instances and keyed/unkeyed attempts, exact-instance
refusal, revoked targets, clock failure, rollback and uncertain acknowledgement.
The corrected ledger suite passes 17 tests with one explicitly ignored case.
The supervisor queue/exclusion test passes independently.

## Commands and retained inputs

Linux x86_64; ordinary Rust 1.98.1, minimum Rust 1.88.0. Cargo commands are
serialized with two build jobs. The task-owned TMPDIR is
`.local/tmp/gitlab-runtime-20260910`; CARGO_TARGET_DIR is its `target` subdirectory.
Ordinary RUST_TEST_THREADS is two; provider journeys select one test thread.
CONNECTORS_TEST_CLI selects the recorded `target/debug/connectors` executable.
The ESS source and dependency pins remain those in Cargo.lock and
`crates/connectors-spec/toolchain.json` (ESS 0.20.0 at
`6f7ef46163e758f3401945d1a946e0fc80ebc003`).

```sh
cargo clippy --locked --offline -p connectors-host -p connectors-gitlab --all-targets -- -D warnings
cargo test --locked --offline -p connectors-host --lib maintenance_tests
cargo build --locked --offline -p connectors
cargo test --locked --offline -p connectors-gitlab --test local_runtime -- --ignored --test-threads=1 --nocapture
cargo run --locked --offline -p connectors-build -- gate --msrv
```

`runtime-artifacts.sha256` identifies the tested CLI, native adapter, integration
test executable, Secret Service daemon and D-Bus daemon. The compressed source
manifest records 180 selected source/dependency inputs, explicitly including both
new source modules. The base commit retains unchanged surrounding inputs.

The full repository gate passes shared and six native ESS roots, generation and
drift checks, Clippy, dependency boundaries and all-target Rust 1.88 compilation.
The host suite has 129 passes and 19 explicitly ignored tests. Conformance reports
315 scenarios, including 34 authored cases, and 22 existing synthesis refusals.
The direct built `connectors-build docs --check` reports 46 selected contracts,
99 total reference pages and no drift. Website presentation, authored guides,
selected public source bytes and example implementations are unchanged; the
[preceding website evidence](../gitlab-owner-recovery-20260911/README.md) remains
applicable. The new local binding link resolves to its existing source.

The gate builds different workspace executable selections. Their intermediate
identities are retained separately in `gate-artifacts.sha256`. Repeating the exact
package build and runtime-test compilation restored all three tested executables
byte-for-byte; `sha256sum --check` passes for all five runtime inputs. Source
checksum verification also passes for all 180 selected inputs. Logs are retained
as deterministic gzip containers, separately from source deliverables.

AEP validation output is retained verbatim, including the existing historical
prose-only review notices. Atlas bot authority was verified against clean local
and remote main at `1e9ea6546fcecbc87335d9f407f17790c296cc4e`. The authority file
and wrapper digests match the preceding checkpoint. `worktree repo list` reports
only the unmanaged primary Connectors checkout, with no linked tree to retire.

## Failed checks and corrections

The first ledger test run failed two new fixtures: one had not selected private
0700 permissions for its temporary state directory; another synthetic revoked
record omitted its schema-required revocation timestamp. Correcting the fixture
inputs produced 17 passes and one ignored test in 12.79 seconds, without weakening
production validation.

The first removed-target fixture attempted to start an owner through the host
client library from the integration-test executable. That launch uses
`current_exe`, which was the test binary and did not provide the production owner
entrypoint. The corrected fixture starts through the actual CLI using another
native instance. It passed in 39.99 seconds; the live/crash/unkeyed fixture passed
separately in 44.46 seconds. The final complete regression above governs the final
source and executable bytes.

Two Clippy runs caught test-only findings, `drop_non_drop` and
`unnecessary_get_then_check`. Both were corrected; final Clippy and the queue test
pass. Original failed logs are retained beside the corrected runs.

## Remaining work

Original-audit reconciliation, every storage-acknowledgement crash and the broader
concurrent admission/final-audit failure matrix remain open. Dedicated GitLab
sandbox acceptance, the C14 create/update atomic-head decision, the other selected
GitLab workflows and isolated distributable reproducibility are still required.
Existing sandbox input requests remain pending; installed credentials are not
migrated or inspected. This evidence establishes disposable local fixtures, not
real GitLab sandbox behavior or two isolated reproducible distributions.

The story, initiative and goal stay active. Full GitLab still precedes Kubernetes
including Helm, PostgreSQL, MCP and remaining providers. Root is the sole
implementation and planning-store writer on primary main. No new decomposition
or critic panel is needed for this existing story's runtime increment. No linked
tree, release tag, push, deployment, Atlas registration or paid governed run is
part of this checkpoint. The driver protocol-loading blocker remains unchanged.
