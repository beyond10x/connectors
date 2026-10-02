unit: story:catalog-cli-journeys — Catalog CLI acceptance, correction through rev12
verdict: green implementation evidence; independent whole-candidate review and root gate pending
cases: baseline48;57 unique local-runtime cases with passing evidence;10logical/15historical variants; final affected selection2passed/0failed
origin: n/a
wrote-outside-worktree: $HOME/.cache/c26d/cat; $HOME/.cache/sccache; $HOME/.cargo; managed worktree lease registry
needs-coordinator: yes — shared classifier/prerequisites, independent review, integration/gates and Git publication

This report is publication-safe: the sole path redaction is replacing the private home prefix with literal `$HOME`. Raw logs and hash manifests retain actual task paths in assigned private scratch. The first report.md, report-public.md and every red log remain unchanged; this is a new final report, not a rewrite of history.

All ten logical obligations and fifteen historical variants now have passing evidence. The final source is frozen; only this report/evidence assembly followed the final checks. This is consolidated evidence across the first implementation and the explicitly scoped fault corrections, not a claim that all fifteen variants ran together against one final executable. Evidence-reuse limits are explicit below.

## Scope and source identities

Tree `$HOME/.local/state/worktree/trees/b10x/connectors/cb26d-catalog`, branch acceptance/cb26d-catalog. Opening base71dec8e6db5ca4fb87e5244662443a0d45d2a756; root supplied manifest/lock-only commit f3fb222b7edc7fc29520dd30effdb58bfdec5274. Worker made no production, dependency/version, AEP, shared-classifier or Git/publication changes. Approved scope is original story plus owner-helper rev8, fault replacement rev9, diagnostic rev10, recovery branch rev11 and restoration control rev12. All runnable additions are Rust; no new CLI or Python.

Author-owned delta against HEAD is **3,451 additions/10 deletions across eight test files**, including all six untracked modules. Plain git diff omits those modules and is insufficient evidence of total scope.

| File under adapters/catalog/tests/ | Additions/deletions or new-file lines | SHA256 |
| --- | --- | --- |
| local_runtime.rs | +89/-2 | d471f3ec4add0ff99a760080a9fb7138217d0092349164d6cda2e98a8d8bad71 |
| local_runtime/cli_journey.rs | +127/-8 | 877685b55949a230236f55a9a414e4374b847b0df33e6c0bd652e825df6c85df |
| local_runtime/background_recovery.rs | new257 | 06e6318eb2b8700e8690e466be9f5e6bbae261eb3c68f7c6fa9d2422ae4dff62 |
| local_runtime/guarded_merge.rs | new1298 | 9ee3ae85475d56677ac91fd3e231f21eca039d9d97116a177f8dda37dd3134e6 |
| local_runtime/lifecycle.rs | new313 | 3fceddf2a40fb76784cfd77a7d0cbd4c674cee86a3cba2c35fb84567b8f94deb |
| local_runtime/owner_replay.rs | new161 | 1ddcf26de557b001278361a81713b7f4da089ccc7e68a72e89d50d9df6f222c2 |
| local_runtime/recorded_state.rs | new468 | 9034be65cb8d754d4284dafd2be7bd8431435d1e9163960a30d288b1b5643765 |
| local_runtime/settlement_fault.rs | new738 | 087868cc1b758962101a07e62d8547bfa7e7d90035077bcb38b17eb051eb8e06 |

Source manifest seccomp-final-source.sha256: ccb9dd9b0b041116d20ffd8c14f95ba166a624a5550582f29e352e4a23304c81. Binary manifest seccomp-final-binaries.sha256: d2dbc4c098c9418b5b37e591755bdbcc0354b6627a9468add2c47c051b3c2421.

| Final release binary | SHA256 |
| --- | --- |
| target/release/connectors | 1bfc86834f96f2d61ad4e7842005e6d25bf615203b5dd82806055ad56d15f4a7 |
| target/release/connectors-catalog-provider | 46a2886b00e72846d8a15fb69f19ce761db34406e1002b3d7765629a19c63c6f |
| target/release/deps/local_runtime-9b8f73d2db6f0a0a | d2f9b0e66b85fa4722e3d6f09b9dacd908612630616720c541057d97bfad1890 |

## What the acceptance exercises

Lifecycle journeys use the actual production CLI, its genuine owner and private catalog child, disposable GNOME Secret Service/D-Bus and controlled local HTTPS GitLab responses. Capture is through protected CLI stdin, not a libtest client bypass of executable admission. Settled owner/2 replay is separately labeled fixture-host acceptance: the same libtest executable runs unmodified owner::serve with real inherited startupFD3 and verified lockedFD4. Parent and helper genuinely share the image; no rewritten digest or weakened same-build guard. Helper handshake, shutdown, socket removal and lifetime-lock release were measured. Pending replay remains genuine production CLI.

The public ER observer attaches to the existing metadata authority, validates complete capture coverage and typed identities and checks physical subject encoding exactly. Only schema-version/authority marker SQL is used, not stale business-table inspection. No second authority or production inspection export. Mutation assertions retain original attempt/request/business key, audit linkage, spent proof, timestamps and effects across recovery.

The controlled merge fixture declares its representation phases. Mode0 first proves the actual merged-state preflight refusal, then deliberately supplies a pinned opened snapshot to reach the spent-proof guard; it does not claim a real merged GitLab MR remains open. HTTP409 maps to the existing invalid_input/refused boundary. Provider counters distinguish PUT dispatch from actual effects.

The seccomp fixture installs an inherited filter only in task-owned production CLI descendants. An unfiltered listener monitor starts before spawn; descriptor transfer uses validated SCM_RIGHTS and pre_exec uses preallocated/stack data and syscalls. Only write/pwrite/writev/pwritev/fsync/fdatasync/ftruncate on exact canonical metadata database/WAL/SHM/journal regular-file identities can receive EIO. Notification validity, path/device/inode and observed mapping consistency are checked; ambiguity records an explicit error and never justifies a guessed denial. This is a controlled fault fixture, not a security boundary or atomic freeze of an fd table. Actual tested denials were pwrite64 on the owned WAL. Shared mmap/io_uring interception is not claimed.

Arming follows the held native response, one effect or exact refusal, original Dispatching/Pending, Spent redemption and incomplete admitted audit. All four armed cases received four actual EIO replies with zero recorded errors/cancellations. Known Applied/Refused response classifications, Unavailable/AttemptStore cause, incomplete audit and original durable uncertainty survive as required. Revoked10/11 preserve non-disclosure and no second effect.

Unrevoked8/9 require actual resumed metadata I/O and recovery after disarm. A quiescent snapshot after exact replacement-owner exit selects the existing path: Dispatching/Pending requires a new different owner; Indeterminate/Quarantined requires passive terminal replay with no owner. Every other pair is refused. Original identity, unknown/replayed=true, complete audit, quarantine, absent timestamps and no duplicate effect remain unconditional. Both paths were observed across retained runs. The final run observed passive replay in8/9; earlier diagnostic/final-repeat runs observed pending recovery through new owners.

Revoked10/11 do not assume passive replay writes. After all original assertions and exact owner exit, an explicit fixture control performs real fsync on a verified read-only metadata descriptor after filter installation, then executes the actual CLI --version and waits for exit0. It requires a new exact fsync CONTINUE, unchanged complete typed terminal state/effects and no owner. This is labeled **fixture read-only fsync restoration**, not production recovery or a revoked-path write. The observer drains before opening this raw descriptor and reopens the same authority afterward, avoiding raw-fd-close interference with SQLite process-owned locks.

Snapshot evidence serializes **all typed AttemptRecord/KeyReservation/AuditRecord/ApprovalRedemption terminal rows from a validated CompleteSnapshot capture**. It does not archive the full capture or event history. Whole-capture completeness and identity checks execute before selecting those four collections. Before/held/before-arm/post, spent-proof, quiescent and recovery observations are retained where applicable. These rows contain typed references, approval subject metadata and fictional fixture outcomes, not credentials, private keys or proof bytes; unrelated metadata is excluded. Original chmod run had assertions but lacked complete serialized before snapshots; this limitation is preserved.

## Cases, commands and actual results

Use the managed tree above, task-owned TMPDIR `$HOME/.cache/c26d/cat`, CARGO_BUILD_JOBS=2, RUSTC_WRAPPER=/usr/bin/sccache, default private target and --release --locked. Build command: `cargo test --release --locked -p connectors-catalog-provider --test local_runtime --no-run`. Production CLI/provider binaries remained unchanged throughout correction runs.

Live command: `TMPDIR=$HOME/.cache/c26d/cat CONNECTORS_TEST_CLI=<tree>/target/release/connectors <tree>/target/release/deps/local_runtime-9b8f73d2db6f0a0a --ignored --exact <case> --nocapture`. All cases below start with `cli_journey::`. Logs are in assigned `.local/provider-wave/catalog/` scratch.

| Exact case suffix | Latest retained green log | Logical / historical variants |
| --- | --- | --- |
| gitlab_catalog_cli_reuses_custody_across_owner_and_keyring_restart | expanded-restart-1.log,13.15s | 1/1 |
| lifecycle::catalog_cli_failed_repair_and_busy_stop_preserve_authority | lifecycle-repair-2.log,7.29s | 1/1 |
| lifecycle::catalog_cli_explicit_revalidation_after_real_expiry_without_reentry | lifecycle-expiry-1.log,66.47s real elapsed expiry | 1/1 |
| guarded_merge::catalog_cli_guarded_merge_applied_refused_and_lost_response_restart | merge-matrix-5.log,58.81s | 1/4, modes0/2/1/3 |
| guarded_merge::catalog_cli_guarded_merge_revocation_finishes_admitted_audit | merge-revocation-1.log,7.21s | 1/1, mode4 |
| guarded_merge::catalog_cli_recovers_abandoned_preparation_only_with_trusted_time | merge-abandoned-1.log,10.96s | 1/1, mode5 |
| guarded_merge::catalog_cli_background_recovery_waits_for_live_work_and_recovers_unkeyed_attempts | merge-background-1.log,25.32s | 1/1, mode6 |
| guarded_merge::catalog_cli_background_recovers_revoked_removed_target_without_disclosure | merge-removed-1.log,23.41s | 1/1, mode7 |
| guarded_merge::catalog_cli_failed_settlement_keeps_the_known_effect_and_recovers_conservatively | seccomp-restoration-matrix-1.log,73.54s | 1/3, modes8/9/10; extra control12 excluded from historical count |
| guarded_merge::catalog_adversary_revoked_disclosure_of_an_applied_merge_never_answers_not_attempted | seccomp-restoration-revoked-1.log,23.10s | 1/1, mode11 |

Final two live runs each exited0 with verbatim summary `1 passed; 0 failed; 0 ignored; 0 measured; 67 filtered out`. The coordinator's final run ordering was honored: inspected matrix exit0 before launching separate11. All cases in the table have green evidence; the first eight rows reuse earlier test builds. Lifecycle rows even predate the original helper/root-permission refinements; the original report records those build/count limits. Their acceptance bodies remain, but this worker did not rerun them after seccomp corrections. New filter behavior is selected only for8–12; shared Cli cleanup/source did change. Independent whole-candidate review and root integration checks must assess that reuse; it is not an exact-final-binary all-case claim.

Final unarmed control12:3255target/8912unrelated CONTINUE,0EIO. Armed8:4EIO and2937target CONTINUE before disarm,4131after recovery; armed9:4EIO and2932→4124. Armed10:4EIO,3072→3224 with explicit fsync restoration; armed11:4EIO,3097→3239 with explicit fsync restoration. All recorded monitor errors/cancellations zero. Full syscall/file identity events remain in raw logs, not expanded here.

Final ordinary command `cargo test --release --locked -p connectors-catalog-provider`: exit0,44 test-result summaries totaling332passed/0failed/21ignored in seccomp-restoration-ordinary.log. Local-runtime ordinary remains47passed/0failed/21ignored. Final Clippy `cargo clippy --release --locked -p connectors-catalog-provider --all-targets -- -D warnings`: exit0,6.29s in seccomp-restoration-clippy.log. Scoped rustfmt --edition2024 --check over all eight files and git diff --check: exit0, empty logs seccomp-restoration-fmt.log/seccomp-restoration-diff-check.log.

Inventory57→68 comprises nine new journey functions and two excluded helpers. Baseline local-runtime evidence47 ordinary plus one original CLI journey=48 unique; final consolidated coverage47 ordinary plus10 selected journeys=57 unique,57 with passing evidence. Full-package332 ordinary plus10 selected=342 unique with passing evidence. Do not count the control, repeated runs or two helpers as additional journeys, turn21 ignored cases into ordinary passes, or invent an original full-package baseline. Helpers are prepared_attempt_exit_fixture and owner_replay::catalog_same_image_owner_fixture under cli_journey::guarded_merge; neither counts as conformance acceptance.

## Retained reds and execution deviation

The immutable first report describes capture OwnerBuildMismatch, ER physical/logical ID encoding, merged-preflight and helper-startup reds. The first fault approach was rejected: chmod0400 did not prevent ER settlement. It was a fixture limitation, not a product defect. Original report SHA445aca6ac928d61e9064fc54f59d077ae95f67528b35217dd378008069dcadb9; original public report SHAdd4b3831a042bc0090574ae10d8bf8bf785d8d0deb7f4b55e4179c24d053a453; chmod log SHA9269829524f6ba8ad61601aac25cbb7c5e3c77ec1371a0d3fb62ffebc44e046f.

First seccomp pair correctly injected EIO but failed the later unconditional owner-connect assertion. No full recovery response was retained there, so its cause remains unknown. Diagnostic pair passed with unchanged assertion and measured pending/new-owner recovery. Rev11 then selected the owner assertion from a quiescent typed precondition, preserving both existing paths. The first failure is not retroactively claimed to prove background recovery.

After an initially green complete corrected matrix, final review fixed a mutex-poison cleanup risk and Clippy's constant-assert refusal. The justified repetition exposed another fixture assumption: revoked10 completed all original product assertions but no incidental metadata I/O followed disarm, so assert_resumed failed. Rev12 replaced only that incidental-write assumption with the explicit read-only fsync restoration control for10/11;8/9 retain real production persistence checks.

During that red repetition, a tool cell launched11 without checking the returned10 exit code. This violated stop-on-first-failure. The worker disclosed it immediately, let the already-started owned run finish cleanup, then stopped. That extra11 passed13.77s but is not labeled an authorized post-red continuation. The coordinator recorded the deviation; final sequencing explicitly inspected the prerequisite exit. No red/deviation was erased and no outcome/deadline/product guard was relaxed.

| Retained deciding log | SHA256 |
| --- | --- |
| seccomp-settlement-1.log | 0f2f5d5ee1c784a04fd6b0fa37401eeb2d0bfae721d4660eaff9151e2181ce93 |
| seccomp-diagnostic-1.log | ac38c5b090a43bf21c82e29aedb801cb01cb0f9f16cdd05e28be43ac3af3b890 |
| seccomp-matrix-final.log, restoration-assumption red | 6cc69e8e8bc49df54776e13f1b9c2bd03561c2f61108cd8e2181bccf3f069c47 |
| seccomp-revoked-final.log, sequencing deviation | 2d719d09be83f72590791025ef7b4896a365de42f30f9ace6e4a3e9d3901f80a |
| seccomp-restoration-matrix-1.log, final green | 9b037a6ae9c220a8c40a455ae7f0d29bf0b545d7a90fe39113305724a21c8983 |
| seccomp-restoration-revoked-1.log, final green | a733c8f7f8bde5598e8936577024de942784154fc589857a979baf58e8a66eb8 |

## Cleanup, prerequisites and remaining review

Disarm precedes owner shutdown. Exact task-owned owner/child pidfds establish exit while listeners remain alive serving CONTINUE; verified owner.lock reacquisition follows exit. Observer shutdown joins; monitors join before listener descriptors close. Notification-seen tracees retain stable pidfds for bounded fallback cleanup. Final runs required no forced cleanup, and post-run process-name/path checks found no owned CLI, custody or D-Bus processes. Task TMPDIR has no files; the empty private root is retained for coordinator review. Default target1.5GiB retained; last free space17GiB, above8GiB reserve. No unrelated cleanup was performed.

The final two fault cases require Linux x86_64, actions_avail user_notif and successful GET_NOTIF_SIZES with exact80/24/64 notification/response/data sizes. Runtime NO_NEW_PRIVS/NEW_LISTENER installation, SCM transfer and procfs identity visibility must succeed explicitly; refusal is failure, not skipped success. Existing qualified GNOME/D-Bus, production binaries, task-owned TMPDIR, provider fixtures and SO_PEERPIDFD requirements remain. Root owns shared runner classification for those two exact tests. Unsupported-host/refused-installation behavior is explicit in source but was not independently exercised on an unsupported kernel or denied policy in this run.

No general seccomp security guarantee, real remote GitLab sandbox acceptance, repository-wide task check, ESS regeneration, full ignored-family run or publication is claimed. Whole-candidate adversarial review and root integration gates remain outstanding. Source freeze was sent before report assembly; only this publication-safe report and scratch identity records were written afterward.
