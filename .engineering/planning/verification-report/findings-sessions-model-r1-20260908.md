---
format: aep.planning-md/1
id: verification-report:findings-sessions-model-r1-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:sessions-model-r1-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 4928e05df65e579118404a7b7460d25c03a434675c9a580c4856bddbe91a5e08
relations:
- verifies: review-result:sessions-model-r1-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:sessions-model-r1-20260908

This supplements [the immutable original](../review-result/sessions-model-r1-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

Verdict: **ship with fixes**. Findings: **0 blocker / 2 major / 2 minor / 1 nit**. This verdict concerns specification stabilization, not approval of a runtime implementation. The selected 2 s cutoff and 5 s local teardown/accounting ceilings are coherent proposed requirements; their physical enforcement remains unimplemented and explicitly unproved.

## Transcription method

5 findings remain in the scope of this report's final stated conclusion.
Closed items in a same-pass disposition and verification/command tables are excluded.
Each nonempty message reproduces a source section or table row verbatim, including
its original identifier and citations. P0/P1, major and blocking map to blocker;
P2, minor and should-fix map to warning; P3 and nit map to note. Ungraded observations
remain unspecified. No per-finding verdict or introduced/pre-existing classification
is invented. The file is the first explicit source citation; when only shorthand
citations are present, legacy-source-excerpt identifies the original report itself.
The full citation context remains in the message and original. This transcription
makes no claim that paraphrased findings across different historical rounds have
identical comparison signatures.

## Findings

```findings
[
  {
    "file": "ess/domains/sessions.yaml",
    "category": "legacy-review",
    "severity": "blocker",
    "message": "### B-S1 — major: recognized terminal authority refusal does not latch the lifecycle\n\nLocations: `ess/domains/sessions.yaml:225`, `ess/domains/sessions.yaml:246`, `ess/domains/sessions.yaml:267`; `contracts/sessions/v1alpha1/scenarios/revocation-beats-renewal.yaml:53`.\n\nEach authority-refused outcome produces only an error. The supervising Session therefore remains Establishing or Ready after a trusted `expired` or `revoked` observation. In Ready, the modeled sequence PermitData(expired) -> authority-refused -> RenewDataLease(allow) -> renewed is still possible without a new session. The existing revocation scenario leaves exactly this gap between the refusal at 12:00:03 and a separate BeginClose at 12:00:04. It only works because the author supplies the next closing command before any competing renewal.\n\nThis contradicts `contracts/sessions/v1alpha1/semantics.md:102` and `:110`: expiry is terminal and a late renewal cannot reopen the session. Deriving expiry from real clocks is correctly out of scope; making an already-recognized terminal decision irreversible is lifecycle causation already represented by this model. A future stale `allow` is precisely what terminal state should fence.\n\nMinimal fix: the refusal caused by terminal authority observations must move to Closing atomically from the command's applicable state and begin cutoff/accounting, retaining the first terminal reason. Ensure an unverified authority fails closed as required by admission policy without inventing a successful renewal. Add a follow-up stale allow/readiness attempt to the refusal scenario and expect StateConflict with no data/lease event. Keep the actual authentication and clock checks as explicit executable obligations.\n\nEvidence: `adversarial/expired-decision-does-not-latch.yaml` is a statically reviewed counterexample describing the permissive current model. ESS compiled it; no runtime execution is claimed.",
    "line": 225
  },
  {
    "file": "ess/domains/sessions.yaml",
    "category": "legacy-review",
    "severity": "blocker",
    "message": "### B-S2 — major: actual continuity loss during Closing cannot become Lost\n\nLocations: `ess/domains/sessions.yaml:153`, `ess/domains/sessions.yaml:178`, `ess/domains/sessions.yaml:318`; `contracts/sessions/v1alpha1/semantics.md:118`.\n\nThe continuity_lost transition starts in Offered, Establishing and Ready only. After BeginClose accepts a first terminal, an actual owner/continuity loss therefore yields StateConflict instead of Lost. A subsequent FinishTeardown(released) can produce Closed. That omits the specified case where cleanup can no longer establish continuity and must report Lost while retaining the earlier terminal reason. It also makes the LoseContinuity comment about preserving a prior terminal impossible to exercise in the ordinary Closing state.\n\nMinimal fix: admit Closing -> Lost for actual continuity/owner loss, preserve the existing first terminal fact, and add that scenario. Clearly distinguish this from an unresponsive peer or an unconfirmed protocol close: those alone must continue to permit Closed when the supervisor confirms local release.\n\nEvidence: `adversarial/continuity-lost-after-closing.yaml` expresses the required Closing -> LoseContinuity -> Lost observation. It exposes the missing transition by direct model inspection. ESS author/synthesize accepting that scenario is not evidence that the current model executes it (B-S4).",
    "line": 153
  },
  {
    "file": "contracts/sessions/v1alpha1/scenarios/revocation-refuses-data-and-renewal.yaml",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### B-S3 — minor: several trusted fixture deadlines exceed an already-issued lease expiry\n\nLocations: `contracts/sessions/v1alpha1/scenarios/revocation-refuses-data-and-renewal.yaml:47` and `:71`; `first-terminal-not-replaced.yaml:48` and `:62`; `media-overload-shared-reason.yaml:47` and `:61`; `revocation-beats-renewal.yaml:48` and `:81` in the same directory.\n\nThe accepted live lease expires at 12:00:04, but the trusted BeginClose cutoff_due_at is 12:00:05 or 12:00:06. The contract and model comment require earlier expiry to dominate and forbid any post-expiry grace. These examples supply an overlong final cutoff even though they are intended as conforming supervisor decisions. The compiler does not calculate timestamps, so textual consistency is necessary.\n\nMinimal fix: cap the recorded final cutoff_due_at at the existing effective expiry (or, if a separate ceiling is intended, name and document separate fields so this value cannot be mistaken for the actual final cutoff). Keep refused duplicate commands distinct from accepted deadline assignments. Re-audit all accepted fixtures against min(existing lease expiry, drain deadline, terminal + 2 s).",
    "line": 47
  },
  {
    "file": "contracts/sessions/v1alpha1/verification.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### B-S4 — minor: verification should explicitly state that compilation does not execute trace transitions\n\nLocations: `contracts/sessions/v1alpha1/verification.md:11`, `:33`, `:49`; `contracts/sessions/v1alpha1/semantics.md:176`.\n\nThe verification correctly disclaims clocks, devices, transports and field assignment, but “checks ... authored expectations” is broad enough to imply checking the sequential lifecycle trace. The independent Closing -> LoseContinuity -> Lost scenario compiles and synthesizes with zero refusals even though Closing is absent from that transition's source states. Author/synthesize validate declarations/references/types and construct obligations; they do not execute the requested trace against this Session model.\n\nMinimal fix: state this exact limitation and label the lifecycle trace conclusions as a manual model/scenario audit. Preserve compilation evidence as structural evidence only. The installed conform run CLI advertises only billing and oracle-fixture targets; do not substitute either for a Connectors target or invent a runtime result.",
    "line": 11
  },
  {
    "file": "ess/domains/sessions.yaml",
    "category": "legacy-review",
    "severity": "note",
    "message": "### B-S5 — nit: the shared allow comment makes readiness a prerequisite of becoming ready\n\nLocation: `ess/domains/sessions.yaml:169` (used by EstablishReady at `:224`).\n\nThe shared GateDecision comment says allow requires readiness, while EstablishReady uses allow to enter Ready from Establishing. This is avoidable ambiguity between completed stream/application negotiation and the Session lifecycle state.\n\nMinimal fix: spell out that EstablishReady requires successful prerequisite negotiation/redemption, whereas PermitData and RenewDataLease additionally require the Session already be Ready.",
    "line": 169
  }
]
```

