# Session revocation verification

Scope: `story:contracts-session-revocation`, source findings **F06 / E20**, opening commit `6c8ecc2e9b1daa8e5bcac8c8311470356eb59e46`. This is specification hardening. No runtime session, media adapter, wire codec or Rust source is added or changed.

The normative owner is [sessions §4.1](semantics.md#41-traffic-cutoff-and-teardown-f06), reflected in [media](../../media/v1alpha1/semantics.md), [auth capability](../../auth/capability/v1alpha1/semantics.md) and the [media adapter design](../../../docs/adapters/media-session.md). It selects 2 s maximum from authoritative host revocation to controlled data cutoff, a live lease of at most 2 s from authoritative issuance including all uncertainty, zero data drain after cutoff, and 5 s from the first accepted terminal fact to local teardown/accounting. These are proposed normative ceilings, not measured provider/runtime performance. A binding unable to enforce them must refuse admission.

The 60 s proof-bound establishment token and the live data lease have different jobs. A disconnected direct path cannot extend the live lease; a late renewal cannot resurrect a session. Cutoff covers all controlled queues/directions and device output. Already-delivered remote bytes cannot be recalled, and remote shutdown confirmation is separate from local cleanup. First-terminal retention and explicit `lost` accounting prevent a teardown failure being reported as successful closure. Both `media_incompatible` and `media_overload` belong to the shared terminal vocabulary.

## Scenario audit

ESS compiles the following **supervisor decisions and expected lifecycle effects**. It does not execute a transport, clock, frame queue, audio device or Connectors target. It also does **not execute the sequential lifecycle trace**: an independently authored impossible transition compiled successfully during review. The scenario conclusions below therefore require a separate manual model/trace audit, as well as structural compilation. Timeline instants order decisions; supplied authority decisions/deadlines are trusted observations, not computed facts. Runtime cases executed: **0**.

| Authored scenario | Required contract result | Remaining runtime evidence |
|---|---|---|
| [Revocation](scenarios/revocation-refuses-data-and-renewal.yaml) | Closing refuses data and renewal; local release can complete with peer shutdown unconfirmed | Saturated queues, ignored close, last emission/delivery and owned task/resource accounting |
| [Direct partition](scenarios/direct-partition-expiry-no-grace.yaml) | Original effective lease expiry begins cutoff, no extra grace | Partition immediately after issuance; bounded clock/delivery/scheduling/device uncertainty |
| [Late renewal](scenarios/late-renewal-does-not-revive.yaml) | Closing and Closed cannot reopen under a renewal | Replay/reorder, receipt-time reset, suspend/resume and clock-change injection |
| [Unverified lease](scenarios/unverified-lease-never-opens-data.yaml) | Unknown trust/deadline bound refuses readiness | Actual authenticated issuer, exact binding and conservative deadline translation |
| [Establishment race](scenarios/revoked-during-establishment.yaml) | Closing cannot become Ready; no usable ready handle | Revoke between each partial stream/application binding step; release all partial resources |
| [Incompatibility](scenarios/media-incompatible-shared-reason.yaml) | `media_incompatible` is a valid terminal before readiness | Mismatched endpoint negotiation, zero data and no ready handle |
| [Overload](scenarios/media-overload-shared-reason.yaml) | `media_overload` is a valid shared terminal | Output overflow and explicit cutoff discard distinguished from live-session silent loss |
| [First terminal](scenarios/first-terminal-not-replaced.yaml) | A second close cannot create another terminal; unaccounted cleanup is Lost | Immutable first reason/actor/time assignment, later observation retention and actual resource inventory |
| [Continuity loss](scenarios/continuity-loss-stays-lost.yaml) | Lost cannot renew, transmit or fabricate close success | Restarted owner, surviving endpoints, no invented authority/continuity |
| [Drain](scenarios/drain-expiry-has-no-extra-grace.yaml) | Announced drain deadline is already the final data deadline | At least 2 s notice or confirmed earlier cutoff; every issued lease capped at the deadline |
| [Revocation wins renewal](scenarios/revocation-beats-renewal.yaml) | A trusted revoked decision atomically enters Closing; later bookkeeping cannot restart terminal time | Atomic host renewal/revocation ordering; no authority minted from stale snapshots |
| [Expired data cannot renew](scenarios/expired-data-denial-cannot-renew.yaml) | An expired data admission enters Closing; a later stale `allow` cannot renew | Trustworthy expiry observation and atomic state/authority update |
| [Loss while closing](scenarios/continuity-lost-during-closing.yaml) | Actual continuity loss in Closing becomes Lost; a later release cannot fabricate Closed | Owner loss detection and preservation of the first terminal field values |

Additional textual cases: a direct or relay peer ignores close; queued input, output, DTMF and lower-layer/device buffers are discarded at cutoff; bridge propagation has one end-to-end bound, not a new allowance per hop; a peer already terminal retains its own first reason; earlier drain/expiry beats the ordinary 2 s cutoff; local accounting failure is a reported conformance violation and `lost`, never hidden by a successful close response. These have no claim of compiled timing proof.

## Model and limits

[sessions.yaml](../../../ess/domains/sessions.yaml) declares one supervising `Session`, its lifecycle, binding/lease/terminal values and trusted authority/cleanup decisions. `Connection` and `SessionAuthority` do not yet have owner models; their opaque coordinates retain explicit UNMAPPED relation markers rather than invented stub entities. No persisted per-endpoint gate entity or guessed cardinality is introduced.

ESS checks declaration names, types, command outcomes and lifecycle causation and compiles authored obligations. It does not execute those traces or reject every impossible sequential expectation. The installed `verify conform run --help` lists only `billing` and `oracle-fixture` targets, neither a Connectors session binding. **UNMAPPED executable obligations** remain: authority derivation and lease sequencing/range/anti-replay, exact binding checks, immutable field assignment, deadlines and clock uncertainty, serialization across renewal/revocation, real data-gate fan-out, queue/device cutoff, request outcome accounting, task/resource teardown and remote shutdown confirmation. A caller-provided `allow` would violate the contract. The private model is not a wire expansion or runtime conformance binding.

ESS 0.20.0 refused the initial authoring spelling `UInt`; the documented primitive is `Integer`. Positive monotonic sequence validation remains a binding obligation. It also refused fractional timeline instants (`ESS-AUTHOR-001`) and equal successive instants (`ESS-AUTHOR-023`). The final scenarios use strictly increasing whole-second acts, with corresponding lease and terminal timestamps. These were authoring errors, not runtime regressions or proof of real races.

## Verification commands

From the repository root, with pinned ESS 0.20.0:

```text
.local/toolchains/ess/0.20.0/bin/ess specify validate --path ess
connectors v1 — 8 file(s), valid
exit: 0

.local/toolchains/ess/0.20.0/bin/ess specify compile --path ess --format json
exit: 0

.local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/sessions/v1alpha1/scenarios --out .local/spec-stabilization-20260908/sessions-authored.json
13 authored scenario(s) from 13 file(s), 0 refusal(s)
exit: 0

.local/toolchains/ess/0.20.0/bin/ess verify conform synthesize --path ess --target ir --scenarios contracts/sessions/v1alpha1/scenarios --out .local/spec-stabilization-20260908/sessions-synthesized.json
201 scenario(s) (13 authored), 0 refusal(s)
exit: 0
```

The repository gate currently collects authored scenarios from operations and two auth directories only. It validates the entire ESS model and synthesizes session obligations, but **does not collect this new authored directory**. The explicit session author/synthesis commands above are therefore required in addition to the full gate. Registering new directories in Rust gate tooling is deferred during this specification-only continuation; the current gate alone is insufficient evidence for this story.

## Independent review corrections

First-pass review A found one major and one minor; review B found two majors, two minors and one nit. Overlap is preserved in the immutable source reports. Corrections: clamp accepted cutoff timestamps to existing lease expiry; make non-allow authority decisions enter Closing atomically from readiness/data/renewal, with first-observation time retained; permit actual Closing-to-Lost continuity loss; distinguish negotiation prerequisites from the Ready lifecycle state; explicitly state that ESS compilation does not execute traces. Two adversarial lifecycle obligations were added, bringing the authored set to 13.

A separate [declaration/trace and supplied-deadline audit](../../../docs/evidence/spec-stabilization-20260908/session-trace-audit.json) checked each of the 13 final traces against the declared guarded outcomes and source/target states, wrong-state expectations and final view state. It also checked supplied lease lengths, accepted cutoff <= existing expiry and terminal + 2 s, and teardown <= first terminal + 5 s. All 13 passed. This was an ephemeral specification audit; it did not execute a session implementation or replace clock/device/authority tests. Both independent final reviewers approve with zero residual findings: [semantics](../../../.engineering/planning/review-result/sessions-semantics-r2-20260908.md) and [model](../../../.engineering/planning/review-result/sessions-model-r2-20260908.md). Original findings are preserved in [semantics first pass](../../../.engineering/planning/review-result/sessions-semantics-r1-20260908.md) and [model first pass](../../../.engineering/planning/review-result/sessions-model-r1-20260908.md). Review B independently audited 13 traces / 77 acts, including outcome guards, states, emitted events and supplied deadlines. The final clarification distinguishes live-data-lease expiry (`lease_expired`) from unredeemed establishment-token expiry (`expired`).

The final full gate exited **0**: **50 tests passed, 0 failed, 0 ignored**, including formatting, drift, clippy and MSRV 1.88 checks. It compiled 222 obligations (34 authored from the existing directories); the additional explicit session command compiled 201 (13 authored), with zero refusals. These totals overlap in generated obligations and must not be added as independent tests. The last comment-only expiry clarification produced byte-identical compiled IR to the gated model. [Gate log and summary](../../../docs/evidence/spec-stabilization-20260908/summary.json) preserve the exact command/result. No Rust source, runtime codec or adapter implementation changed in this checkpoint.

```text
env TMPDIR="$PWD/.local/tmp" CARGO_BUILD_JOBS=2 cargo run --locked --offline -p connectors-build -- --ess "$PWD/.local/toolchains/ess/0.20.0/bin/ess" gate --msrv
exit: 0
```

Review source hashes and the [retained evidence](../../../docs/evidence/spec-stabilization-20260908/) distinguish the frozen first pass, corrected recheck and this later evidence-only append. Runtime session cases executed remain zero.
