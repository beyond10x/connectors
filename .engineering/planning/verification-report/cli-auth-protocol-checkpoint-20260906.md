---
format: aep.planning-md/1
id: verification-report:cli-auth-protocol-checkpoint-20260906
kind: verification-report
status: draft
title: Partial authentication protocol and broader validation checkpoint
relations:
- verifies: story:auth-as-tool-result
revision: 1
---
## Coordinator verification and limit

This is a partial implementation checkpoint. The additive protocol slice has 77passing tests, strict Clippy and fixed-point bundles, while its separate broader validation actually reports 328passes and 2failures in unchanged baseline claim-fence inputs. It is not a full root gate, authentication runtime implementation or whole-story independent review. The story stays active. The complete original reports below preserve their own earlier pending/warm-target statements; the later validation and cleanup supplements supersede those observations without rewriting them.

Root verified all 261 members of the original evidence manifest, whose SHA256 isb9387859f9037b04a072cb3dd5ca6a8628fadbab8a6eaa46b51cc7d6fe8749ec. At2026-09-06T15:27:07.767181Z, root additionally verified all 21 broader-validation members and all 12 cleanup members with no mismatch. Their manifest SHA256 values are a065af64c1bb60e963ec23fc3f05f169fe28015a0b697cd8b4947d071b181bce and 66f1d669be8aa1597b255b029ef19dbac8458449af142c2528cf4ba13b77dcbd respectively. The coordinator verification record remains wave-root-runtime-protocol-evidence-verification-20260906.json under ~/.cache/cw6/p.

The protocol source remains frozen and uncommitted in its assigned managed tree. The source-only stage2-runtime-handoff-plan.md is separately frozen at SHA256 9f684c6b1f63abb834a8e29c37ad487b93769906bba49567796b9c0e1fb1ffb3. Its further implementation requires the reviewed OAuth handoff and exact source/model scope; no such implementation is claimed by these reports. The bounded OAuth-owned correction of the old claim fence remains separate until integrated.

## Complete frozen protocol report

Implementor report: additive authentication protocol slice, 2026-09-06.

The assigned protocol implementation is complete in the managed tree, with a clean bounded protocol gate. This is an implementor handoff, not an independent review, whole-story result, runtime wiring result or publication claim. Broader owning-root consumer/inventory validation is pending the coordinator's build-slot assignment. No remaining failure is known in the checks executed on final source; the measured earlier compiler, schema and Clippy failures are retained below.

Authority is story `auth-as-tool-result` revision 44 and Design 22's appended published migration/handoff section, based on clean `ba41c684def774d614cc5f902a7feb0ef17d0fa3`. The managed tree is `~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231`, branch `impl/auth-protocol-remediation-resumed`. Published Atlas proposal `bfb731377a448ab89443c44852d622601f6c1363`, current ADR0041's dated authentication extension and separate proposed ADR0044 supersede the historical ADR0040 pointer in the earlier plan. This report makes no architecture-acceptance claim.

Scratch and complete unabridged logs: `~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation`. All source edits are frozen at this handoff. No Git/AEP/ESS/operator/live mutation, runtime/server/client/CLI/custody/grant/session wiring, dependency addition, manifest change or lock change was performed. Git commands used for inventory, diff and reverse-apply checking were read-only. The warm owned root target remains available; no target/cache cleanup was performed.

The new Operation v3 reader carries strict typed authentication facts, requiring a non-null payload exactly for `authentication_required`, a non-retriable error, `not_attempted`, trusted next action, and no result or delay. All ordinary v2 request/result/rate value types and validation are retained through the frozen internal v2 API. The three-version decoder selects a known identity then deserializes the selected strict DTO from its original bytes. Pure encoders validate before projection. Authentication downgrade to requested v1/v2 replaces the original message and all extension data with exactly `unavailable`, message `The operation is unavailable.`, and `retriable:false`. Ordinary/rate projections match the unchanged predecessor adapter byte-for-byte in their tests. The new optional authentication value is boxed in Rust to keep returned errors bounded in memory; serialization and schemas are unchanged by that representation choice.

Connection v2 retains ordinary payload types and their deployed-v1 validators. Bound start/status/acknowledge commands reject extra target-selection/authority/credential fields. Input is limited to 64 KiB serialized UTF-8; selected v2 request frames to 128 KiB; selected v1 request frames remain 64 KiB; responses remain 128 KiB. Bound requests and results cannot project to unbound v1 creation: conversion returns a typed non-retriable Protocol refusal. Bound status compares duplicate references, integration, state and expiry, and checks a completed session's Connection against the bound target. Pending requires Pending; Ready/Consumed require Completed; Expired permits matching Completed or Expired with no endpoints, preserving publication-before-expiry; Failed requires Failed. Nested sessions retain the frozen v1 trusted URL and terminal-endpoint validation. These values perform no grant, principal, live-time, readiness, one-use acknowledgement or execution admission check. Trusted session DTOs remain unsuitable for direct model/MCP output; those enforcement owners are later work.

The two typed schema owners and independent generators produce exactly five bundle files per new identity. New JSON inventory entries are strictly additive. The existing internal `pub use wire::*`, all prior bundle assertions and all 85 predecessor operation vectors remain intact. An additive regression pins 21 predecessor artifact/reader hashes; a separate baseline inventory proves all 79 predecessor, manifest and lock files unchanged. Existing bundle tests remain an exact byte prefix. No public source imports OAuth kernel/schema4 source.

Validation counts and limits:

| Observation | Passing | Failing | Ignored | Exit |
| --- | ---: | ---: | ---: | ---: |
| Untouched protocol baseline: 37 unit + 25 bundle + 2 consumer executions | 64 | 0 | 0 | 0 |
| Prepared tests before modules: no test execution | 0 | compiler only | 0 | 101 |
| First executable new-case run | 11 | 1 | 0 | 101 |
| Same new-case run after schema-owner repair | 12 | 0 | 0 | 0 |
| Full protocol after generation: 37 unit + 38 bundle + 2 consumer executions | 77 | 0 | 0 | 0 |
| Final full protocol after boxed payload: same configuration/count | 77 | 0 | 0 | 0 |
| Final strict protocol Clippy, all targets | n/a | 0 | n/a | 0 |
| Root `cargo fmt --all --check` | n/a | 0 | n/a | 0 |
| Repository Markdown link check | n/a | 0 | n/a | 0 |
| Story index and 73 records | n/a | 0 | n/a | 0 |

These repeated executions are separate observations, not a unique summed test total. The first executable run selected 12 additive test groups and filtered 26 tests, including the new manifest test before artifacts were generated. The final unfiltered protocol runs executed all 38 bundle tests, including the new manifest test. No ignored or suppressed assertion was introduced. Thirteen additive test groups cover independent schema/reader vectors, inherited constraints, exact auth downgrade, ordinary/rate adapters, bound-v1 refusal, original-byte duplicate fields at envelope/nested boundaries, all selected/unknown/mismatched versions, version-specific byte budgets, variant coverage and predecessor byte identity.

Operation v3 has 133 reviewed-shape vectors: 33 reader-valid and 35 schema-valid. All 133 actual reader and Draft 2020-12 outcomes match their separate declarations, including the unchanged 85-vector v2 family with only its supported identity adjusted. Connection v2 has 151 vectors: 47 reader-valid and 61 schema-valid; all actual outcomes match their declarations. Both generators use the typed reader from serialized original vector bytes and an independent Draft 2020-12 validator with format assertions enabled. Both vector documents also passed their own Draft 2020-12 vector schemas with unique case names.

Projection limits are explicit rather than inferred from a green vector count. Operation retains v2's exact RFC URI schema/reader grammar and all inherited schema definitions except the versioned envelopes and extended error; exact rate-interval arithmetic and serialized UTF-8 budgets remain reader checks. Connection preserves the deployed-v1 WHATWG URL parser and its local/hosted route/fragment-capability checks; schema enforces its string bounds and lifecycle/endpoint structure without pretending ordinary URI-format checks implement WHATWG normalization. Instance-value equality (including duplicated target/session/expiry and mediated-route self-reference refusal), UTF-8 byte rather than code-point bounds, and serialized budgets remain reader obligations. Equality and inherited URL grammar differences have named schema-valid/reader-invalid vectors. The schema does not establish real-time authority or readiness.

Both new bundles were generated twice, with all ten file hashes unchanged between writes, and both check modes pass. Final checks after the boxed Rust payload still match those same generated bytes. The frozen v2 generator's 85-vector check passes. `bundle-fixedpoint.json` records the ten file hashes and the byte-equality result. Bundle manifests include README, protocol schema, vector schema, vectors and the unchanged artifact-bundle schema. The frozen v1 schema/deployed-reader distinctions were preserved rather than repaired.

The test-first and failure record is retained:

- `deciding-cases-before-implementation.json` and prepared case/test bytes were saved before adding reader modules. The untouched baseline temporarily restored only the saved original bundle test file, then the prepared file was restored. `protocol-before.log` is the actual baseline and `prepared-tests-first.log` contains 13 unresolved-import/module compiler diagnostics (E0432/E0433). This compiler-only absence observation is not counted as a product test failure.
- `reader-schema-first.log` is the actual deciding product red: 11 passed, 1 failed. The Connection schema owner used mutable indexing while inspecting a `$ref` result branch, inserting a null `properties` member. Draft 2020-12 meta-schema compilation refused at `/$defs/ConnectionResult/oneOf/1/properties/value/properties`. The initial source bytes are retained in `first-executable-source/`, `first-executable-tracked.patch` and `source-reader-schema-first.json`. The repair uses non-inserting `get_mut` traversal. No vector expectation or existing assertion changed. `reader-schema-after-projection-fix.log` retains 12 passed, zero failed.
- `protocol-clippy.log` records the first strict Clippy exit101: 11 `result_large_err` diagnostics for the 152-byte new error value. Boxing only the optional authentication payload resolved them. `protocol-clippy-after-box.log` records strict exit0, followed by a complete post-change protocol rerun and both schema/bundle checks. Wire/schema bytes stayed exact.
- One JavaScript orchestration call had a syntax error before invoking any command; it launched no process and changed no source. It was corrected before the second-generation/check sequence. There were no measured resource interruptions or fixture retries in this stage.

Every build used the owned root target, one job, `/usr/bin/sccache`, incremental0 and debug0, `CARGO_TARGET_DIR` unset, TMPDIR `~/.cache/cw6/r`, and a PATH selecting `~/.cargo/bin`. `run_lane.py` records exact argv, cwd, environment overrides, start/end, exit and a one-second free-space timeline per command. It refuses to launch below 12,884,901,888 bytes and terminates only its own process group if the reserve is crossed. The measured minimum across the twenty recorded validation commands was 14,669,938,688 bytes, 1,785,036,800 above that reserve. No command was resource-interrupted. The target measured 372,975,727 bytes after baseline and 487,660,893 bytes at the clean bounded boundary. The compile slot was explicitly returned to the coordinator before Website's gate; no Cargo command has run after that release. Subsequent link/story checks were noncompiling.

Exact validation commands and exits; complete output is in each corresponding `<label>.log`, `<label>.command.json`, `<label>.exit`, `<label>.result.json` and `<label>.resources.jsonl`:

| Label | Exit | Actual argv |
| --- | ---: | --- |
| `protocol-before` | `0` | `cargo test --locked -p protocol --no-fail-fast` |
| `prepared-tests-first` | `101` | `cargo test --locked -p protocol --test bundles authentication_ --no-fail-fast` |
| `reader-schema-first` | `101` | `cargo test --locked -p protocol --test bundles authentication_ -- --skip authentication_bundle_manifests_match_all_published_bytes` |
| `reader-schema-after-projection-fix` | `0` | `cargo test --locked -p protocol --test bundles authentication_ -- --skip authentication_bundle_manifests_match_all_published_bytes` |
| `operation-v3-write-1` | `0` | `cargo run --locked -p protocol --example operation_v3_bundle -- write` |
| `connection-v2-write-1` | `0` | `cargo run --locked -p protocol --example connection_v2_bundle -- write` |
| `operation-v3-write-2` | `0` | `cargo run --locked -p protocol --example operation_v3_bundle -- write` |
| `connection-v2-write-2` | `0` | `cargo run --locked -p protocol --example connection_v2_bundle -- write` |
| `operation-v3-check` | `0` | `cargo run --locked -p protocol --example operation_v3_bundle -- check` |
| `connection-v2-check` | `0` | `cargo run --locked -p protocol --example connection_v2_bundle -- check` |
| `operation-v2-frozen-check` | `0` | `cargo run --locked -p protocol --example operation_v2_bundle -- check` |
| `protocol-after` | `0` | `cargo test --locked -p protocol --no-fail-fast` |
| `protocol-clippy` | `101` | `cargo clippy --locked -p protocol --all-targets -- -D warnings` |
| `protocol-clippy-after-box` | `0` | `cargo clippy --locked -p protocol --all-targets -- -D warnings` |
| `protocol-final` | `0` | `cargo test --locked -p protocol --no-fail-fast` |
| `format-final` | `0` | `cargo fmt --all --check` |
| `operation-v3-final-check` | `0` | `cargo run --locked -p protocol --example operation_v3_bundle -- check` |
| `connection-v2-final-check` | `0` | `cargo run --locked -p protocol --example connection_v2_bundle -- check` |
| `links-stage1` | `0` | `python3 scripts/check-links.py` |
| `stories-stage1` | `0` | `python3 scripts/check-stories.py` |

The minimal proposed broader owning-root validation set is `catalog-build` (closed JSON inventory, architecture and dependency fences) plus `connectors-client`, `service` and `server` (the three direct root-workspace production consumers of protocol). Proposed serial commands, NOT YET RUN: `cargo test --locked -p catalog-build -p connectors-client -p service -p server --no-fail-fast`, followed by `cargo clippy --locked -p catalog-build -p connectors-client -p service -p server --all-targets -- -D warnings`. Protocol itself has the complete green checks above. Root catalog/ESS assembly closure, full twelve-workspace validation, OAuth/runtime integration, live providers and independent whole-unit review remain outside this bounded result. No broader green claim is made.

There are 1,194 source inventory entries, 22 exact assigned changed paths (5 existing and 17 new), and no out-of-scope source change. `source.patch` includes all tracked edits and all new file hunks and passes read-only reverse `git apply --check` against the exact working source. `new-files.tar` contains exactly the 17 new files with deterministic metadata; it complements rather than replaces the full patch. `json-schemas-additive.delta.toml` contains exactly four schema and four document registrations; the coordinator must combine it with OAuth's separate schema4 inventory addition while preserving all previous entries.

| Handoff artifact | SHA256 |
| --- | --- |
| `source.patch` | `554d75023931a3b7fffe495510dc0d43cf7ed9983d5c8e6714eb163e18b63cda` |
| `new-files.tar` | `f0f73fd362cd77b4b4e8c659a86cb5375cfc5a66e8628c6b7ebb562371c47ba0` |
| `source-final.json` | `e392ee93b9f011c86c1175b772ed22cff12203b3c9e3c9984202d8df7c699eb5` |
| `commands.json` | `bec3e95a08d5c8b84ea4c1f6f7e8f1a6b44d54121fb39ff8279fac3bfed9834b` |
| `json-schemas-additive.delta.toml` | `f61e83a726026a8ad52833525294ddf372e2d18af3f5af1defa55af1f2da0856` |
| `bundle-fixedpoint.json` | `c1bbea063939ea0e58210b09b66400f2e06ebb2ccfa948033eea6000fcf77ff3` |

Exact changed source inventory:

| Repository path | Bytes | SHA256 |
| --- | ---: | --- |
| `contracts/connector-connection/v0alpha2/README.md` | 4087 | `b093d8b91f306cd9a67df14c0c12ca7956b2f28d678b22141f1e567227732e98` |
| `contracts/connector-connection/v0alpha2/bundle.json` | 1057 | `a40ce8d4794fa87a73538a9e9b5c83959c7b72956c501315771b4682cafe4439` |
| `contracts/connector-connection/v0alpha2/connector-connection.schema.json` | 47531 | `8790a784eb0fadc63a726fd139e7113f09d593bbdfc5b00cd31f5774891f5909` |
| `contracts/connector-connection/v0alpha2/vectors.json` | 331963 | `fc9e172d349c7ba489f5e1147d9ec0f2863cb6d86a49a6fba8842dc941683778` |
| `contracts/connector-connection/v0alpha2/vectors.schema.json` | 1264 | `8306f43d80f22c15215f78cf33408811bdef3359bec6b543cffd185deecf552f` |
| `contracts/connector-operation/v0alpha3/README.md` | 3672 | `01b02f20fb66f5e51861c68ce8383307cb3280ef69aff00a11a5966fd8e34ae8` |
| `contracts/connector-operation/v0alpha3/bundle.json` | 1051 | `69a8fd828cda5453c72e58e970ef99069d1b0bdc96ba5c1c5fc3d00e9b671a10` |
| `contracts/connector-operation/v0alpha3/connector-operation.schema.json` | 31421 | `34df24fc1f1c97122c95dedc9c11a2bce16c66d02cc98638f1288cc38dafb43c` |
| `contracts/connector-operation/v0alpha3/vectors.json` | 214640 | `2c7a7f9dde35a314dca27fde7c32b52fe1004fdf53df36f2a38ec96cc88d0445` |
| `contracts/connector-operation/v0alpha3/vectors.schema.json` | 1261 | `fc3371719bedf99dfbacda330a9c9d598a5980ac984acd0ecdc8d514d8bebcd6` |
| `crates/protocol/examples/connection_v2_bundle.rs` | 3873 | `ec3e68140a498721688f008ba193c4d2656cf643605458b506714adfafacb34a` |
| `crates/protocol/examples/operation_v3_bundle.rs` | 3868 | `2683b00661433490b3774dff6d2b38211bb2ecbcdc242b1e8d9605d1d886baf8` |
| `crates/protocol/src/connection_v2.rs` | 20292 | `2c66d1600b0559feec1a3d866ff1f9d54c276e451532ffa797093df3b740805d` |
| `crates/protocol/src/connection_v2_schema.rs` | 11850 | `956d64541dc269f75d8510e5a7ab5c97fcf4dd99aca8cca55375755bbbca5fe1` |
| `crates/protocol/src/operation/schema_v3.rs` | 2602 | `d612cb8b44d93787e0d6e3fb8e5cd851f437407325dbe0e900bba63dd5c467d2` |
| `crates/protocol/src/operation/v3.rs` | 11465 | `f37631475ca8d1736ee0104f287d0c6f2146962125a0fdf6156d94adb03dce14` |
| `crates/protocol/src/operation/versions.rs` | 4378 | `c585370e6eb17f74a7dc551705ad2229cc47bd708dd83cb1a92af90a12178c19` |
| `crates/protocol/README.md` | 6155 | `b9f8e58f51db8cd808d4bed221e879b3dda872ff7aabcf8068a09a0dd7a7c792` |
| `crates/protocol/src/lib.rs` | 351 | `8294b85a3bae14908c8cefdace4c6c913aba86b6e26c8114a6ad213f34032fe0` |
| `crates/protocol/src/operation.rs` | 334 | `fb68106033a0477393e6d626866b3fdc268042f5f2446caf0e38d5a5456ae9fe` |
| `crates/protocol/tests/bundles.rs` | 48777 | `3f1ea9af3e55fcee77c42c0b44ecab7f54e6bce310c124b83a5f50b70c3cfc4e` |
| `json-schemas.toml` | 9127 | `53dd6f71fbc36a262cd64180a0af42ef1d043c2a9a1cd00af7dcea7284a15bc4` |

The raw report is retained unchanged. The portable report replaces only the original home-directory prefix with `~`; it does not alter source, test output, vector expectations, command flags or evidence files. Paths in public prose are plain code-form citations; no machine-local Markdown links are introduced. The final evidence manifest is created after these reports and excludes only itself and its own verification-result file to avoid a circular hash.


## Subsequent broader validation report

Portable copy: the original absolute home-directory prefix is replaced with ~; all other report text is unchanged.

Bounded authentication protocol broader validation, 2026-09-06.

Outcome: the approved four-package test invocation reports 328 passed, 2 failed and 0 ignored, exit 101. Both failures are the pre-existing catalog-build hosted Failed claim-fence fixtures. Strict all-target Clippy for the same packages passes, exit 0. This is a bounded compatibility/inventory observation, not a full-workspace green result or an authentication runtime review. No source, test, dependency, lock, generated artifact, AEP, model, Git or provider mutation was performed.

Source: `~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231`, HEAD `ba41c684def774d614cc5f902a7feb0ef17d0fa3`, with the previously frozen additive protocol patch. All 1,194 entries in the original `source-final.json` are unchanged. Working status is exactly the pre-lane status. All 261 entries in the original `evidence.sha256` remain exact; that report and manifest were not rewritten. The separate runtime plan and its 43-entry snapshot manifest also remain exact.

The command cwd was the source root. Commands were executed by the previously frozen `run_lane.py`, with `CARGO_TARGET_DIR` unset; `TMPDIR=~/.cache/cw6/r`; `RUSTC_WRAPPER=/usr/bin/sccache`; `CARGO_INCREMENTAL=0`; `CARGO_PROFILE_DEV_DEBUG=0`; `CARGO_PROFILE_TEST_DEBUG=0`; `CARGO_BUILD_JOBS=1`; and `~/.cargo/bin` prepended to PATH. Exact argv, cwd, environment overrides, start/end times and statuses are in the two command/result JSON pairs; raw stdout/stderr was retained without truncation.

```text
cargo test --locked -p catalog-build -p connectors-client -p service -p server --no-fail-fast
cargo clippy --locked -p catalog-build -p connectors-client -p service -p server --all-targets -- -D warnings
```

| Executed test target | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| catalog-build unit tests | 63 | 0 | 0 |
| catalog-build tests/main.rs | 80 | 2 | 0 |
| connectors-client unit tests | 18 | 0 | 0 |
| server unit tests | 101 | 0 | 0 |
| server tests/rate_adversary_local.rs | 2 | 0 | 0 |
| service unit tests | 64 | 0 | 0 |
| Four package doc-test targets | 0 | 0 | 0 |

The test invocation continued through all packages and doc-test targets because --no-fail-fast was used. No target was omitted, and no test assertion was changed. The catalog JSON governance test `json_governance::every_repository_json_is_classified_and_valid` passed, including the new additive inventory. The pre-existing stage1 protocol count of 77 passed is separate evidence and is not added to this invocation's total.

The retained failures are:

- `ess_claim_fence::the_hosted_failed_claim_is_refused_from_either_side`, at `crates/catalog-build/tests/main/ess_claim_fence.rs:709`: `assertion left != right failed: the deletion has to change the text`.
- `ess_claim_fence::the_hosted_registry_never_reaches_the_state_its_marker_says_it_cannot`, at the same file line 689: the expected old prose is absent, and `crates/service/src/connect_session.rs:233` no longer assigns Failed.

`baseline-claim-provenance.json` proves that the executed fixture, test entry point, Connection ESS model, service lifecycle source and the complete scanned integration-catalog Rust path set are byte-identical to the starting commit: 10 compared files, no mismatch. Four baseline blobs are preserved in `baseline-claim-inputs/`. The model lacks the exact deletion substring; line 233 does not assign Failed, while the current assignment is at line 396; the old integration-catalog scan finds no Failed writer. Thus both failing predicates and all their filesystem inputs already existed at the baseline. This is an executed current test plus exact baseline source/input equality proof; no separate baseline Cargo execution is claimed. OAuth's schema4 fixture changes were neither imported nor used to waive these failures. Coordinator correction/integration remains outstanding before a whole-unit green claim.

The one-second resource monitor enforced the 12 GiB reserve (12,884,901,888 bytes). Test minimum free space was 14,266,064,896 bytes; Clippy minimum was 13,846,798,336 bytes. The smallest measured margin was 961,896,448 bytes. Neither command was interrupted. The existing root target grew from 487,660,893 to 1,539,751,352 bytes (+1,052,090,459); it remains warm and was not cleaned. Source-after observation recorded 13,845,458,944 free bytes and no owned Cargo, rustc, clippy-driver or rustdoc process. The compile slot was explicitly returned to the coordinator.

Evidence is under `~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/broader-validation`: `tests.log`, `clippy.log`, command/result/exit files, both complete resource timelines, before/after source proofs, baseline provenance and retained baseline inputs. `commands.json`, `counts.json` and `evidence.sha256` provide the supplement index. No reports link into machine-local scratch.

The separate source-only runtime handoff remains `~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/stage2-runtime-handoff-plan.md`, SHA256 `9f684c6b1f63abb834a8e29c37ad487b93769906bba49567796b9c0e1fb1ffb3`. It scopes exact owners and tests but does not authorize implementation. Runtime work still requires the coordinator's reviewed OAuth handoff and exact scope.


## Subsequent completed target cleanup report

Portable copy: the original absolute home-directory prefix is replaced with ~; all other report text is unchanged.

Completed owned target cleanup, 2026-09-06.

The coordinator-authorized `cargo clean` completed with exit 0 in `~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231`. Exact stdout: `Removed 4908 files, 1.5GiB total`. This tree's root `target/` is now absent. No other target, shared cache, repository source or evidence was removed. This cleanup supersedes the warm-target status recorded at the earlier validation boundary; the earlier report remains unchanged.

Before cleanup, `target-inventory.json` recorded 4,908 entries, including SHA256 for every regular file and exact symlink destinations where applicable; `target-executables.json` records the 113 executable files and their hashes. `target-freeze.sha256` pinned both inventories and `before.json` before running Cargo. The target's measured du size was 1,539,751,352 bytes; summed regular file sizes count hard-linked paths separately. No process executing from this target and no owned Cargo/rustc/clippy-driver/rustdoc process was present.

The exact command was `cargo clean`, with cwd at the owned root and `CARGO_TARGET_DIR` unset. The unchanged frozen runner retained its jobs1/debug0/incremental0/sccache environment and one-second 12 GiB monitor. It performed no compilation. `clean.command.json` records argv/cwd/environment; `clean.log`, `clean.exit`, `clean.result.json` and `clean.resources.jsonl` preserve the result. There was no reserve interruption.

Post-clean verification retained the exact source HEAD `ba41c684def774d614cc5f902a7feb0ef17d0fa3`, identical working status, all 1,194 source hashes, all 261 original frozen stage1 evidence entries, all 43 runtime-plan evidence entries, all 21 broader-validation evidence entries and all three pre-clean target-freeze entries. No mismatch was found. The additive protocol patch and every original frozen report/manifest remain unchanged.

Free space was 13,790,867,456 bytes at the inventory boundary and 15,111,540,736 bytes after cleanup: an observed host-wide increase of 1,320,673,280 bytes. The command monitor's minimum was 13,785,616,384 bytes. The observed increase is not an isolated filesystem accounting claim; other sessions share this disk.

All cleanup evidence is under `~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage1-protocol-implementation/broader-cleanup`. Source/evidence checks are in `before.json` and `after.json`; the complete cleanup supplement is pinned by its separate `evidence.sha256`. The compilation slot had already been released before this cleanup. No further Cargo execution is scheduled by this agent.
