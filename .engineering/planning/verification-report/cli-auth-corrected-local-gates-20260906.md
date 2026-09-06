---
format: aep.planning-md/1
id: verification-report:cli-auth-corrected-local-gates-20260906
kind: verification-report
status: draft
title: Authentication corrected local gates before whole-unit review
relations:
- verifies: story:auth-as-tool-result
revision: 1
---
## Publication source equivalence — 2026-09-07

The publication checkout carries all 1046 nonplanning source files from the frozen reviewed candidate. Every file is byte-identical except the two explicitly scoped public architecture pages, which now describe the actual default protocol and bounded personal OAuth workflow. The original implementation and full test evidence remain preserved locally. The published planning journal retains its existing prefix, and all new records are created through AEP. This is a source-preserving publication assembly, not a replay of local machine-specific execution records or a rewrite of any published ref.

The only additional source change after this comparison is the scoped Unreleased changelog, whose auth review marker remains pending. The runtime and client reports below reproduce the frozen reports with the local home-directory prefix replaced by ~. Raw originals and their sealed hashes remain preserved. This is presentation normalization only; no command result, test assertion, count or first failure changes.

## Runtime report

unit: story:auth-as-tool-result — A affected common-source gate report
verdict: blocked
cases: assigned affected gates pass; original root/runtime failures and all corrections retained
origin: full-root transport refusal, Clippy and optional local-remediation regressions corrected
wrote-outside-worktree: assigned scratch, private Cargo target and assigned TMPDIR only
needs-coordinator: yes — final CLI/docs assembly, full-wave CI and independent whole-auth review

All assigned A affected gates are complete. The remaining block is coordinator-owned final assembly/full-wave verification and independent whole-auth review, not a remaining observed A product failure. This report does not claim twelve-workspace/thirteen-configuration completion or external adoption. The complete OAuth review, frozen protocol/service slice and earlier A/B evidence retain their separate scope.

Source and corrections

The initial full-root run used clean common99328258395255dd10038104c1487a64fd5abd8f, with all1228 tracked source hashes independently matched. It ran --no-fail-fast and recorded1301passes/2failures/4existing ignores, exit101. The existing frozen-version refusal case exposed an empty400 response for unsupported identity; restoring only the existing typed-v2 refusal envelope selection left strict original-byte rejection before authority/dispatch unchanged. Its deciding case then passed and all115server cases passed. The other root failure expected5 offline fixture artifacts instead of actual6; root alone corrected that expectation, retaining the exact artifact-presence oracles. Full catalog-cli then passed14, including actual namespace execution with --nocapture and no skip message. First strict Clippy exposed one needless borrow in A's connection route; removing it yielded full-root strict success. Initial root fmt exposed only the three root-owned docs formatting hunks; root applied them, and formatting/full-root strict then passed on common289da8a0bc9e1b6eb85fa302024d6d173734eb1e. These first outputs and the45-member boundary seal remain unchanged. Root explicitly required affected full rechecks rather than repeating the unchanged complete root suite.

The first full runtime-default run on289da8a0 recorded445passes/8failures/2existing PostgreSQL ignores, exit101. Optional remediation metadata routing prematurely refused raw catalog backends which do not claim the remediation interface. The recorded correction first checks exact owns_remediation(Target), preserving ordinary dispatch for no claimed route and retaining all actual Refused/Unavailable/ambiguous-owner refusals. Bound Connection Start stays strict and unchanged. Only the new direct AuthenticationBackend fixture gained its explicit route predicate. All eight original assertions remained unchanged and passed in the full recheck.

The same bounded correction added only in-process OneShotOperationV3Outcome::{Reply,RequiresDaemon}, LocalOneShot::operation_v3_outcome and PersonalRuntime::one_shot_operation_v3_outcome, with a registration-only runtime re-export. The existing envelope APIs delegate without wire changes. Only validated persistent control before composition or unsupported ephemeral invocation after composition can create RequiresDaemon; arbitrary backend Unavailable/message text cannot. Two parameterized tests first produced E0432/E0599 missing-API diagnostics before implementation (no executed test and no product verdict), then passed on their first actual execution. They cover all persistent classes before missing configuration, malformed persistent input, backend daemon/private text, exact envelope preservation, dispatch counters, no socket, and state-lock retention through joined async shutdown. The original22785-byte runtime one-shot test body remains byte-identical. Full server115/default runtime455 and both affected strict/format gates passed afterward. These four paths were committed by root at4403f01796b50cfd16eb40431fcf01c79ec98407 and merged4dbe9065ec7cf9e5c100784c1b040b879826a84d; the immutable146-member origin boundary report/seal retains exact source/target details.

Final assigned commands used clean4dbe9065, frozen throughout. Runtime no-default full passed455 with the same2existing PostgreSQL ignores, strict all-target no-default Clippy passed, and exact scripts/gate.sh --final passed. No source changes were made during this final assignment. All1228source hashes and clean HEAD were reverified after completion. B's then-uncommitted three CLI-only files were not upstream inputs to these runtime/catalog lanes; B's complete CLI evidence is coordinator-owned and is not folded into this report.

Exact command record

Each row has full raw stdout/stderr, command/environment JSON, exit, process group and one-second resource timeline in its named files. Counts are passing executions per command, not a unique total across repeated or feature-specific tests.

| Label | Actual command | Exit | Observation |
| --- | --- | --- | --- |
| root-tests-first | root: `cargo test --workspace --locked --offline --no-fail-fast` | 101 | 1301 passed / 2 failed / 4 ignored |
| operation-version-refusal-after | root: `cargo test --locked --offline -p server rate_adversary_hosted_grant_and_approval_refusals_keep_requested_version --no-fail-fast` | 0 | 1 passed / 0 failed / 0 ignored |
| server-tests-after | root: `cargo test --locked --offline -p server --no-fail-fast` | 0 | 115 passed / 0 failed / 0 ignored |
| root-clippy-first | root: `cargo clippy --workspace --locked --offline --all-targets -- -D warnings` | 101 | retained check failure; correction described above |
| root-clippy-after | root: `cargo clippy --workspace --locked --offline --all-targets -- -D warnings` | 0 | check passed |
| root-fmt-first | root: `cargo fmt --all --check` | 1 | retained check failure; correction described above |
| catalog-cli-after | root: `cargo test --locked --offline -p catalog-cli --no-fail-fast -- --nocapture` | 0 | 14 passed / 0 failed / 0 ignored |
| root-fmt-after | root: `cargo fmt --all --check` | 0 | check passed |
| root-clippy-final | root: `cargo clippy --workspace --locked --offline --all-targets -- -D warnings` | 0 | check passed |
| runtime-default-first | runtime: `cargo test --workspace --locked --offline --no-fail-fast` | 101 | 445 passed / 8 failed / 2 ignored |
| origin-api-first | runtime: `cargo test --locked --offline -p connectors-runtime --test one_shot_runtime auth_one_shot_origin_ --no-fail-fast` | 101 | missing API; no test executed |
| origin-api-after | runtime: `cargo test --locked --offline -p connectors-runtime --test one_shot_runtime auth_one_shot_origin_ --no-fail-fast` | 0 | 2 passed / 0 failed / 0 ignored |
| server-tests-origin-after | root: `cargo test --locked --offline -p server --no-fail-fast` | 0 | 115 passed / 0 failed / 0 ignored |
| runtime-default-after | runtime: `cargo test --workspace --locked --offline --no-fail-fast` | 0 | 455 passed / 0 failed / 2 ignored |
| server-clippy-origin | root: `cargo clippy --locked --offline -p server --all-targets -- -D warnings` | 0 | check passed |
| runtime-clippy-origin | runtime: `cargo clippy --workspace --locked --offline --all-targets -- -D warnings` | 0 | check passed |
| root-fmt-origin | root: `cargo fmt --all --check` | 0 | check passed |
| runtime-fmt-origin | runtime: `cargo fmt --all --check` | 0 | check passed |
| runtime-no-default | runtime: `cargo test --workspace --locked --offline --no-default-features --no-fail-fast` | 0 | 455 passed / 0 failed / 2 ignored |
| runtime-clippy-no-default | runtime: `cargo clippy --workspace --locked --offline --no-default-features --all-targets -- -D warnings` | 0 | check passed |
| gate-final | root: `bash scripts/gate.sh --final` | 0 | check passed |

Ignored cases remain explicit. The root suite retains four existing ignores: pack::measure_read_costs (measurement rather than assertion), file::tests::a_foreign_owned_directory_is_refused_without_repair and file::tests::a_foreign_owned_store_is_refused_without_repair (require euid 0; CI invokes them explicitly with sudo), and keyring::tests::a_credential_round_trips_through_the_real_keyring (requires Secret Service and secret-tool). Each runtime configuration instead retains port_tests::the_postgres_backend_conforms and port_tests::the_postgres_backend_serves_grant_evaluation (require CONNECTORS_DATABASE_URL). The live-Vault build warning is separate from these counted ignored cases; no live Vault, database or keyring was supplied. No extra retry or source weakening was used to manufacture stable green.

The exact final gate output was:

```text
65 providers, 70 artifacts verified
markdown links are repository-portable
story index and 73 records are consistent
gate: [ess] specify validate --path ess/system
connectors v1 — 10 file(s), valid
gate: [ess] the committed clap tree is what the specification generates
gate: final checks green
```

PATH explicitly selected ~/.cargo/bin/ess version0.18.0, SHA25611fef16deb8352eb917c635e6a9029f2f6d555f8ce6702571f61e0c4fcb707a9. Exact gate-script SHA256dec01707adf0915adbe961ab2293231ac74d2f271f90605e1b528c361b0f23b7 is recorded with its action inventory. The script used the existing catalog verifier and its own temporary ESS generation/diff; no committed generated artifact was modified.

Resource and preservation evidence

Every compiling lane used the assigned private /dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target, one Cargo job, incremental0, dev/test debug0 and RUSTC_WRAPPER unset. Locked/offline flags were retained; the exact authoritative final script was not replaced. TMPDIR originally used ~/.cache/cw6/p/auth-runtime-tmp; before the catalog correction/no-default sequence the coordinator prospectively shortened it to ~/.cache/cw6/ar to avoid measured Unix-path limits in B. Old scratch/TMP/targets stayed untouched. Disk12GiB, tmpfs8GiB, MemAvailable16GiB and private-target12GiB guards were sampled each second. No interruption or cleanup occurred. All owned process groups were independently verified empty before both explicit compiler releases.

Across these21 actual commands, minimum disk free was14613086208bytes; minimum tmpfs free16194793472; minimum MemAvailable34897268736; maximum owned target8399376384. Final target inventory seals14458files and334executables. Exact per-lane minima/timelines remain in full-resource-summary.json and the raw files.

The365-member original complete checkpoint seal and146-member origin boundary seal were reverified with zero mismatches before the final lanes. No old report, command output, patch or manifest was rewritten. Final source inventory is no-default-source-before.json, with final-source-and-release.json proving all1228hashes and clean exact4dbe9065 after the commands. Source remained unchanged during this verification; earlier patches are recorded in their immutable boundaries and are already coordinator-committed.

The compiler slot is released and A is paused for coordinator assembly/review. No independent whole-auth verdict, deployment, live provider action, remote mutation, planning edit, dependency/lock change or cleanup occurred in this final verification. The portable report changes only the original home-directory prefix to ~. All evidence is under ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/stage2-runtime-implementation/common-full-gates; there are no machine-local Markdown links.

`no-default-source-before.json` SHA256 `7b2f18626f7e7f96226b17ce0379d48087314d9f226db0b8a5f41cd2ec294a30`.

`final-source-and-release.json` SHA256 `d2039dddc3dc7c76d428c48a3764edebe38766e024afc6dfbd546e901d8d0680`.

`full-command-summary.json` SHA256 `6e1c952b529436e3479e0451ace8f30df370bf7621054e58cc43408b72e36f55`.

`full-resource-summary.json` SHA256 `e689a9cbf14e50821bb81222a32ddce2b56c0612553f32208d17776bf67920b7`.

`full-target.json` SHA256 `86c22861b8b60b4fecae0cfc27d1030f2dad2fafc1ceb7deeb24025c744708e3`.

`full-executables.json` SHA256 `3f6a190f6cbb3c1c1abb3472fd968e5a447a3a22ed96ce20d63651d129c1b845`.

`no-default-prior-evidence-proof.json` SHA256 `44b7596d1dbc0c391b356922c687d4ecbf21b8787effedf5306c1caebd886c12`.


## Client report

unit: auth-as-tool-result / owner B / affected full gates and final correction
verdict: assigned console and corrected CLI full tests, strict Clippy and fmt green
cases: console 107; final CLI 140; per-command reruns below are not additional unique cases
origin: n/a (no provider import or live integration in this correction)
wrote-outside-worktree: yes — assigned evidence/TMP/owned targets and normal compiler caches
needs-coordinator: verify/commit three-path delta, merge common source, complete whole-unit gates/review

The assigned B work is complete and source-frozen. Console full tests passed 107 with strict all-target Clippy/fmt 0 at corrected common 289da8a0bc9e1b6eb85fa302024d6d173734eb1e. That unchanged console lane was retained as directed. On actual A counterpart 4dbe9065ec7cf9e5c100784c1b040b879826a84d plus the exact three-file B correction, final CLI full tests passed 140 with no failures or ignored cases; strict all-target Clippy and fmt each passed on first execution. The reusable client's earlier 46-pass package and strict pass remain separate predecessor evidence, not duplicate assembled results claimed here.

The final production change is confined to CLI lib.rs (1410 lines). Only its v3 direct one-shot branch calls the actual PersonalRuntime::one_shot_operation_v3_outcome API. Reply keeps the existing closed v3 reducer. RequiresDaemon constructs a static unavailable/retriable:false instruction to run connectors serve local with the same config/state root. No received error message selects or supplies this hint. The runtime/server counterpart owns the only two constructors: validated persistent session controls before composition, and actual ephemeral-support refusal after composition but before invocation, retaining shutdown. There is no wire discriminator, protocol/schema change, duplicate CLI validator or new dependency.

Test preservation and correction:

- tests/closed_pipe.rs adds exactly three lines to search_output selecting explicit v2 only for the retained operation controls. Removing those exact lines restores the whole prior file byte-for-byte; all old assertions and encoded response bytes remain unchanged.
- tests/remediation.rs adds one actual default-v3 open/closed-output control, covering four formats, strict received identity, one exchange, nonzero refusal and no synthetic private marker. Its first execution passed.
- The two retained one-shot guidance failures now pass with their original assertions, including canonical browser references and aliases and refusal before invocation effects. No change was made to their source during this correction.
- The two doctor failures were caused by the assigned long temporary path: the actual Connect Session endpoint was 114 bytes while bind(2) permits 107. Subsequent commands use the prospectively approved private ~/.cache/cw6/bc, created 0700; the production path refusal and old tests stay intact. The prior ~/.cache/cw6/p/auth-client-tmp remains untouched, including prior interruption evidence.

Actual command chronology (prefix auth-client- under this directory):

| Suffix | Exit | Executed result |
| --- | --- | --- |
| common-full-console-tests-first | 0 | 107 pass / 0 fail / 0 ignored |
| common-full-console-strict-first | 0 | strict all-target Clippy |
| common-full-console-fmt-first | 0 | owning-workspace fmt check |
| common-full-cli-tests-first | 101 | 134 pass / 5 fail / 0 ignored |
| common-full-cli-v3-pipe-control-first | 0 | 1 pass / 0 fail / 0 ignored |
| common-full-cli-failed-targets-recheck | 0 | 43 pass: 20 closed-pipe + 23 one-shot |
| common-full-cli-corrected-tests-first | 0 | 140 pass / 0 fail / 0 ignored |
| common-full-cli-corrected-strict-first | 0 | strict all-target Clippy |
| common-full-cli-corrected-fmt-first | 0 | owning-workspace fmt check |

Each command has original full log, exact argv/cwd/manifest/environment, actual exit, continuous resources and warm target inventory. common-full-final-command-index.json collects those records and preserves per-harness counts. Ordinary test parallelism and no-fail-fast were used; no test or assertion was removed or weakened. The recorded harnesses have zero ignored cases. The dependency build's preexisting live-Vault warning remains unexercised provider coverage; it is not a claimed live test. No additional test execution was used to inflate case counts.

Every initial CLI failure remains in common-full-cli-first-failures.md and its 30-member immutable seal: two long-path doctor failures, the retained v2 pipe-message control exposed by the v3 default, and the two lost local daemon-required hints. An initial manifest-verification script assumed the earlier list shape and raised KeyError files; after validating the reference SHA/clean HEAD it completed all 1228 hash checks using the actual map shape during frozen-source compilation. That verification-script observation is separate from product/compiler results. This final lane had no compiler error, correction retry, manual interruption or resource crossing. All earlier implementation product reds, absent-API compiler observations and the original interrupted privacy fixture remain preserved in the linked predecessor reports/seals.

Exact source and handoff:

- Original B implementation is committed d7e7a673600c868078659d6412a1f6ccb7aefca8. The complete tested counterpart is 4403f01796b50cfd16eb40431fcf01c79ec98407, integrated at current HEAD 4dbe9065ec7cf9e5c100784c1b040b879826a84d.
- Current working delta is exactly crates/connectors-cli/src/lib.rs, crates/connectors-cli/tests/closed_pipe.rs and crates/connectors-cli/tests/remediation.rs. Patch SHA256 116cf2a17f382d95a4bed2643982ab20cedc93e53239f01f65a700ff99c21c48 matches root's handoff and stayed unchanged through all actual-API commands.
- common-full-final-source-inventory.json verifies all 1228 source objects byte-for-byte against the actual-API starting snapshot. The three working files are copied into common-full-final-source/; exact diff is common-full-final-working.patch. No other dirty/untracked source path, no early carrier and no counterpart edit by B.
- common-full-final-preservation.json records the exact inverse old-test proof and line count. common-full-committed-counterpart-path-diff.txt distinguishes committed A/planning changes since the console observation from B's three-file correction.

Resources and outside writes:
All builds used B's own ordinary root/console/CLI targets, aggregate 8 GiB cap, one job, CARGO_TARGET_DIR unset, existing /usr/bin/sccache, incremental 0 and dev/test debug 0. The minimum measured resources across these nine commands were 14780837888 bytes disk, 16641052672 bytes tmpfs and 36321738752 bytes MemAvailable; maximum sampled aggregate target allocation 5403308032 bytes. No reserve was crossed. The final explicit inventory contains 15,259 files allocating 5,403,316,224 bytes and 309 executable files. These inventories include retained warm/historical artifacts; the command provenance identifies each actual tested build, and the current CLI binary is SHA-pinned in the executable inventory.

The compiler slot was explicitly released after final fmt; a read-only /proc observation found no owned Cargo/rustc/clippy/CLI process. Targets remain untouched, with no copy, symlink or cleanup. Outside writes are this assigned scratch directory, newly approved short private TMP directory and test fixtures there, existing assigned target output, and normal preexisting Cargo/sccache activity. No Git/AEP/ESS/model edit, provider/operator action, external source edit, operator credential/config access or live integration was performed by B. All prior evidence is unchanged. The separate portable report abbreviates only the original absolute home-directory prefix.

This is the completed assigned affected-workspace result. Coordinator-owned common integration, complete twelve-workspace/thirteen-configuration verification and independent whole-auth attack remain required; this report does not declare the whole story or a release green.
