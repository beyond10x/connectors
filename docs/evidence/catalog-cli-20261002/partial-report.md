unit: story:catalog-cli-journeys — Catalog CLI acceptance
verdict: red
cases: executed 48→56, red 1
origin: n/a
wrote-outside-worktree: $HOME/.cache/c26d/cat; $HOME/.cache/sccache; $HOME/.cargo; managed worktree lease registry
needs-coordinator: yes — ignored-classifier.patch; retain unmet settlement modes8–11 and scope a replacement fault mechanism

Eight of ten logical obligations and eleven of fifteen historical variants passed. The unchanged-permission settlement control passed. Mode8 failed because chmod0400 did not prevent current ER settlement; modes9–11 remain unexecuted and unmet. This is rejection of the proposed fault injection, not evidence of a product settlement defect. No assertion was weakened to call it green. The source is frozen and the live/build window released.

## 1. Unit and acceptance

story:catalog-cli-journeys: preserve the three native lifecycle obligations and twelve native mutation/recovery variants from8dac09e through the shipped catalog selection, including exact effects, refusal/replay, audit, key states, retention and revoked disclosure. Follow active story plus coordinator's rev8 owner-helper boundary, copied as owner-helper-boundary.md. Initial base71dec8e6db5ca4fb87e5244662443a0d45d2a756; root supplied shared manifest/lock update f3fb222b7edc7fc29520dd30effdb58bfdec5274. No worker Git commit or AEP write.

## 2. Actual source shape

`git --no-pager diff --stat` (also diff-stat.txt):

```text
 adapters/catalog/tests/local_runtime.rs            |  91 +++++++++++++++++-
 .../catalog/tests/local_runtime/cli_journey.rs     | 104 +++++++++++++++++++--
 2 files changed, 186 insertions(+), 9 deletions(-)
```

The authoritative exact stat is diff-stat.txt; the tracked numstat is89 additions/2 deletions in local_runtime.rs and97 additions/7 deletions in cli_journey.rs. Five assigned untracked Rust modules are also part of the candidate: background_recovery.rs257 lines, guarded_merge.rs1195, lifecycle.rs313, recorded_state.rs449, owner_replay.rs161. They are not included by ordinary `git diff --stat`. Total candidate additions2561/deletions9 across seven test files. The root-owned dev manifest/lock commit is already in HEAD and not part of this working diff.

Mechanical adaptations preserve the current catalog input/body/result and HTTP409→invalid_input/Refused contract. The provider models a deliberate pinned-open preflight snapshot separately from the merged PUT response; mode0 first asserts a merged preflight representation refuses before proof spend, then explicitly selects the opened snapshot to test approval_replayed. This is controlled fixture input, not a claim that real GitLab stays opened after a merge.

The test-only observer admits existing metadata, reads only the immutable physical authority marker, loads the exact production ER definition bundle and opens that same recorded authority. Fresh finite complete snapshots join typed AttemptRecord/KeyReservation/AuditRecord/ApprovalRedemption facts. Exact physical `s:<logical-id>` encoding is verified against each identity field; no arbitrary prefix stripping, legacy business SQL, authority copy/provision/import or production inspection export. Public owning Store observations are cross-checks at quiescent stages, not a claimed atomic combined read. Shutdown must join.

Current executable-image admission is preserved. Protected-stdin capture remains production CLI acceptance. Settled owner/2 replay is explicitly a same-image host-library segment after original production CLI settlement: the same libtest executable launches unmodified owner::serve with real inherited startupFD3 and verified lifetime-lockFD4. It uses actual Client/WriteClient start=false and the real build guard. Mode0 proves handshake/shutdown/socket removal/lock release before replay. The pending mode3 observation still uses the production CLI owner. See owner-build-discrepancy.md for the historical mismatch and owner-helper-boundary.md for the authoritative correction; the former's proposed split was superseded by the approved same-image helper.

## 3. Retained red evidence

All commands ran in this assigned tree. Cargo commands used `CARGO_BUILD_JOBS=2 RUSTC_WRAPPER=/usr/bin/sccache TMPDIR=$HOME/.cache/c26d/cat`, default local target, release profile and --locked. No CARGO_TARGET_DIR or lock update.

Compile command:

```console
cargo test --release --locked -p connectors-catalog-provider --test local_runtime --no-run
```

expanded-build-1.log, exit101:

```text
error[E0282]: type annotations needed
   --> adapters/catalog/tests/local_runtime/recorded_state.rs:252:45
252 |                     text(row, "request_id").into(),
```

Fixed by explicit `Vec<RecoveryState>`; subsequent builds2–7 exit0. No behavior assertion changed.

Every live command used `TMPDIR=$HOME/.cache/c26d/cat CONNECTORS_TEST_CLI=$HOME/.local/state/worktree/trees/b10x/connectors/cb26d-catalog/target/release/connectors` and this libtest invocation:

```console
target/release/deps/local_runtime-9b8f73d2db6f0a0a --ignored --exact <full-name> --nocapture
```

Exact names are in the acceptance table below. Retained failed runs:

| Raw log | Exact failure and disposition | Exit |
| --- | --- | --- |
| lifecycle-repair-1.log | `Error { code: OwnerBuildMismatch, acquisition: None, service_code: None, origin: Host }`; replaced invalid libtest→CLI-owner capture with real CLI protected stdin |101|
| merge-matrix-1.log | `exact original attempt missing`; fixture confused physical ER subject IDs with logical attempt IDs; corrected exact declared identity/encoding joins |101|
| merge-matrix-2.log | `left: String("forbidden")`, `right: "approval_replayed"`; real merged-state guard refusal, retained as a control before deliberately pinned-open spend test |101|
| merge-matrix-3.log | `fixture owner exited before readiness`; first helper diagnostic did not retain child's stderr |101|
| merge-matrix-4.log | Child exit101, `owner_replay.rs:155:47`, `Err value: InvalidConfiguration`; exact failure was private root admission before serve. Create the owned root with tempfile Builder.permissions0700; retain private check |101|
| merge-settlement-1.log | Control12 passes, chmod8 permits known Applied settlement; no new fault attempted |101|

The deciding settlement output, verbatim:

```text
catalog settlement control 12: start
catalog settlement control: held Applied response settled; audit complete
catalog settlement control 12: passed
catalog historical variant 8: start
catalog settlement injection mode 8: classification="applied" cause=null audit="complete" durable=("completed", "replayable", Some(1789076482762), Some(1789162882762))
```

```text
  left: Null
 right: "unavailable"
catalog observer shutdown during unwinding: Joined { provider: Ok(()) }
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 67 filtered out; finished in 12.00s
```

The full raw log retains the actual known response, attempt030d7eee-642a-4727-b8d6-74b203487550, request6cb54add-5f18-4605-86b0-5cc59d2d3b8a and completed audit61850c2e-4798-43fd-88db-14615956a8e2. The same exact pre-held original attempt was re-observed after response release. Before injection, passing assertions required one PUT/effect, Dispatching/Pending with no settlement/expiry, exactly one Spent redemption and one incomplete admitted audit joined by original request/operation/connection. Chmod only targeted that fixture's metadata.sqlite3 files. MetadataFault restored every still-existing file to its saved permissions during unwind; restoration errors would panic, and no such error occurred. The observer joined and fixture cleanup completed.

Evidence limitation: raw complete snapshots and a separate permission read-back transcript were not serialized before TempDir cleanup. The retained pre-state evidence is the executed exact assertions and their source, while the post-state tuple and full known response are explicit in the raw log. Do not describe these as archived full pre/post snapshot files. No additional rerun was performed after the coordinator froze this seam.

## 4. Green evidence and counts

Original baseline compilation used:

```console
cargo test --release --locked -p connectors-catalog-provider --no-run
```

baseline-build.log exit0. Only the worker's own authored tracked files were saved/restored around that baseline. The original local-runtime executable was copied to baseline-local-runtime; baseline-binaries.sha256 records identity. The backup copies under authored/ are obsolete after later edits and must not be restored. CLI command `cargo build --release --locked -p connectors` exited0,25.32s in cli-build-1.log.

Baseline ordinary command `.local/provider-wave/catalog/baseline-local-runtime --test-threads=1`, exit0:

```text
test result: ok. 47 passed; 0 failed; 10 ignored; 0 measured; 0 filtered out; finished in 15.80s
```

Baseline exact original restart command used baseline-local-runtime plus the common CLI env and `--ignored --exact cli_journey::gitlab_catalog_cli_reuses_custody_across_owner_and_keyring_restart --nocapture`, exit0:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 56 filtered out; finished in 6.17s
```

Acceptance cases: prepend `cli_journey::` to each full suffix below and use the exact common live command above. Helpers never count as journeys.

| Full suffix | Log | Deciding runner output, verbatim | Obligation/variants |
| --- | --- | --- | --- |
|gitlab_catalog_cli_reuses_custody_across_owner_and_keyring_restart|expanded-restart-1.log|`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 66 filtered out; finished in 13.15s`|1/1|
|lifecycle::catalog_cli_failed_repair_and_busy_stop_preserve_authority|lifecycle-repair-2.log|`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 67 filtered out; finished in 7.29s`|1/1|
|lifecycle::catalog_cli_explicit_revalidation_after_real_expiry_without_reentry|lifecycle-expiry-1.log|`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 67 filtered out; finished in 66.47s`|1/1, real Timing|
|guarded_merge::catalog_cli_guarded_merge_applied_refused_and_lost_response_restart|merge-matrix-5.log|`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 67 filtered out; finished in 58.81s`|1/4, modes0,2,1,3|
|guarded_merge::catalog_cli_guarded_merge_revocation_finishes_admitted_audit|merge-revocation-1.log|`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 67 filtered out; finished in 7.21s`|1/1, mode4|
|guarded_merge::catalog_cli_recovers_abandoned_preparation_only_with_trusted_time|merge-abandoned-1.log|`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 67 filtered out; finished in 10.96s`|1/1, mode5|
|guarded_merge::catalog_cli_background_recovery_waits_for_live_work_and_recovers_unkeyed_attempts|merge-background-1.log|`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 67 filtered out; finished in 25.32s`|1/1, mode6|
|guarded_merge::catalog_cli_background_recovers_revoked_removed_target_without_disclosure|merge-removed-1.log|`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 67 filtered out; finished in 23.41s`|1/1, mode7|
|guarded_merge::catalog_cli_failed_settlement_keeps_the_known_effect_and_recovers_conservatively|merge-settlement-1.log|`test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 67 filtered out; finished in 12.00s`|unmet: mode8 red,9–10 unexecuted; extra control12 passed|
|guarded_merge::catalog_adversary_revoked_disclosure_of_an_applied_merge_never_answers_not_attempted|not executed after seam rejection|No execution claim|unmet: mode11|

Every green row exited0. The matrix log explicitly reports each mode passed and five same-image helper clean exits (mode0 smoke+replay and one replay for2/1/3):

```text
same-image host-library owner: real handshake and build admission passed
same-image host-library owner: exit0, socket removed, lifetime lock released
```

The production CLI/provider binaries remained unchanged across runs. The first three lifecycle greens preceded the final fixture root-permission/merge-preflight/helper refinements; their raw filtered counts identify those earlier test builds. They were not rerun after the coordinator froze source to finish only ordinary/Clippy/fmt. Final mutation modes0–7/control12/chmod8 and the full ordinary package ran the final compiled test source.

Final affected ordinary package command `cargo test --release --locked -p connectors-catalog-provider`, package-ordinary.log, exit0: runner summaries sum332 passed,0 failed,21 ignored across44 target summaries (including empty lib/bin/doc targets). Final local-runtime ordinary summary:

```text
test result: ok. 47 passed; 0 failed; 21 ignored; 0 measured; 0 filtered out; finished in 5.11s
```

Lane counts: local-runtime ordinary executed47→47, exit0; selected catalog journeys executed1→9, exit101 across selection because8 passed/1 failed; two helpers excluded. Unique local-runtime cases executed48→56,55 green/1 red. Full-package332 ordinary cases have no separately executed original full-package baseline; do not invent one. Combined final package+selected unique executions341,340 green/1 red. These aggregate arithmetic counts do not turn21 ignored cases into passes. Inventory57→68 includes9 new journey functions and2 new helpers; final-inventory.txt records actual libtest output.

`cargo clippy --release --locked -p connectors-catalog-provider --all-targets -- -D warnings`, clippy-1.log, exit0:

```text
    Finished `release` profile [optimized] target(s) in 34.40s
```

`rustfmt --edition 2024 --config skip_children=true --check` on all seven changed test sources: fmt-final.log empty, exit0. `git diff --check`: empty, exit0. No blanket workspace format.

Identities: final-binaries.sha256 and final-sources.sha256; raw logs hashed in logs.sha256. Final production CLI1bfc86834f96f2d61ad4e7842005e6d25bf615203b5dd82806055ad56d15f4a7; catalog child46a2886b00e72846d8a15fb69f19ce761db34406e1002b3d7765629a19c63c6f; final libtest34b13b95d09fe283c15f67e88012790b588b4441ab89558afe7d66511e585094. Pinned ER72455539 and Eventlog0a04846 unchanged.

## 5. Exclusions and handoff

- Modes8–11 deliberately remain unmet. No alternate fault, broader filesystem damage, production fault hook or deadline extension was attempted. Root owns the separate replacement-seam decision; do not merge this as fully accepted.
- No real GitLab sandbox, installed forge integration, unrelated provider or external message. Local TLS provider, signing clock and private GNOME/dbus custody are deterministic acceptance fixtures.
- No production source, contract, ESS generated file, AEP, shared classifier or Cargo.lock mutation by this worker. Root already supplied approved dependencies. ignored-classifier.patch is an unapplied proposal updated to exclude both exact helper names; reconcile it with root's existing classifier edits.
- No repository-wide task check, ESS regeneration, full ignored-family runner or publication was run by this worker. Root owns integration/gate decisions. Neither ordinary green nor helper success closes the failed fault obligations.
- Production owner/child crash and sequential handoff use exact SO_PEERPIDFD handles and observed exits. Fixture helpers require successful waited exit0 and released lock. Seed producers require exit73. Custody/provider/clock fixtures own their process/thread handles and drop/wait/join them. All command sessions ended. Final `pgrep -af '$HOME/.cache/c26d/cat/'` returned1 with no matches; no task-owned fixture process remained in that observed scope.
- Current target1.5GiB/free40GiB. Targets, logs, source and managed tree retained. No cleanup/Git publication. Coordinator receives this dirty candidate and decides the next scoped step. Worker lease is released at handoff only.

## 6. Writes outside the worktree

Assigned physical temporary fixture directory `$HOME/.cache/c26d/cat` (individual TempDirs auto-cleaned by tests); standard Cargo cache `$HOME/.cargo`; compiler cache `$HOME/.cache/sccache`; worktree CLI session lease state under `$HOME/.local/state/worktree`. These are the authorized fixture/tool/lease writes. All source, logs, patches, binary copies and this report are in the assigned tree. No other repository was written.

Redaction notice: the sole redaction replaces the private home-directory prefix with literal `$HOME`. This notice is added; all other report content is preserved. The original report remains unchanged with SHA256 `445aca6ac928d61e9064fc54f59d077ae95f67528b35217dd378008069dcadb9`.
