# Restart and visibility finding dispositions — 2026-09-08

Twelve individual findings are fixed: nine initial findings and three first-recheck findings. Both final independent reviewers approve both stories with zero residuals. Classification's five already-recorded outcomes remain separate. The original source items owned here are F11/E08 (restart) and E12 (visibility).

Initial raw [review A](../mutation-profiles-20260908/reviews/a-initial-report.md) and [review B](../mutation-profiles-20260908/reviews/b-initial-report.md), with their frozen archives, remain in the preceding packet. This packet preserves the complete raw [first recheck A](reviews/a-recheck1-report.md) and [first recheck B](reviews/b-recheck1-report.md), including machine-readable findings, plus [archive hashes and exact members](reviews/archive-manifest.json). Final raw [review A](reviews/a-recheck2-report.md) and [review B](reviews/b-recheck2-report.md) approve the frozen normative/evidence bytes; mutable planning and these dispositions are excluded from their approval scope. [Verification](verification.md) distinguishes textual expectations, actual schema decisions, official-source inspection and the existing gate from future runtime conformance.

## mp-a-03

Owner: `contracts-restart-idempotency`. Docker §4.1 selects natural start/stop and none restart consistently, fixes intent/parameters and defines acknowledgement/already-satisfied results without current-state, health, historical replay or automatic-resend claims. Exact v1.56 endpoint declarations are pinned. RI01–05, RI11–12 and RI26 cover repeat/no-op/interference/loss; RV-A-02 below refines acknowledgement precision.

## mp-a-04

Owner: `contracts-restart-idempotency`. Kubernetes §4.1 preserves source-qualified namespace/name/UID/version, selects host-keyed observation and fixes one decimal epoch-ms marker/body per candidate before anchoring. Replays and losing candidates cannot send another body; deliberate changed input is new intent. Verified patch acceptance is distinct from rollout completion and does not reconcile prior uncertainty. RI13–25 cover these cases; RV-B-01 below narrows the valid conditional-version input.

## mp-a-05

Owner: `contracts-restart-idempotency`. Docker requires expected full ID with exact name/ID selection, one separately admitted bounded inspect, configured membership and immutable dispatch ID. Signal/wait are receiver-owned fixed configuration. Source/daemon qualification, known replacement, name ambiguity and the inspect/action non-atomic membership limit are explicit. RI06–10 cover these rules; neither approval.prepare nor mutation adds hidden target substitution.

## mp-a-06

Owner: `contracts-restart-idempotency`. Operations §5.2 defines none as no repeat guarantee, natural through exact declared repeat semantics and keyed as receiver reservation/observation. Correlation, provider identity/version, marker and host key remain distinct. All modes retain current admission, approval, ledger and one-shot dispatch; no-op acknowledgement does not invent a physical change. RI01–05 and RI12–22 preserve conflict, retention, uncertainty and no-resend rules.

## mp-a-07

Owner: `contracts-mutation-visibility`. Service v1alpha2 §3.2/§3.2.1 gives all three admission profiles one visibility/private-lookup/refusal matrix. Current policy precedes private disclosure and stale revision; fresh bound/enabled lookup precedes operation input and key observation. Readiness/approval possession do not suppress admitted metadata. Current federation intersection and actual legacy descriptor-vector/adapter-guard behavior remain distinct. VI01–22 cover the matrix; RV-A-01 below closes the conflicting early operations sequence.

## mp-b-03

Owner: `contracts-restart-idempotency`. Operations §5.2 separates no guarantee, natural repeat semantics and keyed replay without asserting a fresh physical effect on every send. Docker acknowledges selected intent; F01/F02/F03 admission/ledger/approval/no-resend invariants remain shared. RI01–05, RI11–14 and RI22 preserve the difference between a new attempt, observed outcome and original uncertainty.

## mp-b-04

Owner: `contracts-restart-idempotency`. Docker's pinned API, per-operation repeat classification, exact daemon/full-ID target, bounded inspect and fixed signal/wait eliminate ambiguous mutable-name intent. The result reports desired state and acknowledgement disposition, with explicit before-response interference and non-atomic membership limits. RI01–12 and RI26 record the selected behavior; no provider effect was executed to claim conformance.

## mp-b-05

Owner: `contracts-restart-idempotency`. Kubernetes keeps the original UID/version and once-fixed patch marker under host-keyed replay, never refetching/rebasing a replacement version. Accepted PATCH does not mean healthy or completed rollout; later conflicts/readbacks cannot settle the lost original. The pinned provider's unsigned comparison and zero/unconditional branch require the binding-specific admission described in RV-B-01. RI13–25 cover repeat, new intent, refusal, marker and version edges.

## mp-b-06

Owner: `contracts-mutation-visibility`. The selected extended matrix separates implementation, supported binding, enablement, metadata/lookup/result authority and dependency readiness across static, identity-audience and delegated bindings. It defines safe denial/stale/private-lookup/key/preflight precedence, management visibility and current federation intersection. Legacy hidden mutation/hosts.discover remains public not_found while the direct adapter guard is internally forbidden. VI01–22 cover the distinctions; private ESS facts do not implement policy.

## rv-a-01

Owner: `contracts-mutation-visibility`. Operations §4 steps 1–3 now limit early work to bounded envelope/authentication/version/syntactic checks and internal nondisclosing policy resolution. Public lookup/schema errors follow current policy, freshness and bound/enabled lookup; input validation precedes key inspection. VI21–22 explicitly cover malformed hidden input and invalid conditional versions with a possibly existing key.

## rv-a-02

Owner: `contracts-restart-idempotency`. Docker start/stop results use desired_state plus disposition. Operations §5.2 defines applied as the selected provider acknowledgement of intent, including an already-satisfied check, without a synchronized state snapshot. Process exit or restart-policy interference may occur before the HTTP response. RI26 records that edge; no follow-up inspect or health/liveness promise is added.

## rv-b-01

Owner: `contracts-restart-idempotency`. The selected built-in Deployment binding accepts only canonical positive ASCII decimal resource_version in 1..18446744073709551615. Zero, all zero aliases, noncanonical positive spellings and overflow refuse at admitted input validation before key/preflight/dispatch; accepted bytes remain unchanged. Four further exact official source files establish the unsigned parser, unconditional strategy, store branch and PATCH delegation. RI23–25 and twelve new schema-accepted invalid semantic inputs expose the generic String gap; the two boundary cases prove shape acceptance only. The ESS comment explicitly leaves enforcement UNMAPPED for the future binding.
