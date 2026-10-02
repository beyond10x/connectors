unit: story:catalog-cli-journeys — rev13 fault-cleanup correction
verdict: green implementation checks; corrected-candidate review and integrated gate pending
cases: boundary/cache2passed; affected journeys2passed; ordinary334passed/0failed/21ignored; historical10logical/15variants unchanged
origin: introduced cleanup blocker corrected; no production change
wrote-outside-worktree: $HOME/.cache/c26d/cat; $HOME/.cache/sccache; $HOME/.cargo; managed worktree lease registry
needs-coordinator: yes — focused adversary re-review, integrated gate, shared classification and publication

This is a publication-safe final addendum. The sole path redaction is replacing the private home prefix with literal `$HOME`; raw logs and manifests remain unchanged in private scratch. No raw predecessor report is overwritten. The full original acceptance report `report-final-public.md`, SHA256ef4ca397d129fa98b021f289dcc07d13502ba213f75fcdde599a6a6ce63efa3c, remains historical evidence; its source/count identities are superseded by this correction. Its earlier failed injections, unknown-cause recovery red and disclosed dependent-launch deviation remain retained exactly as described, not erased by a later green.

Tree `$HOME/.local/state/worktree/trees/b10x/connectors/cb26d-catalog`, basef3fb222b7edc7fc29520dd30effdb58bfdec5274. Root owns AEP, dependency/lock/classifier changes and Git/publication. All additions are Rust tests. No production, dependency, guard, authority, deadline or native-response changes occurred.

## Reviewed blocker and bounded fix

Whole-candidate source review `adversary-final/report.md`, SHA256feb48e9c79a8bdd79c1a28eb9a600fa840105cb16fad91fabce7bd8950a50653, identified one introduced blocker: list-derived numeric child PIDs could become pidfds for a replacement process, and the new fault cleanup treated those handles as authority to send SIGKILL. A stable pidfd does not establish ownership at acquisition. The reviewer performed no signal experiment or PID-reuse reproduction.

The corrected fault cleanup handles its two provenance classes separately. Only the owner slot obtained from SO_PEERPIDFD can authorize forced retirement there. Child-list handles are polled only; if exit is not observed, cleanup reports a fixture failure and never signals through them. While unwinding, that failure returns a diagnostic instead of a second panic; the fault-only Cli shutdown branch returns so observer and notification-listener cleanup remains reachable. Notification-proven handles remain a separate fallback whose capture requires a still-valid blocked notification from the inherited filter.

The shared owner_handles/owner_process/kill_owner/wait_owner acquisition and crash semantics are unchanged. Accordingly no new modes0–3/6 rerun scope was introduced. The cleanup correction touches only guarded_merge.rs's fault cleanup/new control, cli_journey.rs's fault-only cleanup failure return, and settlement_fault.rs's notification cache/new control. Five other source files are unchanged from the prior frozen candidate.

The notification cache now polls its existing pidfd before accepting a numeric Tgid cache hit. A live entry remains; an exited entry is removed, after which replacement capture still opens a new pidfd and validates the live notification ID. Poll errors refuse observation. No notification validity check was removed and no numeric-PID signalling was introduced.

The pre-fix eight source files are preserved under `cleanup-fix-before/`, alongside `sources.sha256` (manifest SHAb6921bb5a6b39f83b5213f302355ad65ae7c59371fe21ec824465e394c4dc132). They were verified against the prior frozen source manifest before editing.

## Constructed red/green controls

`cli_journey::guarded_merge::catalog_fault_cleanup_observation_handle_cannot_authorize_signal` is a **constructed observation-handle boundary control**, not a reproduction of kernel PID reuse. It creates and retains two direct children itself. The second child is not owned by the selected owner; its handle is supplied in the observation-only slot. Every process is test-owned and reaped. No unrelated process was targeted and no attempt was made to wrap/reuse kernel PIDs.

Before the fix, the actual old cleanup signalled that supplied observation handle. The sibling exited SIGKILL/status9 and the test failed with `observation-only pidfd authorized a signal to a test-owned sibling`. Raw cleanup-negative-red.log: exit101,0passed/1failed/68filtered,5.01s. This is the scoped red showing the unsafe boundary, not a claim to have reproduced the acquisition race.

After the fix, the same sibling remained alive (`status=None`), cleanup refused because exit was not observed, and the test then killed/reaped its own child through its independently held Child ownership. A second portion supplies an owned direct-child stand-in in the strongly identified owner slot, confirms forced retirement still works, reaps it and retains the cleanup refusal. The actual runtime's owner-slot provenance remains SO_PEERPIDFD; the constructed control does not pretend its stand-in came from a production owner socket.

`cli_journey::settlement_fault::catalog_fault_notification_cache_expires_an_exited_handle` creates one real owned child, confirms its live pidfd is retained, kills/reaps through its Child ownership, then confirms the cached exited handle is removed. This establishes live/exited cache behavior, not actual numeric Tgid reuse or PID wrap. The still-valid-notification replacement acquisition remains source-checked and exercised by normal filtered startup, not a fabricated notification.

Combined controls cleanup-controls-green.log: exit0,2passed/0failed/0ignored/68filtered,10.01s. Expected cleanup refusals are caught by the constructed control and appear as panic-hook text; the runner's final result is green. Actual failure-during-unwind was not independently forced by this control; the non-double-panic path is source-level evidence. No claim of exhaustive failure cleanup is made.

## Final affected execution and counts

All commands ran in the managed tree with CARGO_BUILD_JOBS=2, RUSTC_WRAPPER=/usr/bin/sccache, TMPDIR=`$HOME/.cache/c26d/cat`, private default target, release and --locked. The two existing ignored fault selections retain their previous prerequisites. The two new ordinary controls use `/usr/bin/sleep` and Linux pidfd polling; they do not install a seccomp filter or contact a provider.

| Run | Command/case | Actual result |
| --- | --- | --- |
| Constructed red | cargo test --release --locked -p connectors-catalog-provider --test local_runtime, exact observation-handle control, --nocapture | exit101;0passed/1failed/68filtered;5.01s |
| Controls green | cargo test --release --locked -p connectors-catalog-provider --test local_runtime catalog_fault_ -- --nocapture | exit0;2passed/0failed/68filtered;10.01s |
| Fault matrix | cli_journey::guarded_merge::catalog_cli_failed_settlement_keeps_the_known_effect_and_recovers_conservatively | exit0;1passed/0failed/0ignored/69filtered;53.11s;control12,8,9,10 passed |
| Revoked adversary | cli_journey::guarded_merge::catalog_adversary_revoked_disclosure_of_an_applied_merge_never_answers_not_attempted | exit0;1passed/0failed/0ignored/69filtered;12.62s;mode11 passed |
| Ordinary package | cargo test --release --locked -p connectors-catalog-provider | exit0;44summaries,334passed/0failed/21ignored |
| Clippy | cargo clippy --release --locked -p connectors-catalog-provider --all-targets -- -D warnings | exit0;2.41s |
| Format / whitespace | rustfmt --edition2024 --check over eight authored files; git diff --check | exit0;both logs empty |

Live cases use `CONNECTORS_TEST_CLI=<tree>/target/release/connectors <tree>/target/release/deps/local_runtime-9b8f73d2db6f0a0a --ignored --exact <case> --nocapture`. The matrix exit0 was explicitly inspected before launching separate11. No repeat of the earlier dependent-launch deviation occurred.

Current8/9 observed Dispatching/Pending after exact replacement-owner exit, then genuine pending recovery through a different new owner. All same-original request/attempt, unknown/replayed=true, complete audit, Indeterminate/Quarantined, absent settlement/expiry, resumed production metadata I/O and no extra PUT/effect assertions passed. Earlier retained runs measured the other accepted quiescent terminal/passive replay branch. Original red cause remains unknown; no later run is retroactively substituted for its missing observations.

Modes10/11 passed their original revoked/non-disclosure/no-second-effect assertions and the explicit **fixture read-only fsync restoration** control. That control is not production recovery and does not claim revoked replay writes. All armed cases retain actual exact-file EIO evidence. Normal owner/observation-only child exits, verified lifetime-lock reacquisition, observer joins and listener joins passed.

Snapshots remain precisely labeled: **all typed AttemptRecord/KeyReservation/AuditRecord/ApprovalRedemption terminal rows from a validated CompleteSnapshot capture**, not an archived full capture or event history. Whole-capture completeness and typed identity checks precede the serialized subset. Test-only references/approval subjects/fictional outcomes are retained, not credential/private-key/proof bytes. No large notification arrays are reproduced in this report.

Inventory is now70 tests:49 ordinary and21 ignored. The two added controls are ordinary safety cases, not extra historical journeys. Original baseline local-runtime evidence48 unique; consolidated final49 ordinary+10selected journeys=59 unique with passing evidence. Full-package334 ordinary+10selected=344 unique with passing evidence. Original historical scope stays10logical obligations/15variants, including3lifecycle variants and mutation modes0–11. Control12, repeated runs, safety controls and the two existing excluded helpers do not inflate that scope.

Evidence-reuse limit: lifecycle and modes0–7 retain the earlier green runs documented in report-final-public.md. They were not rerun against this final executable. The correction leaves their shared capture/crash semantics intact and is conditional on fault-enabled paths; the whole-candidate reviewer explicitly identified these as the affected selections for a fault-only correction. This is a source-grounded reuse argument, not an all15-on-one-final-binary execution claim. Earliest lifecycle evidence predates prior fixture refinements as already disclosed. Root owns final review/gate disposition.

## Complete authored scope and fixed identities

Total author-owned delta against base is **3,593 additions/10 deletions across eight files**. Tracked diff has local_runtime.rs +89/-2 and cli_journey.rs +129/-8. Six untracked modules add3375lines: background_recovery257,guarded_merge1381,lifecycle313,owner_replay161,recorded_state468,settlement_fault795. This includes every untracked source; plain git diff alone undercounts scope.

| Source under adapters/catalog/tests/ | SHA256 |
| --- | --- |
| local_runtime.rs | d471f3ec4add0ff99a760080a9fb7138217d0092349164d6cda2e98a8d8bad71 |
| local_runtime/cli_journey.rs | cb330065dd81afd611fa33648c2cc47ae5d74774ebaf55839a7e607a0db8a374 |
| local_runtime/guarded_merge.rs | c660af0d0e526c655076de71e3ca20267962d71cb2ecfa25e8fce93a60bb4a62 |
| local_runtime/background_recovery.rs | 06e6318eb2b8700e8690e466be9f5e6bbae261eb3c68f7c6fa9d2422ae4dff62 |
| local_runtime/lifecycle.rs | 3fceddf2a40fb76784cfd77a7d0cbd4c674cee86a3cba2c35fb84567b8f94deb |
| local_runtime/owner_replay.rs | 1ddcf26de557b001278361a81713b7f4da089ccc7e68a72e89d50d9df6f222c2 |
| local_runtime/recorded_state.rs | 9034be65cb8d754d4284dafd2be7bd8431435d1e9163960a30d288b1b5643765 |
| local_runtime/settlement_fault.rs | d992bd64bb920da5bf7731f1ccd0756072e7341651c7e972aa00c4418ad7495a |

Manifest cleanup-final-source.sha256 SHAb545c005ffa28b7dbd19605909d388dde3fd843a02dcd805509ab5137b625c8f; cleanup-final-binaries.sha256 SHA84b609ea5284acf24eeef09fcc74b2cc91a04801536d1733640e3787361ab8de. Production CLI1bfc86834f96f2d61ad4e7842005e6d25bf615203b5dd82806055ad56d15f4a7 and catalog provider46a2886b00e72846d8a15fb69f19ce761db34406e1002b3d7765629a19c63c6f are unchanged. Final test executable1e91b30d108ff1e98ef990c720a173b0fdc2ba2edcb1a5d2649cee727e9a52e4.

| Evidence log | SHA256 |
| --- | --- |
| cleanup-negative-red.log | 9bdbf85aae206744088bc92c432d836d8efc0c89bfbb350cf4440944ff46ac7d |
| cleanup-controls-green.log | 7fcdc5ed6b4a536198297ea60ad8e18363b090116c7827d70532b03930cbfb63 |
| cleanup-matrix-green.log | ceecae237163ae847f49052e559c5db4d75200df9fa4ca81a25686beac1fa10e |
| cleanup-revoked-green.log | 0141a2c73f9b7dd17ad13a31582a8f594fe0b52477b5def6979fe43c03bc4456 |

Source freeze was sent immediately after final checks; only report/evidence assembly follows. Own processes were reaped, owned runtime process check was empty, and private TMPDIR has no fixture files. Target1.5GiB retained; last free17GiB exceeds8GiB reserve. No unrelated cleanup. Own lease is released at handoff. A corrected-candidate reviewer and root integrated gate remain required; no repository-wide gate, publication, security-boundary proof, kernel PID-reuse reproduction or unsupported-kernel experiment is claimed.
