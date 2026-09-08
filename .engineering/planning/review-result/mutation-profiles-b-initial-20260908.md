---
format: aep.planning-md/1
id: review-result:mutation-profiles-b-initial-20260908
kind: review-result
status: active
title: Mutation profiles reviewer B initial review
relations:
- reviews: story:contracts-mutation-classification
- reviews: story:contracts-restart-idempotency
- reviews: story:contracts-mutation-visibility
revision: 1
---
# Independent reviewer B — mutation profiles initial review

Verdict: **NEEDS REVISION**. Six findings: P0=0, P1=1, P2=5, P3=0. Each finding has one source-story owner. This is a textual contract review, not a request to implement runtime behavior.

Baseline: `c7cd05aab947d09992e55bc217aef5db3546f4ec`. The 62 current-source snapshots are exact Git objects, so concurrent source edits cannot mix this baseline. Six historical sources are pinned to old Connectors `81459ac42ddd518d3942f4b079841e9e0ed6efc8`. `source-hashes.json` and `historical-hashes.json` record all bytes. Other reviewers' output was not read; tracked files/planning were not edited; no runtime suite was run.

## MP-B-01 — SIP declaration cannot satisfy the selected mutation discriminator

**P2 — owner: contracts-mutation-classification.**

Evidence: operations §3 (`contracts/operations/v1alpha1/semantics.md:51–54`) makes external_write the mutation discriminator and restricts human_visible to semantic_effects. Its compiler obligation at line 217 rejects mutation without external_write. Media-session's preservation row, contract table and operation map (`docs/adapters/media-session.md:17,37,63`) omit external_write; the latter two put human_visible in executable effects. Sessions line 65 mentions only session_establishment. Atlassian's Effects column also mixes human_visible into write rows (`docs/adapters/atlassian.md:74–82`).

Select one consistent rule across declaration, summaries and future validation. The minimal rule keeps external_write as the discriminator and explicitly includes externally observable session initiation in that meaning; SIP then declares external_write, session_establishment and send_external, with human_visible in semantic_effects. Neither descriptive hints nor the presence of a vocabulary flag grants authority. Verify rejection of missing/unknown/incompatible executable effects before advertisement or dispatch. Keep the effect requirements of other selected profiles explicit rather than treating every network/process operation as a business mutation.

The §8 “optional descriptor fields” wording must also defer to E02's **required extended operation metadata**, while current strict legacy/schema readers remain unchanged. Existing OperationDeclaration lacks those fields; value-shape models or authored textual examples must not be described as an implemented classifier/compiler check.

## MP-B-02 — Session establishment refusal is not proof of no dial effect

**P1 — owner: contracts-mutation-classification.**

Evidence: sessions §4 (`contracts/sessions/v1alpha1/semantics.md:88`) returns offer_rejected/error and no session ref when a terminal wins before full readiness. The old voice-runtime README:12–15 makes the same ready-receipt distinction; its earlier paragraphs establish the SIP leg before the application binding. Media-session §3 says a dial can ring a real endpoint. Operations §4 (`semantics.md:118–127`) permits refused only with evidence of **no business effect**, and treats ambiguous/partial work as unknown. A call that rang or answered before application connection/media readiness failed cannot be classified as that no-effect refusal merely because no ready handle exists.

Define the dial operation's definitive success boundary separately from ready-handle publication and the session's ongoing state. An admitted pre-gate rejection is not_attempted; a definite provider rejection qualifies as refused only when the selected SIP interpretation proves no business effect. Partial establishment, lost readiness receipt or uncertain signaling after possible send must preserve known/unknown effect evidence even while returning no ready handle and cleaning up. Explicitly cover ringing/answer followed by failed application binding, cancellation/timeout after send, and success followed by immediate session termination. A later closed/lost session cannot rewrite an already settled mutation as never attempted or undone. No automatic redial, approval restoration, session reattach or claim that an absent local session proves no call is permitted. Exact error/classification combinations stay within the existing E02 closed code/outcome binding.

## MP-B-03 — Shared idempotency vocabulary overstates what each dispatch proves

**P2 — owner: contracts-restart-idempotency.**

Evidence: operations §3 (`semantics.md:54`) defines none as “every dispatch is a new effect,” although F01 permits definite refusal, no-op success and uncertainty. The same row calls natural provider idempotency without spelling out repeated effect versus repeated result; §5 (`semantics.md:136`) correctly treats a new none/natural invocation as a new attempt and forbids automatic resend.

Define none as **no deduplication or repeat guarantee**, not proof that an effect occurred. Define natural with an exact target, stable intent, relevant provider preconditions and stated effect equivalence under a stated interference boundary; it does not promise an identical result, a replayed original result or resolution of lost-response uncertainty. Keyed is receiver reservation/replay, not a provider transaction or global exactly-once guarantee. Apply F01's ledger, current admission and approval independently to all modes, including a natural operation likely to be a no-op. No rate-limit, authentication refresh, natural label or unchanged request_id authorizes automatic resend.

## MP-B-04 — Docker repeat guarantees and exact lifecycle target are unsettled

**P2 — owner: contracts-restart-idempotency.**

Evidence: Docker's shared mutation summary (`docs/adapters/docker.md:25`) classifies start/stop/restart as natural, while its map (`:45–47`) classifies restart as none. Selection by id **or name** (`:49`) does not say when a mutable name/allowlist result is fixed to the exact approved container. Vendor endpoint names and semantics are expressly unpinned (`:11`).

Pin a reviewed Engine API profile and state the exact start/stop/restart guarantees separately. Document already-running/already-stopped responses versus state changes, restart's repeated interruption, provider refusal versus uncertain failure, and another actor changing state between invocations. A state-restoring guarantee does not suppress repeated start/stop effects when another actor intervenes. Resolve mutable names/labels through separately admitted bounded preparation, or constrain the operation to an immutable exact container id; the approval/fingerprint/dispatch target must not silently follow a recreated name. If required target binding cannot be established, refuse instead of inventing a hidden preflight or cross-daemon identity. State whether the selected profile uses natural/none or keyed storage and make summary/map/scenarios agree. A later inspect result does not prove whether an earlier lost lifecycle request executed. Docker-specific status claims await pinned vendor evidence; no old Docker implementation is asserted.

## MP-B-05 — Kubernetes timestamp alone does not specify safe repeated restart intent

**P2 — owner: contracts-restart-idempotency.**

Evidence: Kubernetes's operation map (`docs/adapters/kubernetes.md:55`) claims natural “annotation timestamp” without a stable request/precondition contract. Pinned old `local_workloads.rs:297–348` actually sends a strategic merge patch containing **the supplied UID and resourceVersion**, plus a newly generated restartedAt value. It validates returned identity and reports patch_accepted, not rollout convergence. Its error interpretation (`:102–125`) distinguishes definite cluster refusals from unknown dispatch outcomes. The baseline document omits those decisive preconditions.

Preserve exact source-qualified cluster/namespace/name/UID/resourceVersion binding and opaque provider-version equality; do not silently refetch new preconditions or resolve a replacement object after approval. Specify stable restart intent/marker and how any generated request bytes are fixed to an admitted attempt before choosing natural/keyed/none. A successful first patch can make a second use of the original version fail; therefore “every repeated dispatch necessarily rolls out again” is not established. Conversely, a later stale-version response can reflect the first request or an unrelated update and cannot by itself resolve the original lost response. New preconditions, a changed marker/target, a new invocation and exact keyed replay must have distinct stated outcomes. A valid patch response proves the declared patch acceptance only, not healthy Pods or completed rollout. Preserve F02 current result admission, immutable fingerprint/replay and pending/unknown retention; a changed precondition cannot silently replace a live key's request. Verify the chosen provider precondition/response claims against official pinned Kubernetes documentation; old source is historical evidence, not current vendor conformance.

## MP-B-06 — Proposed visibility/refusal policy lacks a complete lookup and admission matrix

**P2 — owner: contracts-mutation-visibility.**

Evidence: operations §4 (`semantics.md:94`) requires disabled mutation → Forbidden, while service v1alpha2 §3.2 (`contracts/service/v1alpha2/semantics.md:72`) projects only implemented/enabled/policy-admitted operations for identity-audience/delegated callers. Static-bearer extended visibility, private lookup of hidden-but-disabled implementations and refusal precedence are unstated. Atlassian only says writes are disabled unless listed (`docs/adapters/atlassian.md:111`); Docker says it advertises enabled operations (`docs/adapters/docker.md:61`). Design §6.2 (`docs/design.md:350`) deliberately separates implemented, enabled, ready and authorized facts.

The actual legacy behavior must not be inferred from a direct handler: Kubernetes filters disabled hosts.discover from its Descriptor (`adapters/kubernetes/src/lib.rs:60–62`). With valid auth/current revision, the public host invokes Descriptor.operation before reaching the adapter (`crates/connectors-host/src/server.rs:119–138`); core lookup returns NotFound for its absence (`crates/connectors-core/src/lib.rs:82–86`). The adapter's direct Forbidden branch (`adapters/kubernetes/src/lib.rs:287–293`) is a separate internal guard. E02 §3 deliberately preserves hidden legacy projection → not_found. New fields/codes remain rejected by the actual legacy codec.

Select one explicit extended visibility/refusal policy for all admitted auth modes, with rows for unimplemented/unbound, implemented-disabled, enabled-but-host-denied, enabled-but-not-ready, authorized enabled and unsupported legacy projection. State whether an independently admitted private lookup can identify a disabled implementation and return forbidden even though it is absent from public discovery; never bypass current result authority to expose existence, conflict, approval or attempt information. Distinguish receiver not_granted from adapter/connection/resource forbidden, stale projection from current denial, and unauthenticated/framing failure. Readiness and the absence of a current one-shot approval must not be conflated with implementation/enablement or trigger provider work during describe. Require current invoke admission after every describe, safe projection revisions/federation intersection, and no hidden/direct-handler bypass. A public descriptor is not an exhaustive artifact inventory, a readiness grant or execution approval; no disabled field is required by this correction.

## ESS and verification boundary

The baseline correctly marks OperationDeclaration effects/risk/idempotency/approval as UNMAPPED (operations §9; `ess/domains/declarations.yaml:66–86`). AttemptRecord, KeyReservation and ApprovalRedemption model existing authority/ledger identities, not a SIP recognizer, Docker lifecycle, Kubernetes CAS or a current access-policy decision. Minimal new value types may record selected effect vocabulary, fixed intent/precondition coordinates and visibility decisions, with cross-field/provenance predicates explicitly unexecuted. Do not replace settled F01/F02/F03 or Session lifecycles, invent a persistent provider-resource entity merely to type request coordinates, or report enum/schema acceptance as conformance execution.

Required evidence is a cross-document textual scenario matrix, exact old-source hashes, pinned vendor facts for provider guarantees, shape-negative and accepted-semantic-counterexample cases if values are added, and accurate separation from existing regression/gate results. Root is independently obtaining vendor evidence; this initial review does not claim to have verified current Docker/Kubernetes API behavior. Backend implementation, live calls and runtime failure suites are outside these stories' present scope.
