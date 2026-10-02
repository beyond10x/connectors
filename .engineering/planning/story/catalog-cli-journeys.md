---
format: aep.planning-md/3
id: story:catalog-cli-journeys
kind: story
status: draft
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
revision: 5
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
