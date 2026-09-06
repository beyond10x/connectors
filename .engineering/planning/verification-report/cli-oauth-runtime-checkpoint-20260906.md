---
format: aep.planning-md/1
id: verification-report:cli-oauth-runtime-checkpoint-20260906
kind: verification-report
status: draft
title: Partial OAuth runtime implementation checkpoint
relations:
- verifies: story:connect-session-oauth-custody-in-personal-posture
revision: 1
---
## Coordinator verification and limit

This is a partial implementation checkpoint, not an independent review or completed story. Root verified all 167 members of the sealed runtime evidence manifest at 2026-09-06T15:27:07.767181Z; no mismatches. Manifest SHA256 is 9526797b770ede4f89cf33958781135c9f2cd4f54dcdd9f55bda01c136664930. Portable report SHA256 isf4143e88d08a6833b71dd7824de2716c2e74d66c627028ed2d151b219d4d67f6. Full source and executable inventories are retained within that frozen evidence; source membership is not a test pass claim. The previous root phase was separately sealed with 792passes and 1existing ignored, including its actual initial failures and 106root evidence hashes verified by the coordinator.

Trusted client/console/CLI work has since resumed under the separate serial-workspace target policy. Those later source edits and execution results are outside the runtime report and its 201 pass count. The report's runtime-only resource language describes its own completed lane; subsequent coordinator runbook entries govern later commands. Whole-unit review, final schema 4 checks and publication remain outstanding, and the story stays active.

## Complete frozen runtime report

# OAuth stage 2C runtime checkpoint

The bounded runtime checkpoint passes: **201 tests**, zero failures or ignored cases, strict all-target Clippy for all four affected packages, and runtime workspace formatting. This is not completion of the OAuth story. Trusted client/console/CLI implementation, the additional root schema-4 fixture checks, remaining affected gates, independent review and publication remain outstanding.

Managed source: `~/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685`, HEAD `df2855b91e71e3349c44345eb06e9c9447438e41`; stage2C source baseline `87149e7d5181ce5dff1ccdeceefd1e0bd34e9e30`. Source is uncommitted. No Git/AEP/model/provider/operator mutation was performed in this checkpoint. Earlier root/SQLite/kernel/private-refresh reports remain separate and unchanged.

## Behavior and ownership

The new config split admits explicitly selected GitLab public development registrations and the dedicated unsealed FileStore. The actual runtime keeps OAuth bindings out of ordinary credential import and composes one OAuth backend over the same FileStore and FULL SQLite instance used by acquisition, evidence publication, refresh and dispatch. The temporary unused-custody-module expectation is removed. Raw and OAuth Invoke ownership now match the exact Connection; Search/Describe aggregation and the original raw/hosted constructor tests remain intact.

Acquisition selects exactly one configured binding before listener/session/egress, captures its authority and generation, receives real loopback PKCE callbacks or bounded device polling, and obtains observed token-info client/subject/scope evidence. No requested-scope inference or sibling-credential scope union authorizes an operation. The original raw lease, grant and source-input checks run before refresh without consuming approval/event evidence; delegation checks them again before invocation.

The same binding gate covers acquisition, coherent reads, refresh and dispatch. Refresh uses the receiver's original 30-second window; the existing raw lease contains an operation reference and no usable outer deadline. Actual token/token-info requests share that remaining budget. A FULL-confirmed marker precedes refresh egress; write uncertainty sends zero egress, and surviving markers prevent old-token reuse after rotation/token-info/prepare failure and actual reopen. The marker shares the existing journal's SQLite Arc and does not change its serialized Image/version or create another credential store.

Completion uses the existing prepared-store kernel: a live authority/time claim strictly before the deadline, then FULL decision, credential commit and coherent publication. Claimed I/O may finish after the deadline. Uncertain writes remain unavailable until recovery; they do not produce false Completed or Expired. Pending status observes the receiver's liveness at its serialized decision point. Retirement/drop updates that observation before receiver completion, but it does not promise that a live URL remains live after a status response. Shutdown retires and joins owned acquisition tasks. Stopping a v1 client poll still does not send cancellation; daemon expiry/shutdown owns cleanup.

## Actual deciding evidence

All prefixes below are under `~/.cache/connectors-cli-wave-20260906/connect-session-oauth-custody-in-personal-posture/` and have exact argv/cwd, raw logs, exits and continuous resource readings.

| Prefix | Actual result |
| --- | --- |
| `stage2c-runtime-config-red-1` | 1 pass, 2 failures, 20 filtered; unchanged config rejects both explicit PKCE and device positives. Exit 101. |
| `stage2c-runtime-config-green-1` | Complete config package: 26 passes. |
| `stage2c-runtime-composition-red-1` | 2 passes, 1 failure; existing runtime accepts config but reports no OAuth custody owner (`Null` instead of count 1). Exit 101. |
| `stage2c-runtime-transport-red-1` | 1 pass, 2 failures, 20 filtered; observer stays live after callback and drop. Original deadline observation passes. Exit 101. |
| `stage2c-runtime-oauth-tests-1` | Compiler-only failure: extracted `select_binding` visibility. No test/product-red count. |
| `stage2c-runtime-oauth-tests-2` | Compiler-only failures: three test accesses to a private address field and two temporary context borrows. No test/product-red count. |
| `stage2c-runtime-oauth-tests-3` | Integration-catalog 86 + transport 23 + two compile-fail doctests pass. |
| `stage2c-runtime-composition-green-1` | Four actual runtime/mixed-owner fixtures pass. |
| `stage2c-runtime-full-tests-1` | Complete runtime package: 62 passes. |
| `stage2c-runtime-clippy-1`, `-2` | Retained strict-check failures: two needless borrows, then new-test lexical guard/slice-clone issues. No suppressions added. |
| `stage2c-runtime-clippy-3` | Strict all-target Clippy passes for config, transport, integration-catalog and runtime. |
| `stage2c-runtime-affected-tests-final-1` | Final combined full packages: config 26; transport 23 + 2 doctests; integration-catalog 88; runtime 62. **201 passes**. |
| `stage2c-runtime-fmt-1` | Runtime workspace `cargo fmt --all -- --check`, exit 0. |

The observer deciding baseline is an explicitly additive read-only observer seam with the original `retire` method and original absence of Drop retained. It does not claim the old API exposed that observer. The two fixes after the actual run are the atomic retirement store and Drop calling retire; their exact patch is `stage2c-transport-retirement-fix.patch`. Config, runtime and transport preimages are retained separately. The expected compile-fail documentation tests verify that private transport values do not implement Debug; their expected diagnostics are successful tests.

Final new composed fixtures cover real PKCE and device acquisition; optional refresh omission/deletion; observed scope admission and pre-claim authority revocation; unique binding, owner/profile/persistence refusals; exact mixed raw/OAuth routing; stale lease/source-input zero-egress refusals; one serialized refresh for concurrent invokes; original refresh-window expiry and cancellation of its real in-flight egress future; uncertain marker confirmation with zero egress; rotation followed by token-info or actual FileStore prepare refusal; and same-binding reopen refusal of old credentials.

Completion fixtures cover expiry before claim, delay after a timely claim, decision write failure before/after confirmed durability, secret commit before metadata publication, and metadata publication before receipt reclamation. They close/reopen actual SQLite/FileStore owners without an intervening shutdown-recovery call. A cfg(test)-only JournalIo wrapper exposes only closed phase/moment observations, delegates bytes to the existing FULL journal, and requires byte-equal journal readback before its AfterWrite callback. Tests prove absent/present credentials before the error and publication/abort on actual reopen. This is actual committed-record/readback/reopen evidence, **not a power-loss simulation or secret-store digest-readback attestation**. Synthetic provider responses exercise the composed path; no live provider compatibility is claimed.

## Preservation and dependency closure

`stage2c-runtime-preservation.log` records byte-exact original custody/session-refresh tests, ten unchanged rate adversary paths, the existing integration-catalog helper's sole `oauth: None` addition, and the complete original transport test prefix. The separately scoped catalog-invariants exceptions remain disclosed in the root report. New schema-4 tests were expanded after the sealed root run to include all original rate URI and actual provider-loading vectors; these new root-only tests still require targeted execution and are not included in this checkpoint's 201 passes.

All twelve locked/offline metadata graphs pass. Only `crates/connectors-runtime/Cargo.lock`, `crates/connectors-cli/Cargo.lock` and `crates/connectors-console/Cargo.lock` changed: integration-catalog gained the existing local `connector-oauth` and `connect-session-transport` edges; console also gained those two local package rows. All existing package identities/versions/checksums are retained. Nine other locks are byte-unchanged. The integrated GitLab/jsonschema dependency graph was already resolved before this stage and was not newly introduced here. Exact before/refresh/after rows and raw outputs are in `stage2c-runtime-metadata-*`.

The frozen source inventory has 1,185 paths; the patch and copied untracked files preserve the exact checkpoint. It includes other already-owned uncompiled source preparation, including the new CLI parser fixture and additional root tests; inventory membership alone is not a pass claim. Mutable `stage2c-proposed` candidates are deliberately outside the immutable runtime evidence manifest.

## Resources and remaining work

The coordinator's prospective runtime-only note assigned `/dev/shm/b10x-cli-wave-wt-4f1de73d0685-nr00xjox/runtime-target`. Every build used jobs 1, incremental/debug information disabled, original short TMPDIR `~/.cache/cw6/a`, and RUSTC_WRAPPER unset. Continuous guards enforced disk above 12 GiB, tmpfs free at least 8 GiB, MemAvailable at least 16 GiB and target at most 8 GiB. No guard interrupted a command. The final test run's minimum readings were disk 15,231,320,064; tmpfs 28,918,603,776; memory 45,028,806,656 bytes. Its maximum target allocation was 4,478,238,720 bytes. The complete warm inventory has 8,317 files and 156 ELF executable hashes. Both ordinary root/runtime worktree targets remain absent. No other tree or cache was cleaned.

Outside repository writes were persistent `stage2c-*` scratch evidence/candidates, the exact private target above, and transient compiler/test files under the original assigned TMPDIR; Cargo retains its ordinary tool-managed metadata bookkeeping. No target output substitutes for persistent evidence. The warm target is preserved for the coordinator's separately recorded serial-workspace handoff; this report authorizes no workspace transition by itself.

Next: actual client/console/CLI deciding execution and implementation, private instruction/output/headless fixtures, remaining root/client/service and protocol/server regression gates as applicable, final model projection coordination, whole-unit independent review, then coordinator publication. Connection v1 and Operation v1/v2 artifacts remain unchanged; no future Design22 runtime is included. Schema3 publication still precedes any schema4 writer publication.

The portable report changes only the original absolute home-directory prefix to `~`. Raw/portable reports are separate; all original evidence remains intact.
