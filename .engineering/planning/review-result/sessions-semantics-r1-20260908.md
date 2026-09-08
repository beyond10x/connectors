---
format: aep.planning-md/1
id: review-result:sessions-semantics-r1-20260908
kind: review-result
status: active
title: F06 E20 independent session semantics review, first pass
relations:
- reviews: story:contracts-session-revocation
revision: 1
---
# Independent review A: session revocation semantic hardening

Verdict: **ship with fixes**. Findings: **0 blockers, 1 major, 1 minor**.

Scope: the uncommitted F06/E20 specification hardening based on local commit `6c8ecc2e9b1daa8e5bcac8c8311470356eb59e46`. Reviewed sessions semantics and verification, media semantics, the session-authority capability cross-reference, the media adapter design, the sessions ESS domain/system registration, and all 11 authored session scenarios. The source hashes are in `reviewed-sources.json` beside this report. Read the root AGENTS instructions, the ESS specify skill, the relevant design sections, and the existing session-revocation story. This was independent and read-only: no tracked files, planning artifacts, runtime code or managed worktrees were changed.

## Major R1 — successful authored close examples extend cutoff past existing lease expiry

Locations:

- `contracts/sessions/v1alpha1/scenarios/first-terminal-not-replaced.yaml:62`
- `contracts/sessions/v1alpha1/scenarios/media-overload-shared-reason.yaml:61`
- `contracts/sessions/v1alpha1/scenarios/revocation-refuses-data-and-renewal.yaml:71`
- `contracts/sessions/v1alpha1/scenarios/revocation-beats-renewal.yaml:81`

Each scenario successfully establishes the live lease with effective expiry `12:00:04Z`, with no later successful renewal. The first two successful `BeginClose` decisions nevertheless publish `cutoff_due_at: 12:00:05Z`; the other two publish `12:00:06Z`. These are not adversarial inputs expected to be refused: they are accepted close decisions and emit `ClosingBegun`.

The normative contract requires cutoff at effective expiry without any grace (`contracts/sessions/v1alpha1/semantics.md:102`, `:110`), and the model comments explicitly say earlier expiry/drain deadlines dominate (`ess/domains/sessions.yaml:175`). A later ordinary close ceiling cannot supersede an already earlier deadline. The textual audit therefore currently accepts examples inconsistent with its own chosen rule. ESS compilation does not catch this because the specification explicitly leaves timestamp comparisons unmapped; that disclosure is correct but does not make the example deadlines coherent.

Minimal fix: set each successful close cutoff to the earliest applicable lease expiry, drain deadline and terminal ceiling. In these four examples that is at most `12:00:04Z`. Keep intentionally rejected later close inputs in the first-terminal example clearly identified as adversarial observations. Re-run the explicit session author/synthesis checks after correcting the examples, and audit the resulting supplied deadlines against the prose.

## Minor R2 — the revocation-race example shifts the authority time after observing revocation

Location: `contracts/sessions/v1alpha1/scenarios/revocation-beats-renewal.yaml:58`, with the later terminal record at `:80` and teardown deadline at `:82`.

The scenario supplies a trusted `revoked` authority decision at `12:00:03Z`, then records the host's terminal acceptance at `12:00:04Z` and gives teardown until `12:00:09Z`. Its summary explicitly describes a revocation already known before the local closing transition. The prose instead defines the bounds from authoritative acceptance and says the host serializes revocation and renewal (`contracts/sessions/v1alpha1/semantics.md:97`, `:108`). As written, it is unclear whether the later lifecycle bookkeeping is incorrectly allowed to start a fresh terminal timer.

Minimal fix: distinguish authoritative acceptance from the later modeled lifecycle observation. For example, preserve an authoritative `accepted_at` of `12:00:03Z` (or earlier), cap teardown at `12:00:08Z`, and explain that the `BeginClose` act at `12:00:04Z` records that existing fact without renewing its deadlines. Alternatively use a scenario ordering that has no earlier trusted revocation observation. Do not imply that observing an already committed revocation lets a later local transition reset the clock.

## Other conclusions

The proposed normative rules close the original F06 gap in substance: data cutoff is a separate obligation from entering `closing`; expired authority is fail-closed; delivery delay and clock/scheduling uncertainty consume the same lease budget; queued data and lower-layer/device output have no extra drain allowance; peer shutdown confirmation is separate from local release; direct paths, bridges and relays inherit the same ceiling; first-terminal reason retention survives later cleanup failure. The enforcement boundary correctly excludes recalling bytes already delivered beyond controlled endpoints. Both media reasons are now in the shared vocabulary, closing E20's vocabulary mismatch.

The ESS representation honestly models trusted supervisor decisions and lifecycle causation only. The verification record explicitly states that runtime cases executed are zero, identifies clock/queue/authority/field-assignment obligations, and warns that the current repository gate does not collect the new authored scenario directory. No runtime conformance or provider timing is invented. Connection and SessionAuthority relations remain visibly UNMAPPED rather than introducing guessed ownership or stub entities.

The two issues above concern the consistency of the new authored examples with those otherwise coherent normative rules. They do not call for session implementation code in this specification-only task.

## Independent checks

Using the pinned `.local/toolchains/ess/0.20.0/bin/ess`:

- `specify validate --path ess`: exit 0, `connectors v1 — 8 file(s), valid`.
- `verify conform author --path ess --scenarios contracts/sessions/v1alpha1/scenarios --out .local/spec-stabilization-20260908/reviewer-a/authored.json`: exit 0, 11 authored scenarios from 11 files, 0 refusals.

Logs and authored output are confined to this reviewer's directory. The parent is running overall gate validation independently; this report makes no claim about its result. No live media, session transport, or timed cutoff tests were executed.
