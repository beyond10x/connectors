# Independent session specification review B — first pass

Date: 2026-09-08. Reviewed the frozen, uncommitted F06/E20 session hardening at opening commit `6c8ecc2e9b1daa8e5bcac8c8311470356eb59e46`. Exact reviewed file hashes are in `reviewed-sources.json` beside this report. No other reviewer output was inspected. No tracked file, planning record, implementation code or managed worktree was changed.

Verdict: **ship with fixes**. Findings: **0 blocker / 2 major / 2 minor / 1 nit**. This verdict concerns specification stabilization, not approval of a runtime implementation. The selected 2 s cutoff and 5 s local teardown/accounting ceilings are coherent proposed requirements; their physical enforcement remains unimplemented and explicitly unproved.

## Findings

### B-S1 — major: recognized terminal authority refusal does not latch the lifecycle

Locations: `ess/domains/sessions.yaml:225`, `ess/domains/sessions.yaml:246`, `ess/domains/sessions.yaml:267`; `contracts/sessions/v1alpha1/scenarios/revocation-beats-renewal.yaml:53`.

Each authority-refused outcome produces only an error. The supervising Session therefore remains Establishing or Ready after a trusted `expired` or `revoked` observation. In Ready, the modeled sequence PermitData(expired) -> authority-refused -> RenewDataLease(allow) -> renewed is still possible without a new session. The existing revocation scenario leaves exactly this gap between the refusal at 12:00:03 and a separate BeginClose at 12:00:04. It only works because the author supplies the next closing command before any competing renewal.

This contradicts `contracts/sessions/v1alpha1/semantics.md:102` and `:110`: expiry is terminal and a late renewal cannot reopen the session. Deriving expiry from real clocks is correctly out of scope; making an already-recognized terminal decision irreversible is lifecycle causation already represented by this model. A future stale `allow` is precisely what terminal state should fence.

Minimal fix: the refusal caused by terminal authority observations must move to Closing atomically from the command's applicable state and begin cutoff/accounting, retaining the first terminal reason. Ensure an unverified authority fails closed as required by admission policy without inventing a successful renewal. Add a follow-up stale allow/readiness attempt to the refusal scenario and expect StateConflict with no data/lease event. Keep the actual authentication and clock checks as explicit executable obligations.

Evidence: `adversarial/expired-decision-does-not-latch.yaml` is a statically reviewed counterexample describing the permissive current model. ESS compiled it; no runtime execution is claimed.

### B-S2 — major: actual continuity loss during Closing cannot become Lost

Locations: `ess/domains/sessions.yaml:153`, `ess/domains/sessions.yaml:178`, `ess/domains/sessions.yaml:318`; `contracts/sessions/v1alpha1/semantics.md:118`.

The continuity_lost transition starts in Offered, Establishing and Ready only. After BeginClose accepts a first terminal, an actual owner/continuity loss therefore yields StateConflict instead of Lost. A subsequent FinishTeardown(released) can produce Closed. That omits the specified case where cleanup can no longer establish continuity and must report Lost while retaining the earlier terminal reason. It also makes the LoseContinuity comment about preserving a prior terminal impossible to exercise in the ordinary Closing state.

Minimal fix: admit Closing -> Lost for actual continuity/owner loss, preserve the existing first terminal fact, and add that scenario. Clearly distinguish this from an unresponsive peer or an unconfirmed protocol close: those alone must continue to permit Closed when the supervisor confirms local release.

Evidence: `adversarial/continuity-lost-after-closing.yaml` expresses the required Closing -> LoseContinuity -> Lost observation. It exposes the missing transition by direct model inspection. ESS author/synthesize accepting that scenario is not evidence that the current model executes it (B-S4).

### B-S3 — minor: several trusted fixture deadlines exceed an already-issued lease expiry

Locations: `contracts/sessions/v1alpha1/scenarios/revocation-refuses-data-and-renewal.yaml:47` and `:71`; `first-terminal-not-replaced.yaml:48` and `:62`; `media-overload-shared-reason.yaml:47` and `:61`; `revocation-beats-renewal.yaml:48` and `:81` in the same directory.

The accepted live lease expires at 12:00:04, but the trusted BeginClose cutoff_due_at is 12:00:05 or 12:00:06. The contract and model comment require earlier expiry to dominate and forbid any post-expiry grace. These examples supply an overlong final cutoff even though they are intended as conforming supervisor decisions. The compiler does not calculate timestamps, so textual consistency is necessary.

Minimal fix: cap the recorded final cutoff_due_at at the existing effective expiry (or, if a separate ceiling is intended, name and document separate fields so this value cannot be mistaken for the actual final cutoff). Keep refused duplicate commands distinct from accepted deadline assignments. Re-audit all accepted fixtures against min(existing lease expiry, drain deadline, terminal + 2 s).

### B-S4 — minor: verification should explicitly state that compilation does not execute trace transitions

Locations: `contracts/sessions/v1alpha1/verification.md:11`, `:33`, `:49`; `contracts/sessions/v1alpha1/semantics.md:176`.

The verification correctly disclaims clocks, devices, transports and field assignment, but “checks ... authored expectations” is broad enough to imply checking the sequential lifecycle trace. The independent Closing -> LoseContinuity -> Lost scenario compiles and synthesizes with zero refusals even though Closing is absent from that transition's source states. Author/synthesize validate declarations/references/types and construct obligations; they do not execute the requested trace against this Session model.

Minimal fix: state this exact limitation and label the lifecycle trace conclusions as a manual model/scenario audit. Preserve compilation evidence as structural evidence only. The installed conform run CLI advertises only billing and oracle-fixture targets; do not substitute either for a Connectors target or invent a runtime result.

### B-S5 — nit: the shared allow comment makes readiness a prerequisite of becoming ready

Location: `ess/domains/sessions.yaml:169` (used by EstablishReady at `:224`).

The shared GateDecision comment says allow requires readiness, while EstablishReady uses allow to enter Ready from Establishing. This is avoidable ambiguity between completed stream/application negotiation and the Session lifecycle state.

Minimal fix: spell out that EstablishReady requires successful prerequisite negotiation/redemption, whereas PermitData and RenewDataLease additionally require the Session already be Ready.

## Evidence and boundaries

Read AGENTS.md, the applicable ESS skill, design intent and session/media sections, all eleven authored scenarios, the complete session model/contract/verification, and the relevant media/auth capability/adapter cross-references. The review accepts the explicit UNMAPPED Connection and SessionAuthority relations; inventing those owners to satisfy this bounded story would be inappropriate. It also accepts zero data drain, coverage of device/lower-layer buffers, finite leases measured from authoritative issuance, anti-replay/clock uncertainty obligations, independently recorded peer shutdown, and first-terminal preservation as coherent specified rules.

Pinned command checks run from the repository root:

```text
.local/toolchains/ess/0.20.0/bin/ess specify validate --path ess
connectors v1 — 8 file(s), valid
exit: 0

.local/toolchains/ess/0.20.0/bin/ess verify conform synthesize --path ess --target ir --scenarios contracts/sessions/v1alpha1/scenarios --out .local/spec-stabilization-20260908/reviewer-b/synthesized.json
197 scenario(s) (11 authored), 0 refusal(s)
exit: 0

.local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios .local/spec-stabilization-20260908/reviewer-b/adversarial --out .local/spec-stabilization-20260908/reviewer-b/adversarial-authored.json
2 authored scenario(s) from 2 file(s), 0 refusal(s)
exit: 0

.local/toolchains/ess/0.20.0/bin/ess verify conform synthesize --path ess --target ir --scenarios .local/spec-stabilization-20260908/reviewer-b/adversarial --out .local/spec-stabilization-20260908/reviewer-b/adversarial-synthesized.json
188 scenario(s) (2 authored), 0 refusal(s)
exit: 0
```

The first adversarial-only author/synthesize run (before adding the expired-observation counterexample) also returned exit 0 with 1 authored / 187 synthesized scenarios. That result established B-S4. The final output files contain both scenarios. No runtime session tests were executed. The root agent owns the full repository gate; this report does not claim its result.
