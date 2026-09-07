---
format: aep.planning-md/1
id: review-result:inspect-upgrade-adversary-2-20260907
kind: review-result
status: active
title: 'Inspect upgrade extraction: final independent review'
relations:
- reviews: story:the-binary-says-what-it-carries
revision: 1
---
unit: inspect upgrade extraction, 0f2f5c64a63bd05ec016207288e694e8dd2c21d8 versus 157ef0cc3ad8102b4c20d13c24dfb52d996e3aa8
verdict: nothing found
cases: executed 4→4, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 assigned roots, retained members inventoried below
needs-coordinator: record this final pass; no correction requested

Portable record: home prefixes are normalized; original report SHA256 is 9a83dc8e3f3176fa9b56d91eb39a90bf5ebb7a296f8de2f4d7dd8ee00a3098e8.
```text
$ git --no-pager diff HEAD --stat
(empty)
$ git status --short
(empty)
```

No repository file, existing test, waiver, implementation, Git index/ref or AEP record was changed. The exact two-file correction is retained in `reviewed-correction.patch`; six relevant source/test identities are in `source-final.sha256`. `candidate.md`, `source-probe.sh` and `output-probe.sh` were written and hashed before execution. The exact adversary charter, correction, prior report, acceptance, shared-format callers and architecture ceiling guided this bounded attack.

| Probe / exact command | Measured result |
|---|---|
| `bash ~/.cache/cw7/upgrade/adversary-2/source-probe.sh` | Exit 0: reversing only the declared extraction reconstructs the entire prior `file.rs` byte for byte; `prepared.rs` is byte-identical; actual parent 2617 lines, original 2625 ceiling remains |
| CLI cwd: `cargo test --locked --offline --test upgrade_adversary -- --nocapture` | Exit 0: 3 passed, 0 failed, 0 ignored |
| Console cwd: `cargo test --locked --offline --test upgrade_adversary reported_versions_match_ordinary_prepared_reopened_and_later_writes -- --exact --nocapture` | Exit 0: 1 passed, 0 failed, 0 ignored |
| `bash ~/.cache/cw7/upgrade/adversary-2/output-probe.sh` | Exit 0: four real current-binary invocations, four stdout byte comparisons equal, four empty stderr files |

The native scripts are exact executable probe records. `raw-execution-transcript.md` reproduces all four commands, complete outputs, environment/cwd, process groups and exits verbatim; original records remain in `native-source.*`, `cli-callers.*`, `console-storage.*` and `native-output.*`.

Reach: the CLI cases exercise public embedding without Tokio, fast-path routing of four other/parser-help inputs, and 12 actual binary invocations across output modes and flag positions. The console case reaches the unchanged public `file::supported_format_versions` API through the real report and compares it with ordinary v1 writes, prepared v2 writes, commit/reclaim, and reopened values. It retains its normal TempDir guard. These four existing cases had already executed in pass 1; there are no new cases or new skip/ignore conditions. Thus the selected coverage is **4→4**, not an increase in the prior full CLI151/console111 record. No full CLI/root suite was rerun, and the coordinator-supplied correction checks are not counted as executions here.

The separate native output probe runs `connectors inspect upgrade -o <text|compact|json|yaml>` from the freshly rebuilt CLI target with `TOKIO_WORKER_THREADS=0`, synthetic unusable HOME/XDG paths and an unusable PATH. Each output equals its first-pass native counterpart under `~/.cache/cw7/upgrade/adversary-1/upgrade-adversary-549586-0/`. Current stdout/stderr and executable SHA256 are retained. No strace or other undeclared prerequisite was added to Cargo tests; this pass does not claim a new syscall trace. The source reconstruction, real callers and unchanged output bytes are the measured extraction checks.

No finding or failed command. The coordinator-reported earlier architecture-fence failure is the correction's premise; this pass measured that the original ceiling remains unchanged and now contains the parent. Parser/writer and reported-format identities survived the move, including the public reexport. No current introduced, pre-existing or undecided finding remains to classify. These observations neither simulate a power loss nor exercise live providers or installation/update behavior.

All four guarded process groups finished; `processes-after.txt` is empty. The compiler lane was released before hashing/reporting, at the timestamp in `compiler-released-at.txt`. Minimum observed free disk was **70,906,621,952 bytes**; maximum aggregate target size was **5,342,310,400 bytes**, within the 20 GB floor and 8 GiB cap. The prescribed environment kept one job, debug/incremental disabled, ordinary per-worktree targets and `/usr/bin/sccache`; `CARGO_TARGET_DIR` remained unset. No target or scratch cleanup was performed.

Outside-tree writes, in full:

- `~/.cache/cw7/upgrade/adversary-2`: candidate, native scripts and source copies, synthetic poison files, stdout/stderr, raw command/result/resource records, report and manifests.
- `~/.cache/cw7/a2`: coordinator-assigned 0700 TMPDIR; normal compiler/test temporary children may remove themselves on completion.
- `~/.cache/cw7/sccache`: prescribed compiler cache; observed modifications since the candidate preflight are listed in `compiler-cache-modified-paths.txt`.

`outside-tree-paths.txt` enumerates retained scratch members with absolute paths. `evidence.sha256` seals retained report/probe files, and `source-final.sha256` seals source separately. Transient children already removed by their ordinary lifetimes cannot be enumerated afterward; their exact command environments and assigned root remain recorded. Four guarded commands, two Cargo invocations and 16 explicit binary invocations are exposed by these records. Aggregate harness tool counts and token/cost accounting are unavailable.

```findings
[]
```
