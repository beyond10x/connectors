# Kubernetes acceptance — 2026-10-02

Four new explicitly selected real-provider cases passed through production CLI
and adapter processes against the owned k3s v1.31.5 fixture. Four existing ignored
CLI journeys also passed after the shared test process helper changed. The ordinary
package separately ran 39 passing cases and ignored eight. These counts describe
different executions; ignored cases are not live-provider proof.

| New case | Actual result |
| --- | --- |
| Admitted reads across restart | [1 passed, 16.41 s](live-restart-1.log) |
| RBAC versus empty/scope/invalid-token outcomes | [1 passed, 8.54 s](live-rbac-1.log) |
| Continuation scope and revision binding | [1 passed, 17.28 s](live-continuation-3.log) |
| Repair, revoke, custody and exact-child stop | [1 passed, 16.30 s](live-lifecycle-2.log) |

The tests provision unique objects in the authorized fixture namespaces, invoke
real selected resources/endpoints/hosts and check identities/provenance. A bounded
test HTTPS observer verifies the real API's CA, forwards only selected requests,
counts method/path/status without credentials, and can hold a completed real
response for lifecycle ordering. It does not simulate the upstream result.

The first [continuation](live-continuation-1.log) and
[lifecycle](live-lifecycle-1.log) attempts exposed newly authored assertions against
the wrong error envelope, not production defects. An intermediate
[accessor correction](live-continuation-2.log) also failed. The final assertions
require `service_failure` / `stale_cursor` / `dispatch` and
`unavailable` / `readiness`, respectively, while preserving all other controls.
The exact [assertion delta](assertion-corrections.patch) and every failed attempt
are retained. Explicit [missing prerequisites](missing-prerequisite.log) refused
before provider provisioning. Product deadlines and the 60-second evidence
lifetime were unchanged.

[Package](final-package.log), [Clippy](final-clippy.log) and
[formatting](final-fmt.log) completed with exit 0. The four `existing-*.log` files
retain each pre-existing journey's separate result. Final
[fixture residue](fixture-residue.log) and [empty namespace](empty-residue.log)
observations show no new test objects left behind; root's original fixture remains.
[Identities](identities.sha256) distinguish source and final binaries. Earlier
test binaries differed only in the retained assertion corrections; production
CLI/adapter bytes were unchanged across the live runs.

Source base: `f3fb222b7edc7fc29520dd30effdb58bfdec5274`; frozen authored patch:
`aa9bb1ced05c6fbd84260d11c432bea3f652bb9a392ab0461e11caf2f2889dfa`.
This unit changes tests and a guide only. It does not establish SSAR, logs,
rollout/restart, exec/copy/tunnels, additional Helm operations, deterministic
packaging, a workspace/MSRV gate or a source release. A pending unschedulable Pod
and zero-replica Deployment prove object identity, not workload readiness.

Published copies replace the local home prefix with literal `$HOME`.
[Original hashes](original-inputs.sha256) identify pre-redaction evidence bytes,
not checksums of these published copies. Raw logs remain privately retained.

The subsequent [read-only adversary pass](review.md) found no concrete defect and
ran no additional tests. The verified worker results remain the runtime evidence.
Reviewed source commit: `8ce90cb6d0f765a78f4273ca0ccade1ed7253a35`; local integration
is complete, with the combined repository gate and source release still pending.
