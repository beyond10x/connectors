---
format: aep.planning-md/3
id: story:catalog-cli-journeys
kind: story
status: implemented
title: Re-run the production CLI mutation journeys with the catalog provider as the child
relations:
- decomposes: initiative:complete-local-connectors
- derived_from: epic:retire-native-gitlab-adapter
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: adapters/catalog/Cargo.toml
- confidence: cited
  path: adapters/catalog/tests/local_runtime
- confidence: cited
  path: adapters/catalog/tests/local_runtime.rs
- confidence: inferred
  path: crates/connectors-build/src/ignored.rs
- confidence: cited
  path: docs/evidence/catalog-cli-20261002
revision: 15
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T15:17:51Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":4}}, correlation: "wave-20261002d-provider-acceptance"}
- {from: "proposed", to: "active", at: "2026-10-02T15:17:51Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":4}}, correlation: "wave-20261002d-provider-acceptance"}
- {from: "active", to: "implemented", at: "2026-10-02T18:55:40Z", actor: "human:timo", revision: 15, decided_on: {"recorded":{"test_result":3,"review_outcome":7}}}
---
## Outcome

The production CLI journeys that proved approval spend, mutation settlement, lost-response handling, owner crash with same-key observation, background recovery and post-effect revocation run with the catalog provider as the child. Retain the actual records as evidence.

## Reconciled baseline — 2026-10-02

Native reference is 8dac09e. Current catalog local_runtime.rs wires seven modules, including an existing production GitLab CLI restart journey and Basic/OAuth journeys. Augment gitlab_catalog_cli_reuses_custody_across_owner_and_keyring_restart instead of duplicating it: concurrent one-owner reuse, exact probe/read counts, stale stop, suppression/restart and refused invoke after revoke are not yet all asserted. Add the other nine logical cases. Historical cli_journey.rs has three lifecycle tests; guarded_merge.rs has seven mutation/recovery tests plus a subprocess helper; background_recovery.rs contains helpers only. Preserve ten logical obligations and fifteen historical variants. The prior initial scope refresh overlooked module declarations; this audited scope corrects it.

## Acceptance

All ten named obligations in the mapping below pass through a built production CLI, qualified disposable custody and the shipped catalog selection, with every variant preserving exact effect/dispatch counts, approval spend, ledger/audit transitions, refusal codes or explicitly documented current-contract replacements, and no duplicate uncertain effect. Actual case outputs, source/binary identities and commands are retained under docs/evidence/catalog-cli-20261002/. A helper is not a passing journey. Dedicated real GitLab sandbox acceptance remains a separate initiative requirement.

## Scope

Derived 2026-10-02 by `story-scoper`. Every item is cited or inferred.

- **Primary surface:** `adapters/catalog/tests/local_runtime/` — cited; the existing production CLI/custody fixture is here, and the story assigns the missing lifecycle and mutation ports here.
- **Files:** `adapters/catalog/tests/local_runtime.rs` — cited; owns the existing TLS Provider, child bootstrap and test-module wiring. Extend its fixture or wire a narrowly isolated mutation fixture.
- **Files:** `adapters/catalog/tests/local_runtime/cli_journey.rs` — cited; reuse Custody, Cli, configure, success/refusal and augment its existing GitLab restart case instead of duplicating it.
- **Files:** new `adapters/catalog/tests/local_runtime/guarded_merge.rs` and `background_recovery.rs` — cited; named by the story's historical source and port destination. The historical first file owns seven top-level mutation/recovery tests and a subprocess helper; the second owns recovery helpers, not five top-level tests.
- **Also likely:** `adapters/catalog/Cargo.toml` and `Cargo.lock` — inferred; pidfd, signing-clock and current recorded-state fixture dependencies need dev dependencies not presently declared by this crate (libc, ring, and possibly rusqlite/ER reader dependencies). Existing base64 is already a normal dependency.
- **Also likely:** `crates/connectors-build/src/ignored.rs` — inferred; classify exact new libtest identifiers after the runner lands, including the helper exclusion and the real-expiry timing case.
- **Documents:** `docs/evidence/catalog-cli-20261002/` — cited; actual commands, per-case results and current-contract replacements belong here after execution. This audit itself is not passing evidence.
- **Confidence:** medium — inferred; test/fixture ownership is direct, but ER inspection and settlement-failure injection must be proven before promising a mechanical port.
- **Would collide with:** catalog local-runtime tests and their shared fixture, catalog dev dependencies/Cargo.lock, or the ignored-suite classifier — inferred; run the classifier edit serially after the current runner unit.
- **Safety fact:** the production catalog child already uses runtime::PreparedWrite and the host's existing approval/mutation path; this port can exercise that path without adding provider operations or shared entities — cited at `adapters/catalog/src/local.rs:1067` and `adapters/catalog/src/lib.rs:775`; proof level 2, unproven at runtime in this audit.
## Required adaptations

1. **Identity/configuration.** Keep provider `gitlab`, instance `fixture-gitlab`, auth profile `gitlab.pat`, identity probe `/api/v4/user` and scope probe `/api/v4/personal_access_tokens/self`; the child adapter identity is `catalog`, binary `connectors-catalog-provider`, local provider configuration `connectors-catalog-local/2`. Existing alias is `gitlab`; either consistently use it or deliberately configure `forge`, but never confuse alias with adapter identity. Use the printed bootstrap and its actual revision. The prepared-attempt helper hardcodes adapter `gitlab` at the native reference: change it to the actual `catalog` fingerprint or its seeded record will not be the request being observed. Removed-target recovery must bootstrap another catalog config/instance, not the retired binary.

2. **Write admission.** Existing catalog Cli::configure uses local format 1, private protocol 1 by default, and only project.get/issues.list. Mutation fixtures need the existing local format 2 and explicit connectors-private/2, permitted merge_request.merge and approval key/policy. Resolve live descriptors before constructing/issuing exact subjects. Do not relax approval spend or same-key admission.

3. **Inputs/results.** Native project input `{project: ...}` becomes `{id: ...}`. Merge input becomes `{id: "org/project", merge_request_iid: 4, body: {sha: <pin>}, pipeline_id: 12}` (plus explicit existing optional provider flags only if intended). Pipeline id is a guard-only input, not a PUT body member. Catalog does not insert native default `auto_merge:false` or `should_remove_source_branch:false`; assert the exact body submitted. Raw result assertions move from `item` to `body` inside the parsed CLI result. This is envelope adaptation, not deletion of business assertions.

4. **Shipped guard.** Load `adapters/catalog/providers/gitlab/operations.json`, not a fixture-selected weaker guard. Serve the MR preflight with matching sha, opened, mergeable, matching head_pipeline.id and success; serve merged/matching sha after the one PUT. Route and method counters must distinguish identity/scope reads, preflight and effect dispatch. Existing Provider reads headers only and lacks method/body/effect/held-response bookkeeping; add bounded Content-Length parsing and explicit latched barriers. Retain the historical held-refusal latch: release must not turn the selected refusal response into success. No CI paging/trace/native validation ports are required.

5. **Refusal replacement must be explicit.** Native modes 2/9 return HTTP409 with no effect and assert forbidden. Current catalog Prepared::execute (`lib.rs:787-809`) maps 409 to invalid_input and Refused; contracts/catalog §3 distinguishes generic read status mapping from mutation effect proof. Preserve the fixture's proven zero effect and HTTP409, and use a named assertion such as `catalog_merge_409_is_refused_invalid_input_without_effect` in the relevant journey/report. Do not change the fixture to403 just to retain an old expected string; do not interpret 409 as idempotency_conflict. Approval_replayed, approval_required, revoked, outcome_unknown, interrupted and idempotency_conflict remain host-owned assertions.

6. **ER is the largest port risk.** Historical raw SQL helpers read mutation_attempts, mutation_keys and execution_audits directly from metadata.sqlite3. Those physical rows are now immutable imported anchors; current compatibility tables live in a private in-memory projection (`docs/local-er-metadata.md:30`, Metadata::activate_er). Copying raw queries can produce stale or absent evidence. Use `mutations::Store::{lookup,observe}` for exact attempt/request state and timestamps: Observation includes reservation_id, state, result, settled_at_ms and replay_expires_at_ms. Seed helpers can retain/emit exact AttemptRef before exit. `audit::Store::observe` reads a known exact audit reference. Exact audit enumeration before a held response and key lifecycle state require a bounded test-owned recorded-ER reader or a separately reviewed inspection seam; public stores do not expose those enumerations. Do not add production diagnostic exports or a second state authority merely for these tests. Re-scope with the coordinator if test-only reading cannot retain every old assertion.

7. **Settlement injection.** Historical modes 8–11 chmod every metadata.sqlite3* file to0400 after provider has latched its known response, then restore0600. It might still refuse fresh Metadata::update opens, but pooled Eventlog handles differ from the native baseline. Prove the fault occurs after effect and prevents terminal persistence, with pre-fault committed spend/dispatch and no accidental earlier failure. The current host's cfg(test) fault hooks are unavailable to a separately built production CLI. If chmod is insufficient, report the precise missing fixture seam; do not drop the failed-settlement cases or edit production host code under this unit silently.

8. **Process/time/custody.** Keep exact Linux SO_PEERPIDFD ownership for owner/child crashes, fixture-only private bus/keyring and short physical task TMPDIR. Reuse the independent signing-clock encoder at crates/connectors-host/tests/fixtures/clock/server.rs. Keep the real 60-second expiry test; classify it Timing rather than pretending it is a fast disposable test. Discovery counts and approval deadlines may expose known ER cost; do not extend production admission deadlines to make fixtures pass. If unrelated elapsed time threatens credential validity, explicitly revalidate before the critical window, preserving the post-expiry no-dispatch test.

## Smallest bounded implementation order and acceptance mapping

Use one implementation unit with serial internal checkpoints; shared TLS/CLI/ledger fixture state makes concurrent authorship here a poor speedup. All executable additions are Rust; any new CLI uses clap derive. Scope source tests and fixture dependencies only; production defects return for separate review.

1. Reuse/augment existing restart test, then add two remaining lifecycle tests. Run these before porting the write matrix so configuration/custody differences are isolated.
2. Add the exact shipped merge fixture, signing clock, current ER observation strategy and subprocess producer. Run modes 0/2 first, then lost response/owner crash. Do not move into recovery cases until real ledger and audit observations are demonstrated.
3. Port modes 4–11 preserving each assertion and variant. Record every current-contract replacement beside its source assertion. Add runner classifications once names settle; collect actual ignored inventory and execute explicitly selected families with prerequisites required.
4. Record ten logical obligations/15 variants as passed/failed individually, exact CLI/provider identities, command lines, counters, and scope limits. Run catalog tests and affected conformance/gate; the dedicated GitLab live sandbox remains separate initiative acceptance.

Suggested acceptance test mapping (names proposed, not authored):

| Historical obligation | Smallest catalog acceptance | Assertions that must remain |
| --- | --- | --- |
| persistent_gitlab_cli_owner_and_keyring_restart | augment existing gitlab_catalog_cli_reuses_custody_across_owner_and_keyring_restart | Four concurrent CLI reads reuse one new owner/custody; exact provider probes/reads; stale stop refused, suppression and restart; revoke blocks dispatch without new owner. Explicit revalidation adds exactly its two probes and preserves revision. |
| gitlab_cli_failed_repair_and_busy_stop_preserve_authority | catalog_cli_failed_repair_and_busy_stop_preserve_authority | Protected-file refusal before provider; stdin connect; wrong-subject repair preserves authority; duplicate/stale/forbidden input no effect; busy stop bounded; capture invalidated; custody unavailable leaves owner absent; no secret persistence. |
| gitlab_cli_explicit_revalidation_after_real_expiry_without_reentry | catalog_cli_explicit_revalidation_after_real_expiry_without_reentry | Actual expiry pending/not_granted no provider/no owner; saved-custody revalidation two probes, same revision;503 does not invalidate;401 does; retry after invalidity no send. |
| gitlab_cli_guarded_merge_applied_refused_and_lost_response_restart | catalog_cli_guarded_merge_applied_refused_and_lost_response_restart | Modes0–3: applied/unknown/refused/owner-crash unknown; one PUT and0or1 effect; missing/spent proof refuses; exact original attempt/request; same key replays without proof/custody/clock/provider; changed pipeline id conflicts. Preserve HTTP409 replacement above. |
| gitlab_cli_guarded_merge_revocation_finishes_admitted_audit | catalog_cli_guarded_merge_revocation_finishes_admitted_audit | Revoke while preflight held; zero PUT/effects; one admitted audit retains reference and finishes refused/revoked. |
| gitlab_cli_recovers_abandoned_preparation_only_with_trusted_time | catalog_cli_recovers_abandoned_preparation_only_with_trusted_time | Seed prepared/pending through real durable port; unavailable clock leaves prepared; trusted time yields aborted/replayable and 86,400,000ms retention; settled replay no owner/clock/provider. |
| gitlab_cli_background_recovery_waits_for_live_work_and_recovers_unkeyed_attempts | catalog_cli_background_recovery_waits_for_live_work_and_recovers_unkeyed_attempts | Live dispatched keyed attempt and distinct prepared unkeyed attempt coexist; maintenance does not steal live work; restart produces indeterminate/quarantined, then trusted-time abort for unkeyed; one PUT/effect, no recovery send. |
| gitlab_cli_background_recovers_revoked_removed_target_without_disclosure | catalog_cli_background_recovers_revoked_removed_target_without_disclosure | Removed/revoked target recovered by owner started on other catalog instance; no provider traffic or effect; unavailable clock waits; trusted clock aborts; restored config still refuses disclosure without mutation projection. |
| gitlab_cli_failed_settlement_keeps_the_known_effect_and_recovers_conservatively | catalog_cli_failed_settlement_keeps_the_known_effect_and_recovers_conservatively | Modes8–10: known applied/refused survives settlement failure with safe attempt_store cause/incomplete audit; durable dispatching/pending then quarantined unknown after restart; spend stays spent; revoked disclosure withheld; one PUT, exact0or1 effects. |
| adversary_revoked_disclosure_of_an_applied_merge_never_answers_not_attempted | catalog_adversary_revoked_disclosure_of_an_applied_merge_never_answers_not_attempted | Mode11: known effect1 then revoked admission plus persistence failure; never not_attempted; no extra send; durable uncertain state/key retained without settlement/expiry; refused replay starts no owner. |

Keep prepared_attempt_exit_fixture as an explicitly classified helper (expected exit73), not a passed top-level journey. Maintain parent invocation's exact fully qualified helper name after module moves.

## What could not be established

- No runtime evidence was gathered: all ten logical obligations remain unaccepted by this audit, including the existing restart case's missing assertions.
- Exact current ER audit enumeration/key-state observation and chmod fault behavior are not yet proven; they are the first implementation feasibility checks, not permission to relax ledger assertions.
- Full wall-clock time and credential-evidence expiry under current ER cost are unmeasured for these ports.
- No new product semantic decision or entity is necessary for the identified mechanical adaptations. Adding an inspection API, changing authority/replay/guard semantics, or changing persistent records would exceed this conclusion and must return for ESS/contract review before implementation.
- GitLab sandbox acceptance is not established by deterministic fixtures and is deliberately outside this one unit.

## Current ER fixture seam — 2026-10-02

A read-only source audit of the pinned ER public APIs identified a bounded,
test-owned observer. This is feasibility from source, not runtime proof. Add a
narrow `adapters/catalog/tests/local_runtime/recorded_state.rs` helper only to this
integration-test target. Use host Metadata::inspect for existing admission checks,
read only the immutable authority marker from physical SQLite, then open that
exact existing authority through RecordedProviderFacade. Load the exact production
entity definition JSON; never provision/import/copy an authority or query legacy
business tables. The facade performs observational reads but its existing-store
attachment can use a write-intent validation transaction; do not claim it is an
OS-read-only database handle or hold the host lifecycle lock across barriers.

Capture complete_snapshot anew per observation, with finite explicit limits and
waits, requiring completeness and exact identity. Join AuditRecord, AttemptRecord
and KeyReservation terminal facts from the same capture. Preserve literal key
states and absent settlement/expiry, exact before/held audit-set differences,
request/attempt/reservation/connection/operation identities, and checked RFC3339
millisecond conversion. Cross-check public Store observations at quiescent stages;
those are separate reads, not an atomic combined snapshot. Keep the observer open
before fault windows, expose no write method, and require joined shutdown before
fixture cleanup. Retain exact seeded AttemptRef before helper exit73. Unknown
fields/states, partial captures, missing/duplicate rows or capacity refusal fail.

Dev-only dependencies may reuse the already pinned workspace entity-core,
entity-eventlog, entity-store, eventlog-core and time, plus host-aligned rusqlite
0.40.2/bundled, libc0.2 and ring0.17. Add uuid only if actually imported. Root
serializes Cargo.lock and ignored-runner integration. No new upstream version,
production export or new authority is selected.

The historical chmod0400 fault is not established: host privacy checks permit it,
SQLite READ_WRITE may fall back to read-only, and pooled Eventlog handles can keep
write access. First prove a paired unchanged-permission control and one controlled
fault case using identical production binaries and a fixed held response. Prove
committed dispatch/spend and exact0or1provider effect before injection, then require
the exact known-outcome/attempt_store/incomplete-audit response and fresh durable
Dispatching/Pending state before restoration/recovery. The observer must not cause
an earlier refusal or stand in for settlement failure. If chmod allows settlement,
record that injection as rejected and return for a separately scoped fixture seam;
do not broaden filesystem damage or drop the historical modes8–11 assertions.

## Current executable admission and replay fixture — 2026-10-02

The first expanded repair case failed with OwnerBuildMismatch because the libtest
Client::begin called a production-CLI owner. Current transport.rs:351-358,487-492,
519-536 requires the caller and owner to have the same measured executable image.
This is intentional, and apps/connectors/tests/owner_build_security.rs protects it.
The retained failure is not a production defect.

Capture invalidation remains production-CLI acceptance: hold protected stdin after
connections connect has admitted Begin (apps/connectors/src/local/session.rs:177-201),
observe the exact new Pending Acquisition through the existing ER observer, stop the
busy child, then supply the fixture credential. Require lifecycle_conflict and no
new provider send. No fabricated capture or build digest is allowed.

Settled public CLI retries return locally before WriteClient
(apps/connectors/src/local/operations.rs:123-151). Keep those passive controls and
label them accurately. Preserve the separate successful owner/2 replay obligation
using the existing catalog fixture and a same-image host-library helper. This is a
sequential replacement of the invalid cross-image test seam within the same owned
test files, not a new runtime feature or removal of an acceptance obligation.

After original settlement through the production CLI/catalog child, stop and wait
for that exact owner. Spawn the same libtest executable with one exact ignored
helper that calls existing unmodified public owner::serve. Pass a real startup
UnixStream on FD3 and the exact verified private owner.lock lifetime file on FD4,
following production descriptor duplication, flock and isolated process rules.
The libtest parent WriteClient and helper naturally share actual /proc/self/exe
bytes; never forge a greeting/digest, change same_build, construct a fake Owner,
add a production test hook, or run fixed-FD mutation in a shared test thread.
Use start=false; no production-only hidden-command fallback through libtest.

Label this segment same-build host-library transport acceptance, distinct from the
production-CLI original effect. First measure successful handshake, exact owner
incarnation and bounded shutdown/lock release. Then prove one applied settled
replay before refusal/lost-response variants. Preserve all original request/attempt,
exact delivery/refusal, key, spent proof, clock, provider PUT/effect and audit checks.
A replay has its own correlated final audit; zero clock/provider calls does not
mean zero metadata writes. Stop custody/delete proof before replay, pass no proof,
and observe no new native child. Keep pending mode3 on its genuine public CLI owner
path and preserve all fifteen variants, including independent settlement-fault
feasibility controls. Helper source and deps remain in current test scope; no new
Cargo dependency is justified. Root adds its exact name as an excluded helper and
it never counts as a journey. The helper refuses absent private fixture selection.

Source-grounded feasibility is recorded in the coordinator's assigned
.local/provider-wave-briefs/owner-replay-feasibility/report.md. It is not executed
proof. If actual inherited-FD startup, cleanup or replay assertions fail, retain the
failure and return the unmet obligation without weakening deadlines or authority.

## Settlement fixture correction — 2026-10-02

The frozen first pass is partial: eight of ten logical obligations and eleven of
fifteen historical variants passed. Unchanged-permission control12 passed; chmod
mode8 failed because current Entity Runtime successfully settled despite0400.
Modes9–11 were not executed and remain unmet. This rejects the injection, not the
product. Ordinary package332 passed/0 failed/21 ignored; final Clippy/fmt passed.
Original author report SHA256445aca6ac928d61e9064fc54f59d077ae95f67528b35217dd378008069dcadb9;
deciding merge-settlement-1.log SHA2569269829524f6ba8ad61601aac25cbb7c5e3c77ec1371a0d3fb62ffebc44e046f.
Pre-held typed assertions passed, but no complete pre-snapshot was serialized;
do not imply otherwise. Preserve this red and all original evidence.

A separate single throwaway Rust seccomp USER_NOTIF smoke measured genuine writes
through an already-open descriptor: write/fsync succeeded before arm, both returned
EIO after arm, unrelated writes succeeded, and target writes resumed after disarm.
The filter survived exec; child exited0 and listener/control closed. This establishes
kernel feasibility only, not interception of actual ER settlement writes.

Authorize one bounded test-only replacement inside the existing catalog integration
test scope, with a narrow settlement_fault.rs helper if useful. No production hook,
new dependency, file corruption, descriptor theft, upstream change or relaxed
assertion. Install the filter only on the fixture-owned production CLI before exec;
its genuine owner descendants inherit it. Service the listener in the unfiltered
test process before spawn can block on the exec-error pipe. Transfer the listener
through SCM_RIGHTS with validated ancillary data and close every descriptor.

Before arming, serialize complete typed before/held snapshots and prove exact
Dispatching/Pending attempt, Spent proof, incomplete audit and one native effect
(or exact refusal/no effect). At arm identify only the assigned metadata.sqlite3,
its WAL/journal siblings by exact canonical parent/name plus device/inode. Return
EIO only for observed write/sync/truncate syscalls on those exact files; continue
unrelated syscalls. Record syscall, target identity and denial counts without
credentials. This is a controlled test fixture, not a security boundary; account
for fd identity races and unknown/closed descriptors explicitly.

First run the unchanged-fault control, then one known-Applied mode8. Require actual
denied ER I/O and unchanged original known-outcome/attempt_store/incomplete-audit
assertions, plus serialized fresh durable Dispatching/Pending state. Reject a
normal settlement or a different failure; do not tune expected outcomes to pass.
Only after this pair passes execute9–11 with all historical assertions. Disarm
before recovery and prove actual resumed writes/recovery. Retain monitor until
the exact owner has shut down/exited, since it setsid and outlives the CLI process
group. Bounded cleanup must work on panic, join the observer, reap helpers, release
the lifetime lock and leave no owned fixture processes. Unsupported kernel/arch
is an explicit missing prerequisite, never a skipped success.

Stop and return the first ineffective fault, unexplained result or unsafe cleanup;
do not widen damage or deadlines. All fifteen variants and ten obligations remain
required. Root owns shared classifier and AEP. This scoped fixture correction has
one implementation unit, so no multi-item decomposition panel applies. Final
independent adversarial review and repository gate remain required before acceptance.

## Measured injection and bounded recovery diagnostic — 2026-10-02

Filtered control12 passed with zero EIO (3270 target and8972 unrelated CONTINUE).
Mode8 produced four actual pwrite64 EIO replies to its exact metadata.sqlite3-wal
identity, with no monitor errors/cancellations. It preserved known Applied,
Unavailable/AttemptStore, incomplete audit and original Dispatching/Pending with
no settlement/expiry. Source/binary identities remain in worker raw evidence.
The selected case still failed: after disarm, proof-spend refusal and a top-level
outcome_unknown response, the fixture unwrapped owner::Client::connect(false) and
received Unavailable. Remaining response/state assertions did not execute.
No successful recovery or mode8 completion is claimed. Modes9–11 remain unrun.
Raw seccomp-settlement-1.log SHA256
0f2f5d5ee1c784a04fd6b0fa37401eeb2d0bfae721d4660eaff9151e2181ce93.
Cleanup observed graceful captured owner/child exits and joined observer/monitors.

Authorize one diagnostic rerun of the existing control12/mode8 pair, changing only
retained observations before the unchanged failing assertion: serialize the full
recovery response, exact original typed attempt/key/audit/redemption rows, socket/
incarnation observations where naturally reachable, and monitor counters. No new
provider request, altered deadline, forced owner start, relaxed assertion or mode9–11
execution. The competing source paths are passive final replay at
apps/connectors/src/local/operations.rs:121–127 and pending transport error mapping
at155–171; top-level outcome_unknown alone distinguishes neither.

If observation establishes prior background recovery in the real replacement
owner started by the spent-proof control, record that actual incarnation/state
before its shutdown. Do not yet move/delete the failing owner assertion: return
the exact measured distinction for the coordinator's scoped fixture correction.
Final filtered-owner cleanup must additionally prove the exact verified lifetime
lock is reacquirable after observed exit, matching the existing helper invariant.

## Recovery observation correction — 2026-10-02

The one authorized diagnostic control12/mode8 run passed in23.26s with all original
mode8 assertions unchanged. The exact original remained Dispatching/Pending after
the spent-proof control under a different real owner. Following its shutdown,
the retry started another owner and returned unknown/replayed=true/cause=null,
complete audit and matching original attempt/request; durable state became
Indeterminate/Quarantined without timestamps and without another PUT/effect.
Resumed target I/O, captured process exits and lifetime-lock reacquisition passed.
Diagnostic log SHA256ac38c5b090a43bf21c82e29aedb801cb01cb0f9f16cdd05e28be43ac3af3b890.
This does not establish which path produced the earlier missing-owner failure.

Authorize the narrow deterministic fixture correction grounded in both existing
production paths. Record the actual replacement owner created by the spent-proof
control and prove its incarnation differs from the original. After its exact
shutdown/exit, capture the original durable state before final retry. If it is
Dispatching/Pending, require retry to create a different reachable owner as before.
If already Indeterminate/Quarantined, require passive replay with no owner. Refuse
every other state pair; require absent settlement/expiry in either branch. Keep
the existing final original request/attempt, unknown/replayed=true, complete audit,
quarantine, no timestamp, resumed persistence and no duplicate effect assertions
unconditional. No sleep, forced startup, deadline extension or product mutation.
This selects assertions from an observed quiescent precondition, not from whichever
result happens to arrive. Retain both paths and report which actually ran.

After this correction, execute control12 and modes8,9,10, then11 with all retained
historical assertions; stop on first unexplained failure. A mode8 pass is only one
additional historical variant, not completion of the multi-variant obligation.
Finish ordinary tests/Clippy/fmt and freeze for whole-candidate review only after
the required matrix is green. The original red and diagnostic remain immutable.

## Revoked-path restoration control — 2026-10-02

The corrected control12/modes8–10 run passed37.62s and11 passed7.64s. During final
cleanup review the worker removed a panic-while-holding-monitor-mutex risk without
changing product assertions. The justified final repeat passed8/9, including the
pending/new-owner path, but mode10 failed the added assert_resumed: after correct
revoked disclosure, passive retry and exact owner/child exit/lock release, no target
metadata syscall occurred. All earlier mode10 product assertions passed. The
fixture must not require incidental metadata I/O from a revoked passive path.
Retain red log SHA2566cc69e8e8bc49df54776e13f1b9c2bd03561c2f61108cd8e2181bccf3f069c47.

Execution protocol deviation: a tool cell launched11 without conditioning on the
returned10 exit code. That already-started owned run finished green13.77s with clean
exits/lock release; then all execution stopped. Its log SHA256
2d719d09be83f72590791025ef7b4896a365de42f30f9ace6e4a3e9d3901f80a.
Do not describe the sequence as stopping before11. Future dependent launches must
explicitly inspect the previous result before starting the next run.

Authorize one explicit test-fixture restoration control for revoked10/11 after
all original revoked/no-owner/state/effect assertions and exact owner exit. Open
only the existing verified exact metadata file read-only. In a task-owned filtered
child, perform genuine fsync on that preopened descriptor in an async-signal-safe
pre_exec hook after filter installation, then exec the production CLI --version.
The listener service must already be running; require fsync success, child exit0,
a new exact-target CONTINUE notification while disarmed, no owner socket, unchanged
original typed state and provider effect counts. No metadata bytes may be written,
no authority provisioned or provider invoked. Label this fixture fsync restoration,
not production recovery or a write by revoked replay. Keep unrevoked8/9's actual
production recovery/resumed-I/O assertions. Preserve all original product checks.

Run the affected control12/8/9/10 pair first; start11 only after an observed exit0.
Stop and report any new unexplained failure. Then ordinary/Clippy/fmt as warranted
by the final source, freeze, and whole-candidate review. No production, dependency,
deadline, guard or outcome change is authorized.

## Adversary cleanup correction — 2026-10-02

The whole-candidate source review records one introduced blocker: list-derived
numeric child PIDs are converted to pidfds without post-acquisition ownership
validation, and the new fault cleanup then signals those handles. A stable pidfd
does not prove the process was still the owned child when captured. No wrong-process
signal or PID-reuse reproduction occurred. Immutable review:
review-result:catalog-whole-candidate-20261002, report SHA256
feb48e9c79a8bdd79c1a28eb9a600fa840105cb16fad91fabce7bd8950a50653.

Authorize the smallest fault-only correction: preserve list-derived child handles
as observation-only. Never send a signal through them. Keep the strongly identified
SO_PEERPIDFD owner handle separate; only it and handles captured while a live
filtered notification proves identity may authorize forced retirement. Failure to
observe an untrusted/list-derived child's exit is a fixture failure/diagnostic,
not permission to signal it. During unwind avoid a second panic that would bypass
listener/observer cleanup. Keep shared capture/crash semantics unchanged, so this
does not expand the modes0–7 rerun surface.

Add a safe task-owned negative control at this cleanup boundary before the fix:
an observation-only handle to a process not owned by the selected owner must never
be signalled, while proven-owner cleanup remains effective. Every process used by
the control must itself be created and reaped by the test. Retain its red under
the old signal behavior and green after correction. Label it a constructed
observation-handle boundary control, not reproduction of kernel PID reuse. Never
try to force system PID reuse or target unrelated live processes.

The same review identifies a bounded cache limitation: a numeric Tgid key can hold
an exited pidfd and hide a later filtered process that reuses the number. In the
same test-only helper, expire an exited cached handle before capturing a replacement
under a still-valid notification ID; do not weaken notification validation. Use
real task-owned live/exited handles to verify the cache distinction if a narrow
control is feasible; do not claim an actual PID-wrap experiment.

Preserve the frozen pre-fix eight sources and hashes. Change only fault cleanup,
its notification cache and the exact new controls; no production/dependency/guard/
deadline/authority changes. Rerun the two affected fault selections in order,
checking the first exit before the second, and appropriate ordinary/Clippy/fmt.
If shared owner capture/crash behavior changes, stop and return for explicit added
modes0–3 and6 verification. Final reviewer must inspect the corrected candidate;
the lifecycle remains active until that and the integrated gate pass.

## Corrected candidate and release handoff — 2026-10-02

Whole-candidate review found an introduced signal-authority defect in test cleanup.
The author separated strongly owned owner handles from observation-only list-derived
child handles, and expires exited cached notification handles before reacquisition.
The constructed boundary control failed on the original cleanup, then both boundary
and cache controls passed. No real PID-reuse race or unrelated process was targeted.
Shared capture/crash semantics remain unchanged; the two affected fault selections
passed sequentially with checked exits. Final package334passed/0failed/21ignored,
Clippy/fmt/diff-check0. Detailed public evidence is
`docs/evidence/catalog-cli-20261002/cleanup-fix-report.md`, SHA256
a3d6090bd12ec43a84d5ac00e06fd08e2bd9ca25f1d99677f3b7b8fdf933b6cc.
Final source manifest SHA256b545c005ffa28b7dbd19605909d388dde3fd843a02dcd805509ab5137b625c8f.

All ten logical obligations/fifteen historical variants have passing evidence across
retained runs; lifecycle/modes0–7 are not rerun on the final executable. Preserve
this unchanged-relevant-source reuse limit and the actual CLI/host-library seam
split. Correction review and integrated gate remain pending; this update alone
does not close the story. No production/dependency/authority/guard changes.
