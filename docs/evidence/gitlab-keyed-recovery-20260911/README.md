# Exact-key GitLab mutation recovery evidence

Local development increment under active `story:guarded-gitlab-merge`, based on
`08cf5551823ac9aa6692e2f61b917e1904d88648`. This receipt and implementation belong
to the same source commit. This is not a completed provider batch or release.

## Binding and scope

An admitted exact-key observation now distinguishes a pending durable original
from a terminal result. A terminal result retains its passive path. A pending
original may start the owner and queue recovery on its serialized instance worker.
A busy worker returns the pending observation; its busy flag is only a latency
hint. Recovery authority comes from execution between jobs on the retained worker,
with an exclusive borrow of its native child slot. The owner lifetime lock stays
held through kernel process exit, including early return or unwinding after
request admission begins.

Recovery rechecks current policy and target admission. It applies the existing
ledger transitions: Prepared becomes Aborted with known non-dispatch; Dispatching
becomes Indeterminate with a permanently quarantined key. Prepared settlement
requires fresh configured trusted time for the fixed replay retention interval.
Time acquisition happens before policy and metadata locks. Missing time leaves
the preparation pending. Unknown dispatch quarantine needs no clock.

No failed recovery acknowledgement grants another recovery mutation or provider
send: the coordinator reads the authoritative original once. Recovery consumes no
new proof, credential or native capability and does not lift suppression. The
recovery task starts no adapter child; newly starting an owner still honors its
separately configured automatic startup settings. The fixtures select on-demand
startup and verify no native child is launched by recovery.

There is no persisted schema, migration, generated projection, native behavior,
dependency pin or wire-format change. The additive Rust observation wrapper is
not serialized. Existing live replies with `replayed=false` cannot be mistaken
for proof that a durable original remains pending.

## Runtime cases and limits

The four-outcome production merge fixture covers Applied, Refused, lost response,
and an owner killed after one provider-applied PUT while its reply is withheld.
The last case retains its live concurrent observer, exact socket-derived owner
pidfd, native-child exit verification and spent-proof check from the preceding
increment. After owner death, the production CLI with its proof file deleted and
custody stopped starts a different owner and quarantines the original attempt.
The test reads the durable Indeterminate/Quarantined state, without settlement
expiry, and proves exactly one PUT. Terminal replay starts nothing.

A separate subprocess fixture exits with status 73 immediately after acknowledged
durable preparation. With proof deleted and custody stopped, production CLI
recovery makes one trusted-clock request. An unavailable response leaves Prepared
pending with the original attempt and an unavailable cause. Restoring time allows
the next observation to settle Aborted/Replayable, retaining the original attempt
and request and exactly 86,400,000 milliseconds between settlement and expiry.
A subsequent terminal observation with time disabled starts no owner, makes no
clock request and does not extend that interval. No provider request or PUT occurs
during any of these recovery observations.

The preparation producer calls the existing durable port with Approval::NotRequired
and no native preflight, proof verification/spend or dispatch gate. Its process
exit proves port-producer durability plus production recovery. It does not prove
a production approved-merge crash between preflight/spend and dispatch.

## Verification

Linux x86_64. Task-owned TMPDIR is `.local/tmp/gitlab-runtime-20260910`, with
CARGO_TARGET_DIR at its `target` subdirectory. CARGO_BUILD_JOBS and ordinary
RUST_TEST_THREADS are two; the runtime journeys run serially. CONNECTORS_TEST_CLI
selects the exact built `target/debug/connectors`. Cargo runs are serialized.

The first complete runtime run had eight passing tests and two admission timeouts:
initial connect in the revocation journey and concurrent owner startup in the
persistent journey. Both exact isolated rechecks passed with unchanged source,
binaries and deadlines (19.35 and 50.42 seconds respectively). The latter includes
a task-scoped syscall trace without request or credential payloads. A second full
run failed three startup/admission cases, so the isolated passes did not close
the failure. Both failed full logs are retained. One initial recheck used a short
name with `--exact` and selected zero tests; that log is retained and not counted.

The trace and source then identified three full SHA-256 passes over the roughly
99 MB debug owner executable before startup. Those passes take approximately
9.6 seconds together in this trace, leaving almost no margin in the unchanged
ten-second budget. The private launch path now checks source ownership/type/path
and hashes during sealed snapshot capture, omitting the redundant pre-capture
content pass. Owner launch retains its initial self-identity hash; native adapter
launch needs only its configured digest and verified snapshot. The public
Executable::open/check contract is unchanged, and unverified source opening is
private to the configuration implementation. No startup deadline is extended.
This removes a measured cost; the trace is not proof of every failed call's
precise timing. Focused capture tests pass, including changed source content,
snapshot preservation, symlink/broad-permission refusal and the original deadline.

The final complete runtime suite passes all ten tests in 368.12 seconds: nine
production CLI journeys plus the inert subprocess fixture driver. It runs the
stronger crash recovery, clock-unavailable preparation recovery and every prior
read/lifecycle case on the updated startup path. The invocation is:

```sh
cargo test --locked --offline -p connectors-gitlab --test local_runtime -- --ignored --test-threads=1 --nocapture
```

The preceding package build and test compilation select the production binaries
recorded in `runtime-artifacts.sha256`. `source-inputs.sha256.gz` records 100
selected source/dependency inputs, including the new recovery module; the base
commit retains unchanged surrounding inputs. Failed/pre-fix runtime identities
are recorded separately. None of this claims isolated distributable reproducibility.

The full repository gate passes, including shared and six native ESS roots,
generation/drift, Clippy, dependency boundaries, Rust 1.88 and conformance (315
scenarios, 34 authored, 22 existing synthesis refusals). The host unit suite has
125 passes and 19 explicitly ignored cases. Reference verification also passes:
46 selected contracts, 99 total reference pages, no drift. Website authored
inputs, selected public content, presentation and example implementations are
unchanged; the preceding receipt's website checks remain applicable. The new
relative contract link resolves to its existing local source.

| Command | Result |
|---|---|
| `cargo test --locked --offline -p connectors-host --lib local::runtime::artifact::tests` | Both snapshot/admission tests pass. |
| `cargo build --locked --offline -p connectors` | Production CLI build passes. |
| `cargo test --locked --offline -p connectors-gitlab --test local_runtime --no-run` | Native executable and runtime test build pass. |
| Full runtime command above | Ten tests pass; nine journeys and one fixture driver. |
| `cargo run --locked --offline -p connectors-build -- gate --msrv` | All required checks pass. |
| Built `connectors-build docs --check` | No reference drift. |
| Repeated exact package build and test compilation, then `sha256sum --check` | All five runtime artifact identities match the tested bytes. |

The gate builds different workspace executable selections. Their intermediate
identities are retained separately in `gate-artifacts.sha256`; the exact package
builds restored the accepted CLI, native adapter and integration-test executable
byte-for-byte. Runtime evidence is not attributed to the gate's different bytes.
Final source checksum verification also passes. AEP validation output is retained
verbatim, including its existing prose-only historical review notices.

## Remaining work

Background recovery, unkeyed/unobserved abandoned-attempt cleanup, crashes across
every storage acknowledgement and the broader concurrent admission/final-audit
failure matrix remain required. Dedicated GitLab sandbox access, C14 create/update
atomic-head semantics, the other selected GitLab workflows and isolated
distributable reproducibility remain open. Existing sandbox input requests remain
pending; no installed credentials are migrated or inspected.

The story, initiative and goal remain active. Full GitLab still precedes
Kubernetes including Helm, PostgreSQL, MCP and the remaining providers. Root is
the sole implementation/planning writer on primary main. This increment creates
no linked tree, source release, external push, deployment, Atlas registration or
paid governed run. The existing driver protocol-loading blocker is unchanged.
Atlas bot authority was checked against clean local and remote main at
`1e9ea6546fcecbc87335d9f407f17790c296cc4e`. Only the primary Connectors checkout
exists, so there is no task-owned linked-tree cleanup to apply.
