# Local bounded clock verification — 2026-09-10–11

This increment adds `connectors approvals clock-check --adapter forge` and a
process-bound host clock. The [contract](../../../contracts/service/clock.md)
and [guide](../../local-clock.md) define the explicitly admitted source and
Linux timer assumptions. Authenticated acquisition is bounded; every subsequent
observation expands its interval conservatively and refuses uncertain continuity,
expiry, excessive width or a possible UTC day/leap-second boundary. No system
wall-clock fallback, time adjustment, provider request or approval issuance occurs.

The starting commit is `a5b399d4f790e993aa3ab76f6a61ac1ee25b6c7a`; the commit
containing this receipt identifies the increment. [Source hashes](source-inputs.sha256)
exclude planning, historical evidence, wave receipts and the externally edited
AGENTS.md. [Tool inputs](tool-inputs.sha256) and [versions](versions.txt) identify
the selected environment. Cargo.lock adds only the CLI test dependencies on
already pinned base64 and ring; no new production dependency or vendor refresh
is introduced. Existing migration bytes and native GitLab behavior are unchanged.

## Verified runtime and compatibility

| Command | Observed result |
|---|---|
| `cargo test -p connectors-host local::clock -- --nocapture` | [14 deterministic passes](clock-tests.log.gz), one explicit live test ignored. The final gate repeats these after the Clippy changes. |
| `cargo test -p connectors --test local_cli` | [Four production CLI passes](cli-tests.log.gz), repeated by the final gate. |
| `cargo run -p connectors-build -- gate --msrv` | [Complete gate passed](repository-gate.log.gz): shared/native ESS, generation, 73 structural CLI cases, runtime conformance, workspace tests, Clippy, boundaries, Rust 1.88 all-target checks and AEP. The host suite has 91 passes and 17 ignored auxiliary/integration entries. |
| `cargo build --release -p connectors` | [Optimized CLI built](release-build.log.gz). |
| `cargo test -p connectors-host local::clock::tests::independent_live_source -- --ignored --exact --nocapture` | [One explicit live pass](live-source-final.log.gz) after the final codec changes, with UTC interval [1789078325997,1789078328062] ms, width 2,065 ms. |
| `CONNECTORS_TEST_CLI=$CARGO_TARGET_DIR/release/connectors cargo test -p connectors-host --lib local::approval_keys -- --include-ignored --skip key_crash_child --test-threads=1` | [Eleven key tests passed](key-tests.log.gz) in 8.62 seconds. |
| `CONNECTORS_TEST_CLI=$CARGO_TARGET_DIR/release/connectors cargo test -p connectors-gitlab --test local_runtime -- --ignored --test-threads=1 --nocapture` | [All six GitLab journeys passed](gitlab-cli-journeys.log.gz) in 101.65 seconds. |

Every Cargo command in this table also uses `--locked --offline` and the bounded
environment below. Native fixtures verify independent signed packets, unknown
metadata, every truncation and byte substitution, exact Merkle paths/indexes,
rate/rounding/age/width/overflow and UTC-day boundaries, a real fork, discontinuity,
oversized/delayed/unavailable UDP peers and safe refusals. The CLI test returns a
bounded observation, replaces only the configured source key and refuses in a
second process. It removes the state directory and supplies a nonexistent adapter
executable, proving that neither invocation needs metadata or service startup.
Listing performs no exchange. Fixture signatures use a separate test encoder.

The key suite reruns the real disposable Secret Service restart, custody failure,
crash, concurrency, rotation, recovery, revocation and exact-retirement cases
because this increment changes the CLI/configuration and lock inputs. All six
GitLab journeys use the optimized generic CLI and the Cargo test-profile native
GitLab executable; [their hashes](executable-sha256.txt) and read-only task-owned
copies under clock-runtime/verified-bin preserve that exact pair before packaging.
These are disposable private HTTPS/provider fixtures, not GitLab sandbox evidence.

## Website and packaging

`npm run typecheck` in website [passes](website-typecheck.log.gz).
`npm --prefix website run build` [passes](website-build.log.gz), including
Rust/WASM generation, broken-link/anchor checks, 119 indexed pages and 483
audited public files. The [reference check](docs-check.log.gz) reports 46
contract pages, 98 total reference pages and no drift. The locked offline
example suite [passes all 15 tests](website-examples.log.gz) using
`cargo test --manifest-path website/examples/realization/Cargo.toml --target-dir
website/.cache/demo/target`. Presentation and browser interaction code are
unchanged; no new browser-interaction acceptance is claimed.

The [initial website build](website-build-initial.log.gz) refused a guide selected
through the contract-only reference inventory. The selection was removed and the
clock instructions added to the existing authored getting-started guide. The
clock contract remains a selected canonical reference. These two documentation
files are the only differences between the [gate source hashes](gate-source-inputs.sha256)
and final inputs; [the exact hash delta](post-gate-source-hashes.diff) is retained.
The corrected website/reference checks passed without changes to runtime/model
inputs or bypassing the public-source boundary.

`cargo run --locked --offline -p connectors-build -- package --output
.local/tmp/gitlab-runtime-20260910/clock-package --image
connectors-v2-gitlab:clock-20260910 --jobs 2` [passes](package.log.gz).
The [build receipt](package-build-evidence.json),
[source capture](package-source.sha256.json.gz),
[rootfs manifest](package-rootfs.sha256.json) and
[realization](package-realization.json) retain exact local inputs and artifact
identities. This is a local image build with separate runtime acceptance, not a
registry publication or proof of two isolated reproducible builds.
[All 503 selected source hashes match the package capture](package-source-check.json).

## Independent protocol evidence

A task-owned Rust 1.88 probe uses exact `roughenough-client` and
`roughenough-protocol` 2.0.0. Its [manifest](probe/probe.Cargo.toml),
[lock](probe/probe.Cargo.lock), [resolver configuration](probe/resolver-config.toml),
[initial probe](probe/initial-probe.rs), [capture program](probe/capture-probe.rs),
[first output](probe/build-run.log) and [capture output](probe/capture.log) are
retained. To reconstruct that independent probe, copy the manifest/lock to
Cargo.toml/Cargo.lock in an empty task-owned directory, the resolver to
.cargo/config.toml and the chosen source to src/main.rs; run
`CARGO_BUILD_JOBS=2 cargo +1.88.0 run --locked`. The capture writes request.bin and
response.bin in that working directory. This is research evidence, not a production dependency.

The successful capture from roughtime.se at `192.36.143.134:2002` has midpoint
1789076475, radius one second and observed round trip 86 ms. Its exact
[request](../../../crates/connectors-host/tests/fixtures/clock/roughtime-se-request.bin)
has SHA-256 `7a53f9a08e3c6559a00806af9e8798f25951c893674a83f7610ac303561fe8b2`;
the [response](../../../crates/connectors-host/tests/fixtures/clock/roughtime-se-response.bin)
has SHA-256 `71bd425e48143d6373b504f59035fc5a8be2345af25b0a3a32820ed97923c69f`.
These fixtures are verified by the new production codec without a current-time
assertion. Separate fresh acquisitions use fresh random nonces.

The first probe also records an unfamiliar advertised version refused by the
independent library at time.txryan.com, and a Cloudflare timeout. The new bounded
codec accepts unknown advertised versions only when the exact requested signed
version remains present; it adds no downgrade. An earlier `roughtime` 0.1.0
candidate required Rust 1.89 and was not selected. A lookup of nonexistent
`roughenough` 1.3.0 failed; the separately inspected 1.3.0-draft14 was not selected.

## Commands and corrections

All repository Cargo commands use `--locked --offline`, two jobs,
`TMPDIR=$PWD/.local/tmp/gitlab-runtime-20260910` and
`CARGO_TARGET_DIR=$TMPDIR/target`. The gate owns an additional temporary directory
under `.local/tmp`. Cargo builds, service fixture suites and packaging are
serialized. The independent probe has its own Cargo target directory and lock.

The initial clock test tried to corrupt an all-zero Merkle index by writing the
same all-zero bytes. Its failed assertion is retained; the fixture now writes
nonzero bytes. A separate test was corrected to avoid a real-time read masking
its intended synthetic boundary. Neither change weakened protocol verification.
Two rejected patch applications were atomic and changed no files. The initial
probe command selected a directory before it existed; it was recreated from an
existing working directory and then ran successfully.

The first repository gate found a stale 22-command conformance inventory.
The final inventory contains 23 commands, exercises clock-check dispatch with no
protected input acquisition and adds four authored structural fixtures. The
second gate found duplicate fixture-module loading and Clippy style violations.
One shared test module and Rust-1.88-compatible slice APIs resolve them without
lint exceptions. Initial failures remain evidence, not passing acceptance.

## Boundaries and remaining work

Signatures establish an assertion from the configured source. Neither the live
exchange nor deterministic tests establish the source's physical UTC accuracy,
the host's timer-rate bound or protection against a compromised kernel. The
operator must qualify those infrastructure assumptions before using this profile
for approvals. The model retains their explicit UNMAPPED qualification boundary.
Public clock-check output is historical observation and cannot reconstruct a
clock capability or authorize a later request.

Eight immutable AEP reviews preserve both rounds. The first design review's
missing dependency on the mutation ledger was fixed and recorded as one outcome;
all four second-round critics approve. Sonnet was unavailable, so the inherited
model was used; the fourth independent reviewer followed the first three under
the worker limit. Root remains the sole source/planning writer. The later UTC
midnight guard, command fixtures and Clippy fixes preserve the reviewed outcome.
[AEP validation](aep-validation.log) is retained verbatim: 223 artifacts, valid,
with 134 historical/empty-array findings-block notices. The clock story is
implemented against its conditional acceptance; the parent initiative stays active.

Approval issuance, authenticated current subject policy, current clock-selection
guarding, approval/audit/attempt dispatch and native GitLab writes remain required.
Dedicated GitLab sandbox evidence and the create/update atomic-head decision are
still missing. The order remains full GitLab, Kubernetes, PostgreSQL, MCP and
remaining providers. No provider-batch completion, reproducible distribution,
source release, deployment or paid governed run is claimed by this clock increment.

The read-only Atlas authority checkout is based on remote main
`15c99a14a78148e97ceda0c5f3ad7511b465c58c`;
[authority hashes](atlas-authority.sha256) identify its instructions and private
bot wrapper. No Atlas content or registration changes are part of this increment.
The externally edited AGENTS.md and live managed tree
`connectors-release-v020-20260910` belong to separate release work and are preserved.
