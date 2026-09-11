# Exact GitLab audit acknowledgement recovery

Development increment under active `story:guarded-gitlab-merge`, based on
`857569291b1a0601b27ad2bb8b050cdefa022f2f`. This receipt and implementation belong
to the same source commit. The full provider batch and parent goal remain open.

## Implemented behavior

Production finalization retains one exact final audit observation while resolving
a failed append acknowledgement. It reads the original host-qualified audit;
only an identical stored observation proves completion. A readable anchor without
a final observation permits one retry using the unchanged UUID, outcome, safe
code and timestamp. A failed retry gets at most one final read. A different final
observation, missing record or unavailable/malformed metadata cannot authorize
overwrite or completion.

Additional storage calls check both the original invocation deadline and a 250 ms
recovery budget between calls. The metadata port retains its bounded wait within
a call. The original first append keeps its existing bounded cleanup behavior.
Unresolved failure reports incomplete audit while preserving the acknowledged
reference and the known business result. Only audit append can be retried; no
provider, mutation gate, approval spend or credential operation is repeated.
Both execution and original-result observation pass their original deadlines to
the finalizer.

This binds the existing AuditRecord and FinalObservation semantics in
`ess/domains/execution_audit.yaml` and audit contract section 3. There is no new
entity, relation, migration, persisted lifecycle, wire format or dependency pin.
Fault scripts, call counters and shared unit fixtures compile only under
`cfg(test)`; the production binaries expose no fault-injection input.

## Verification inputs and results

Linux x86_64, Rust 1.98.1 with Rust 1.88.0 compatibility checks. Cargo commands
are serialized with CARGO_BUILD_JOBS=2 and RUST_TEST_THREADS=2. TMPDIR is the
task-owned `.local/tmp/gitlab-runtime-20260910`; CARGO_TARGET_DIR is its `target`
subdirectory. CLI journeys use one test thread and CONNECTORS_TEST_CLI selects
the recorded `target/debug/connectors` executable.

The audit suite passes 17 tests with one explicitly ignored subprocess entry in
7.41 seconds. Five new cases cover exact-field reuse after rollback, first/second
lost acknowledgement, refusal of all four changed observation fields, expired
recovery budget with no extra calls, wrong identity/malformed metadata, and
stopping after a failed retry. Counters verify the bounds: at most two append
calls and two reads. Reconstruction preserves the same record and observation.
The suite retains the earlier four abrupt process-exit cases.

The first focused production-finalizer check passed 28 constructed-result/fault
combinations in 9.56 seconds. It was then extended with seven Applied cases whose
native response is unusable, for 35 combinations in the final source. These are
real-SQLite tests of the production helper, not injected native CLI failures.
The extra test cases are excluded from production compilation; the running CLI
regression's production inputs remained unchanged by that extension.

That initial CLI regression passed all 12 tests (11 production journeys and the
inert fixture driver) in 447.05 seconds. The first full gate then failed one
existing private write timeout assertion: Unknown/Unavailable was returned where
Unknown/Timeout was expected. Its host result was 134 passed, one failed and 19
ignored; the expanded finalizer cases passed. This failed gate is retained.

Source review found that the child can close at its captured deadline just before
the parent's socket timer fires. The commit path now rechecks both original
deadlines after unavailable transport and reports Timeout if either has expired.
Early loss remains Unavailable; the effect remains Unknown and no send is retried.
An independent peer fixture closes at the captured wall deadline, retaining the
strict timeout assertion and exact child/effect checks. Initial runtime/source
identities are preserved separately from verification of this correction.
All nine private write-protocol tests passed after the fix in 39.04 seconds.

The next full gate passed, including 135 host tests (19 explicitly ignored),
Rust 1.88 and 315 conformance scenarios. Its following CLI regression failed
one of 12 tests in 444.70 seconds: the settled owner-replay check observed six
clock requests instead of five. The immediately preceding provider-count check
passed. The failed run and its executable/source identities are retained as
`prefixture` evidence; they do not establish a repeat GitLab write.

The synthetic CLI clock signed a fixed midpoint for every exchange. A new sample
could then have a lower bound behind the ledger's retained extrapolated bound,
correctly refusing abort settlement of a preparation whose approval was already
spent. Background recovery could legitimately query time for that still-pending
attempt during the replay check. The fixture now advances its synthetic midpoint
by elapsed monotonic whole seconds and one tick per request. The journey also
requires the approval-replayed refusal to settle without an AttemptStore failure,
while retaining strict replay clock counts, provider counts and one-PUT checks.
No production clock or replay rule changes for this fixture correction.
The strengthened four-mode CLI journey passed in 115.90 seconds. Reference drift
checking also passed: 46 selected contracts and 99 total reference pages are
current. None of the changed private contracts or local operating documents is a
selected public input in `website/publication.json`; website/example implementation
and presentation are unchanged. The earlier website build/typecheck evidence in
`gitlab-owner-recovery-20260911` remains applicable to those unchanged inputs.

The full gate was rerun on the corrected fixture source and passed: 135 host tests
(19 explicitly ignored) in 109.30 seconds, shared and six native ESS roots,
generation/drift, workspace tests, Clippy, adapter boundaries, all-target Rust 1.88
and 315 conformance scenarios (34 authored, 22 existing synthesis refusals).
`audit-ack-gate-accepted.log.gz` retains this final passing gate separately from
the earlier failed and pre-fixture runs. Exact package builds after the gate
select the production CLI and native GitLab executables for final CLI acceptance.

Final CLI acceptance passed all 12 tests (11 production journeys and one inert
fixture entry; three ordinary tests were filtered out) in 464.32 seconds.
The four-mode merge journey retained exact provider/effect counts, settled the
refused reused-proof preparation, and performed terminal replay without another
clock request. The background recovery, revocation, trusted-time refusal/recovery,
CI, MR, lifecycle and credential-restart journeys also passed.
`audit-ack-runtime-accepted.log.gz` retains the final result. All five runtime
artifact checksums and all 184 selected input checksums passed after the run.
The final executable identities are the ones in `runtime-artifacts.sha256`;
gate build identities and earlier run identities remain separate. This evidence
does not establish two isolated reproducible distributable builds.

```sh
cargo test --locked --offline -p connectors-host --lib local::audit::tests
cargo test --locked --offline -p connectors-host --lib final_audit_recovery_preserves
cargo test --locked --offline -p connectors-host --lib local::runtime::process::write_tests
cargo clippy --locked --offline -p connectors-host --all-targets -- -D warnings
cargo build --locked --offline -p connectors
cargo test --locked --offline -p connectors-gitlab --test local_runtime --no-run
cargo test --locked --offline -p connectors-gitlab --test local_runtime -- --ignored --test-threads=1 --nocapture
cargo run --locked --offline -p connectors-build -- gate --msrv
"$CARGO_TARGET_DIR/debug/connectors-build" docs --check
```

`source-inputs.sha256.gz` records 184 selected source/dependency/document inputs,
including the new finalizer tests and failure matrix. The base commit retains
unchanged surrounding inputs. ESS remains 0.20.0 at the exact source revision
`6f7ef46163e758f3401945d1a946e0fc80ebc003`, selected by the unchanged toolchain pin.
`runtime-artifacts.sha256` identifies the production CLI, native GitLab binary,
integration test executable, Secret Service daemon and D-Bus daemon.

After the planning updates, standalone `aep plan artifact validate` passed for
231 artifacts with 139 existing notices about historical prose-only review
findings. Its full output was relayed verbatim and is retained in
`audit-ack-aep-validate.log.gz`. Story revision 25 and initiative revision 31 remain
active; the story has ten recorded test-result checkpoints. No new decomposition
was created, so no new critic panel was required.

Atlas authority was freshly checked against clean local and remote main at
`1e9ea6546fcecbc87335d9f407f17790c296cc4e`. Its AGENTS and bot-wrapper SHA-256
identities are respectively
`68012da51a37e1a06776e8b3b47d688093713d885f5f93c47c9956f145b63933` and
`8645c7100bda1e1f5a8285aa93f7feed045779ddf503d440dc2df15615c1e6ee`.

## Scope that remains open

The [failure matrix](../../gitlab-write-failure-matrix.md) identifies each remaining
CLI/owner acknowledgement case separately from existing port, protocol and
finalizer coverage. The full joined failure matrix, dedicated GitLab sandbox
acceptance, C14 create/update head-guard decision, other selected GitLab workflows
and isolated distributable reproducibility remain required.

General original-audit completion after process loss is a separate unresolved
persistence binding. Current anchors precede attempt creation, have no attempt
reference, and retain no pending final-observation intent. A similar request ID
does not establish the missing relationship or recover a lost observation.
This increment implements the exact-observation retry already selected by the
audit contract; it does not invent a background audit reconciliation scheme.

The story, initiative and goal remain active, in the order GitLab, Kubernetes
including Helm, PostgreSQL, MCP and remaining providers. Root is the sole writer
on primary main. No new decomposition, sub-agent, linked tree, release, external
push, deployment, Atlas registration or paid governed run belongs to this
checkpoint. Existing sandbox input requests and the driver blocker are unchanged.
Two managed trees belonging to separate work were observed and preserved:
`http-template-decision-20260911` and `review-warnings-toolchain-20260911`.
Both remain Active; the latter has a live lease and in-progress changes. Their
presence does not authorize this checkpoint to finish or remove them.
The initiative's current publication section now reflects the already-recorded
operator authorization for verified provider source releases. Its older blanket
Connectors publication exclusion was stale. The dated authorization and release
history remain intact; this internal development checkpoint creates no release.
