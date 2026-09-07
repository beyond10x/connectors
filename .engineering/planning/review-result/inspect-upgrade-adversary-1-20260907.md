---
format: aep.planning-md/1
id: review-result:inspect-upgrade-adversary-1-20260907
kind: review-result
status: active
title: 'Inspect upgrade: adversary pass 1'
relations:
- reviews: story:the-binary-says-what-it-carries
revision: 1
---
unit: inspect upgrade at 1b1cf58c27d34f5cda4a376f8815ebcb6db3e0e8, base 80c7666f4a36dc3b8902a1c8106a5edfe7afefe1, plus two test-only additions
verdict: nothing found
cases: executed 258→262, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 assigned roots, with exact retained members inventoried below
needs-coordinator: record the pass; no implementation correction requested

Portable record: local home prefixes are rendered as `~/`; the exact raw report and its SHA256 are retained in the assigned review scratch and wave record.
```text
$ git --no-pager diff HEAD --stat
(empty: both additions are untracked; no existing tracked file changed)
$ git status --short
?? crates/connectors-cli/tests/upgrade_adversary.rs
?? crates/connectors-console/tests/upgrade_adversary.rs
$ git --no-pager diff --no-index --stat /dev/null <each new test file>
 .../connectors-cli/tests/upgrade_adversary.rs      | 71 ++++++++++++++++++++++
 1 file changed, 71 insertions(+)
 .../connectors-console/tests/upgrade_adversary.rs  | 89 ++++++++++++++++++++++
 1 file changed, 89 insertions(+)
```

The complete additions are in `test-only.patch`; final identities are in `source-final.sha256`. `tracked-diff-final.txt` is empty. No implementation, existing assertion, manifest, ESS projection, Git index/ref or AEP record was changed. The unit's complete committed diff, acceptance, new tests, changed documentation, owning constants and actual binary/library callers were read before candidate execution.

1. Candidate cases, written before the first run

All commands, environments, process-group IDs, complete stdout/stderr and exit results are retained in the adjacent `<label>.command`, `.log`, `.result` and `.resources` files. `raw-execution-transcript.md` reproduces these records verbatim in execution order. `candidates-before-first-run.sha256` identifies the initial candidate bytes.

| Candidate and actual reach | First result | Final disposition |
|---|---|---|
| `embedding_upgrade_completes_on_first_poll_without_a_tokio_runtime`: public `run_from` future polled directly once with a standard-library no-op waker | 1 passed, exit 0 (`candidate-cli-embedding`) | Retained; no runtime required by the embedded report |
| `synchronous_dispatch_does_not_claim_other_commands_or_parser_errors`: four calls covering an ordinary inspect command, unknown flag, leaf help and version | 1 passed, exit 0 (`candidate-cli-routing`) | Retained; all returned `None` for ordinary dispatch |
| `output_flags_at_each_clap_depth_keep_upgrade_synchronous_and_equivalent`: 12 real binary invocations, four output modes at three flag positions, with `TOKIO_WORKER_THREADS=0` | 1 passed, exit 0 (`candidate-cli-options`) | Retained; equal bytes within each format, JSON/YAML facts equal |
| `traced_report_never_touches_poisoned_installation_paths_or_creates_sockets`: four real binary invocations under native strace, with unusable synthetic home/config/state paths | 1 passed, exit 0 (`candidate-cli-syscalls`) | Complete initial source retained in scratch as `native-probe-before-relocation.rs`; removed from repository tests at coordinator request because `/usr/bin/strace` is not a declared Linux test prerequisite |
| `reported_versions_match_ordinary_prepared_reopened_and_later_writes`: real FileStore writes, v1 reopening, prepare, v2 reopening/commit/reclaim, later ordinary write and reopening | Initial fixture setup failed, then 1 passed, exit 0 (`candidate-console-storage-fixture-corrected`) | Retained; v1/v2 report agrees with actual file headers and reopened values; report unchanged by writes |

The storage case's first execution reached the existing directory guard before any credential write. Its complete first output is in `candidate-console-storage.log` (exit 101, 0 passed / 1 failed). Exact diagnostic:

```text
called `Result::unwrap()` on an `Err` value: Denied { path: "~/.cache/cw7/upgrade/adversary-1/.tmp8Y7JNv", reason: "its mode is 0755, wider than 0700; create an owner-only child directory or use a conventional per-user state root. Never narrow a shared ancestor for this store" }
```

`console-first-fixture-mode.txt` retains the actual 0755 observation and `console-candidate-before-mode-correction.rs` preserves the initial source. Only the newly created fixture directory was given 0700. This is an unsuccessful fixture setup, not a product finding. The corrected case ran alone before the suites. At coordinator request, its final version retains the TempDir guard so future test runs clean their own store; all assertions remain unchanged. The final single-case run, strict test-target Clippy and formatting passed after that lifetime-only correction. The prior kept fixtures and source remain in scratch.

2. Suites and exact counts

The implementor supplied CLI 148 and console 110 executed cases: the comparable baseline is 258, not a suite run made before adding candidates. The separate root connector-secrets/client checkpoint of 125 executed and three existing ignored live-Vault cases was supplied by the implementor and was not rerun in this pass.

| Label | Exact command within named workspace | Executed result / exit |
|---|---|---|
| `suite-cli` | CLI: `cargo test --workspace --locked --offline --no-fail-fast` | 152 executed: 150 passed / 2 failed; exit 101 |
| `suite-cli-final` | CLI: `cargo test --workspace --locked --offline --no-fail-fast` | 151 passed / 0 failed; exit 0 |
| `suite-console-final` | Console: `cargo test --workspace --locked --offline --no-fail-fast` | 111 passed / 0 failed; exit 0 |
| `strict-cli`, `strict-console` | Each workspace: `cargo clippy --workspace --locked --offline --all-targets -- -D warnings` | Both exit 0 |
| `fmt-cli`, `fmt-console` | Each workspace: `cargo fmt --all --check` | Both exit 0 |
| `candidate-console-storage-final` | Console: `cargo test --locked --offline --test upgrade_adversary reported_versions_match_ordinary_prepared_reopened_and_later_writes -- --exact --nocapture` | 1 passed; exit 0 |
| `strict-console-final-test` | Console: `cargo clippy --locked --offline --test upgrade_adversary -- -D warnings` | Exit 0 |
| `fmt-console-final` | Console: `cargo fmt --all --check` | Exit 0 |

CLI cwd is `~/.local/state/worktree/trees/b10x/connectors/wt-inspect-upgrade-20260907/crates/connectors-cli`; console cwd is its sibling `crates/connectors-console`. The isolated CLI candidate commands use `cargo test --locked --offline --test upgrade_adversary <exact function name> -- --exact --nocapture`; their exact expanded arguments and all raw output appear in the transcript.

The first full CLI run preceded native-probe relocation and thus executed four added CLI cases. It failed only two existing doctor fixtures: `a_healthy_doctor_accepts_a_closed_report_reader` and `doctor_enumerates_bounded_and_persistent_verbs`. The latter's actual report measured a 115-byte Connect Session path against the 107-byte limit. Complete first output is preserved in `suite-cli.log`; `doctor-owner-and-tests-base-diff.txt` confirms those source/test files are unchanged from base. The coordinator then assigned `~/.cache/cw7/a1` (verified 0700) as shorter TMPDIR. Both unchanged cases passed in the final full CLI suite. No failure was skipped, and no doctor assertion or path guard was changed.

After native-probe relocation, the retained suite grew by three CLI and one console cases: **258→262** (151+111). Initial individual runs, the extra native case, the first 152-case CLI run and final lifetime-only single-case recheck are recorded separately, not summed as unique coverage. Final full-suite output and all earlier failures are retained verbatim in the transcript and original logs.

3. Findings and limits

No product finding. The three failed assertions encountered during execution were one private-directory setup refusal and two longer-TMPDIR fixture refusals; their corrected executions and untouched production guards are described above. There is no introduced/pre-existing/undecided product-finding classification to assign.

The bounded attack could not break synchronous dispatch, existing embedding, global output-option handling or the actual advertised credential-format transition. The four initial syscall traces and four first-suite repetitions are retained beneath the two `upgrade-adversary-*` directories in the long scratch. They show the actual report invocation making no socket, socketpair, connect, sendto or sendmsg call and no access to the poisoned installation paths. This concerns report processes, not compiler IPC or unrelated test fixtures. It is not a power-loss simulation or a live-provider/Vault test. Source `Taskfile.yaml` confirms `task install` is the existing source-checkout installer; no installation or update was attempted. ESS projection identity was supplied by the coordinator, not regenerated in this pass.

4. Resources, writes and freeze

All 16 guarded command records are complete. Minimum observed free disk was **92,213,207,040 bytes**; maximum aggregate of the three assigned in-tree targets was **4,976,701,440 bytes**, below the 8 GiB cap. The 20 GB floor held. Jobs remained 1, incremental/debug info disabled, `CARGO_TARGET_DIR` unset, with prescribed `/usr/bin/sccache`. The first command began at 2026-09-07T11:34:21Z; the release timestamp is in `compiler-released-at.txt`. `processes-after.txt` is empty: no recorded process group remained when the compiler slot was released. No targets were cleaned or removed.

Outside-tree write roots, in full:

- `~/.cache/cw7/upgrade/adversary-1`: candidate preimages, native probes and their synthetic fixtures, raw commands/logs/results/resource records, report, patches and manifests.
- `~/.cache/cw7/a1`: coordinator-assigned shorter private TMPDIR for later commands and test fixtures; automatic test-fixture lifetimes may remove their own temporary children.
- `~/.cache/cw7/sccache`: prescribed compiler cache, unchanged ownership; cache entries modified since the candidate preflight are listed separately.

`outside-tree-paths.txt` lists retained members with absolute paths; `compiler-cache-modified-paths.txt` lists observed cache modifications. `evidence.sha256` seals the report and retained scratch files. Temporary compiler/test children already removed by their normal lifetimes cannot be inventoried afterward; their assigned roots and exact command environments remain recorded. The two worktree test files are sealed separately by `source-final.sha256`. No live credentials, operator configuration or external integrations were accessed by this review. Harness token usage and aggregate tool-call accounting are unavailable; 16 guarded Cargo command records and the native invocation reach above are available.

```findings
[]
```
