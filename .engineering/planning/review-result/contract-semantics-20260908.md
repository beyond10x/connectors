---
format: aep.planning-md/1
id: review-result:contract-semantics-20260908
kind: review-result
status: active
title: Primary and two-agent contract semantics review, 2026-09-08
relations:
- reviews: specification:contract-driven-connectors-design
revision: 1
---
# Contract semantics review — 2026-09-08

Verdict: **needs revision before further implementation**. The division of responsibilities is coherent, but several rules produce incompatible observable behavior. Resolve the six P1 findings before building on these contracts; resolve each P2 before implementing the affected profile. These priorities describe design consequences, not confirmed defects in running code.

The primary reviewer and two independent agents reviewed all 15 semantics documents (14 newly authored plus the existing service contract), the contract index, five adapter plans, and design context including §30. AGENTS.md was also included in the 23-file snapshot. Both independent reviewers returned **needs-revision**. They received the completed input set without seeing each other's findings or the primary review notes. The primary reviewer then checked and consolidated their conclusions.

Evidence: [input manifest](contracts-2026-09-08-complete-input/manifest.json), [architecture review](2026-09-08-contract-semantics-reviewer-architecture.md), [behavior review](2026-09-08-contract-semantics-reviewer-behavior.md), and [primary notes recorded before reading the independent verdicts](2026-09-08-contract-semantics-own-notes.md). Citations below point into that preserved snapshot. Its 23 files still match the checkout byte for byte. The checkout now contains the reviewed documents in local commit `db1c329`; the primary reviewer made no commits.

## Findings

### F01 · P1 · Mutation recovery can report an applied effect as aborted

[Recovery rule](contracts-2026-09-08-complete-input/contracts/operations/v1alpha1/semantics.md:111), [execution ordering](contracts-2026-09-08-complete-input/contracts/operations/v1alpha1/semantics.md:102), [crash scenario](contracts-2026-09-08-complete-input/contracts/operations/v1alpha1/semantics.md:131).

Recovery uses approval spending to distinguish indeterminate from aborted, while the scenario requires aborted after a crash between spending and dispatch. That durable state is indistinguishable from dispatch followed by a crash before recording the outcome. With `approval: not_required`, an effect can also be dispatched without any spent approval. Reporting aborted can invite a duplicate effect.

Define recovery independently of approval presence. Aborted requires durable proof that dispatch was prevented; otherwise an unresolved attempt remains indeterminate. Conformance should cover crashes on both sides of dispatch, including mutations with no approval requirement. A durable local record alone cannot prove what a remote provider executed.

Agreement: architecture A1, behavior B1, primary.

### F02 · P1 · Idempotency lacks an authority namespace and complete request identity

[Lookup and digest](contracts-2026-09-08-complete-input/contracts/operations/v1alpha1/semantics.md:100), [concurrent requests](contracts-2026-09-08-complete-input/contracts/operations/v1alpha1/semantics.md:120), [keyed Atlassian writes](contracts-2026-09-08-complete-input/docs/adapters/atlassian.md:72).

Two admitted callers can use key `create-1` with identical input through different connections or operations. The text does not determine whether these are separate requests or a replay. A broad lookup could suppress the second effect or disclose the first result. Canonical input alone does not identify an authorized operation.

Define the key namespace separately from the stored request fingerprint: admitted caller scope, receiver-owned source, connection, operation identity, and the treatment of semantic/configuration revisions. Recheck current authorization before returning a stored result. Conformance should repeat the same key and body across each boundary, then revoke access before replay.

Agreement: A2, B2, primary.

### F03 · P1 · Federation changes the identities bound into unchanged approval evidence

[Federation mapping](contracts-2026-09-08-complete-input/contracts/service/v1alpha1/semantics.md:109), [approval subjects](contracts-2026-09-08-complete-input/contracts/operations/v1alpha1/semantics.md:99), [unchanged forwarding](contracts-2026-09-08-complete-input/contracts/operations/v1alpha1/semantics.md:154).

The gateway prefixes operation names, derives its descriptor revision, and authenticates downstream with its own credential. Approval binds caller, operation, connection, input and revision but is forwarded unchanged. An approval for Alice's gateway-visible operation cannot automatically match the leaf's operation, revision and authenticated gateway caller. Implementors must otherwise reject legitimate requests or weaken verification.

Choose authoritative approval coordinates and define how clients obtain them, how the gateway maps them, and who verifies and spends the approval. Preserve original admitted authority through an explicit trusted context or narrowly authorized delegation. Test a complete gateway-to-leaf mutation and reject attempts to reuse approval across origins or revisions.

Agreement: A3 (P1), B5 (P2), primary. Consolidated as P1 because this is an authority boundary.

### F04 · P1 · Refresh coordination and recovery exceed custody's promised atomicity

[Required refresh lease](contracts-2026-09-08-complete-input/contracts/auth/acquisition/v1alpha1/semantics.md:78), [custody CAS boundary](contracts-2026-09-08-complete-input/contracts/auth/custody/v1alpha1/semantics.md:57), [rotating Atlassian refresh](contracts-2026-09-08-complete-input/docs/adapters/atlassian.md:56).

Acquisition requires a cross-replica custody lease, while custody promises immutable versions and active-reference CAS as its only required atomicity. CAS cannot stop two external exchanges before publication. Even with a lease, replica A can send a rotating refresh and lose ownership before publishing; a successor sees the unchanged reference and has no specified recovery rule preventing another exchange.

Name the coordination owner and require exclusion before exchange, durable attempt state, stale-owner fencing, and recovery when the provider may have consumed the token. Lease takeover must not itself authorize repeating an uncertain exchange. Keep coordination separate from opaque secret storage if custody retains its narrow role. Test owner loss before send, after send and before publication.

Agreement: A4, B3, primary consolidation.

### F05 · P1 · Credential replacement can retain evidence about a different identity

[File rotation](contracts-2026-09-08-complete-input/contracts/auth/connection/v1alpha1/semantics.md:100), [identity checks](contracts-2026-09-08-complete-input/contracts/auth/evidence/v1alpha1/semantics.md:65), [evidence freshness](contracts-2026-09-08-complete-input/contracts/auth/evidence/v1alpha1/semantics.md:73), [credential placement](contracts-2026-09-08-complete-input/contracts/auth/capability/v1alpha1/semantics.md:67).

A token file changes from account A's credential to account B's. Connection identity and descriptor revision stay stable, the next dispatch uses new bytes, and age-valid identity/scope evidence can still describe A. Checks limited to acquisition and explicit repair leave this transition undefined.

Bind evidence to a credential generation and state which checks replacement invalidates. Admission and dispatch must use the same validated generation. A changed external identity requires explicit reassignment; ordinary rotation must preserve the established identity. Test replacement both between requests and between admission and dispatch, without exposing credential digests publicly.

Source: B4, verified by primary.

### F06 · P1 · Revocation bounds entry into closing without bounding continued traffic

[Two-second revocation rule](contracts-2026-09-08-complete-input/contracts/sessions/v1alpha1/semantics.md:84), [conformance](contracts-2026-09-08-complete-input/contracts/sessions/v1alpha1/semantics.md:101), [media readiness gate](contracts-2026-09-08-complete-input/contracts/media/v1alpha1/semantics.md:67).

A session can enter `closing` within two seconds yet keep forwarding audio while waiting for peer teardown. The maximum call duration in the adapter plan does not establish prompt cessation. A state label alone does not bound revoked access, particularly on a direct media path.

Specify the cutoff for new requests, incoming and outgoing data, and queued data at revocation/closing. Declare bounded drain and cleanup behavior, including unresponsive peers and direct paths. Conformance must observe traffic cessation, not only the control-state transition.

Agreement: B7, primary.

### F07 · P2 · Connection readiness conflicts with operation-specific scope admission

[Insufficient scope state](contracts-2026-09-08-complete-input/contracts/auth/connection/v1alpha1/semantics.md:58), [non-ready refusal](contracts-2026-09-08-complete-input/contracts/auth/connection/v1alpha1/semantics.md:74), [read permitted despite missing write scope](contracts-2026-09-08-complete-input/contracts/auth/evidence/v1alpha1/semantics.md:88).

The connection contract turns missing scopes for an enabled write into a non-ready connection that refuses invocation. The evidence scenario requires reads to proceed on that same credential. These rules give different results for the same request.

Separate connection viability from per-operation eligibility, or define an equally explicit readiness reduction. Name failures that block every operation and failures that affect only particular requirements. The read-without-write-scope scenario must have one outcome across both contracts.

Agreement: A6, primary.

### F08 · P2 · The permission-check budget cannot satisfy namespace fan-out

[Exact namespace checks](contracts-2026-09-08-complete-input/contracts/auth/evidence/v1alpha1/semantics.md:74), [one-call budget](contracts-2026-09-08-complete-input/contracts/auth/evidence/v1alpha1/semantics.md:81), [two-namespace scenario](contracts-2026-09-08-complete-input/contracts/auth/evidence/v1alpha1/semantics.md:89), [Kubernetes plan](contracts-2026-09-08-complete-input/docs/adapters/kubernetes.md:43).

Fresh evidence for namespaces `a` and `b` requires separate authorization checks, but one invocation permits at most one permission-check provider call. The required partial-success scenario cannot satisfy both rules when evidence is absent.

Define the budget unit: for example, one check per distinct authorization target under a bounded invocation budget, or operations restricted to one target. Specify exhaustion behavior. Test two uncached namespaces with one allowed and one denied. The exact numeric ceiling can follow the semantic decision.

Agreement: A7, primary.

### F09 · P2 · Monitoring's explicitly unauthenticated profiles are outside the closed auth vocabulary

[Monitoring profile declarations](contracts-2026-09-08-complete-input/docs/adapters/grafana.md:74), [closed scheme list](contracts-2026-09-08-complete-input/contracts/auth/profile/v1alpha1/semantics.md:64), [mandatory credential placement](contracts-2026-09-08-complete-input/contracts/auth/capability/v1alpha1/semantics.md:67).

The plan permits direct `credential: null` and names `none` for direct and mediated profiles. The closed scheme vocabulary has no such case, and HTTP capabilities require credential resolution and placement. These declarations cannot be expressed consistently.

Define explicit unauthenticated HTTP configuration and distinguish it from a child whose parent authenticates a mediated hop. Neither may arise as fallback from a missing credential. Validate all monitoring profile rows and test that a broken bearer binding refuses rather than becoming anonymous.

Agreement: A5, primary.

### F10 · P2 · SIP dial violates the mutation effect discriminator

[SIP declaration](contracts-2026-09-08-complete-input/docs/adapters/media-session.md:17), [operation map](contracts-2026-09-08-complete-input/docs/adapters/media-session.md:37), [effect vocabulary](contracts-2026-09-08-complete-input/contracts/operations/v1alpha1/semantics.md:51), [required validation](contracts-2026-09-08-complete-input/contracts/operations/v1alpha1/semantics.md:155).

Dial is a mutation with `session_establishment` and `send_external` but lacks the required `external_write` discriminator. `human_visible` is also placed in `effects` although it belongs to `semantic_effects`. A conforming validator must reject a motivating mutation use case.

Decide whether `external_write` intentionally covers call establishment or whether specified external-effect classes independently qualify for mutation. Align the metadata fields and validator rule. Add SIP dial as a conformance example of the chosen classification.

Agreement: A10, B10, primary.

### F11 · P2 · Restart idempotency declarations disagree or omit the stabilizing condition

[Docker summary](contracts-2026-09-08-complete-input/docs/adapters/docker.md:25), [Docker restart map](contracts-2026-09-08-complete-input/docs/adapters/docker.md:47), [Kubernetes restart declaration](contracts-2026-09-08-complete-input/docs/adapters/kubernetes.md:55).

Docker assigns natural idempotency to the grouped lifecycle operations, then explicitly assigns `none` to restart. Kubernetes calls restart naturally idempotent because it writes an annotation timestamp, without defining whether repeated input fixes that timestamp or generates a new one.

Make Docker's declarations agree. For Kubernetes, state the exact stable intent and preconditions that justify the label, including repeat-result semantics. The old implementation also uses UID/resourceVersion preconditions, so this review does **not** assert that replay inevitably produces two rollouts. A timestamp alone does not establish the guarantee. Test identical input before and after the first effect, including a lost response and intervening resource change.

Source: B6; primary qualified the Kubernetes claim against the cited old source (`../connectors/crates/integration-kubernetes/src/local_workloads.rs:296`, `hosted.rs:839`).

### F12 · P2 · Timestamp-only log continuation can skip entries or make no progress

[Continuation](contracts-2026-09-08-complete-input/contracts/datasources/logs/v1alpha1/semantics.md:67), [unordered ties](contracts-2026-09-08-complete-input/contracts/datasources/logs/v1alpha1/semantics.md:75), [continuation scenario](contracts-2026-09-08-complete-input/contracts/datasources/logs/v1alpha1/semantics.md:94).

With 1,001 entries at the same timestamp and a 1,000-entry limit, an exclusive next window skips the remaining entry; an inclusive window can return the same page indefinitely. Identical lines cannot safely stand in for event identities.

Specify interval inclusivity, duplicate handling and progress guarantees. Require continuation that handles ties, or explicitly report non-resumable partial output when it cannot be guaranteed within bounds. Conformance must exceed the page limit at one timestamp, including identical lines across streams.

Agreement: A8, B8, primary.

### F13 · P2 · A partial discovery refresh can falsely imply withdrawal

[Implicit withdrawal](contracts-2026-09-08-complete-input/contracts/discovery/resources/v1alpha1/semantics.md:68), [atomic replacement and denied namespaces](contracts-2026-09-08-complete-input/contracts/discovery/resources/v1alpha1/semantics.md:78), [resource bound](contracts-2026-09-08-complete-input/contracts/discovery/resources/v1alpha1/semantics.md:87).

Publishing a refresh truncated by the resource cap or missing a denied namespace removes previously observed resources from the generation. The withdrawal rule then treats them as absent despite incomplete coverage. Refusing publication instead produces another plausible implementation with different behavior.

Only complete coverage of a defined scope may establish absence. Model partial/stale/unknown coverage separately from confirmed withdrawal. Current permission denial must still stop unauthorized route use; retaining an observation is not a grant. Test a previously complete set followed by cap exhaustion, namespace denial and provider failure.

Agreement: B9, primary.

### F14 · P2 · Document allowlist admission needs an unavailable membership fact

[Opaque document ID input](contracts-2026-09-08-complete-input/contracts/datasources/records/v1alpha1/semantics.md:34), [scope check before any provider call](contracts-2026-09-08-complete-input/contracts/datasources/records/v1alpha1/semantics.md:63), [Atlassian space admission](contracts-2026-09-08-complete-input/docs/adapters/atlassian.md:106).

An unseen Confluence page ID does not tell the receiver which configured space contains it. The plan supplies a space allowlist but no trusted membership mapping, while requiring admission before any provider call. The receiver cannot perform the stated local check from the declared input.

Require receiver-owned membership evidence, an admitted scope-constrained lookup, or an explicitly bounded authorization lookup before reading the document. Define unknown and stale membership behavior, and reject caller-asserted membership as authority. Test an unseen page ID and a page moved between spaces.

Agreement: A9, primary.

### F15 · P2 · Read refresh retry conflicts with the inherited service behavior

[Base retry prohibition](contracts-2026-09-08-complete-input/contracts/service/v1alpha1/semantics.md:43), [automatic refresh and redispatch](contracts-2026-09-08-complete-input/contracts/auth/capability/v1alpha1/semantics.md:70), [retry conformance](contracts-2026-09-08-complete-input/contracts/auth/capability/v1alpha1/semantics.md:91), [compatibility claim](contracts-2026-09-08-complete-input/contracts/auth/capability/v1alpha1/semantics.md:98).

The existing service contract promises no automatic retry and distinguishable provider 401 responses. The capability contract mandates refresh and one redispatch for reads while describing the bearer capability as a rename with no wire change. No explicit profile rule determines which observable behavior applies.

Preserve the old behavior for existing profiles, or define an explicitly selected profile that permits refresh retry. Specify the shared deadline, maximum exchanges, error outcome and admission/evidence checks for redispatch. Conformance should observe one call under the old profile and the declared bounded sequence under the new one.

Source: primary; also present in behavior review's earlier pass, before its completed-set top-ten selection.

## Boundaries to retain

Keep caller admission, provider authentication, evidence and secret custody separate. Keep discovery observational and materialization explicit. Preserve fixed mediated destinations and the absence of fallback. Keep records, logs and series distinct, and keep SIP/RTVBP protocol code outside the shared media contract. Neither reviewer found a reason to abandon these boundaries.

The completed SIP plan connects outward to a configured application endpoint before reporting readiness. That resolves the initial attachment-order concern for this selected composition; it is not a blocker in this report. The generic session profile should state that prebound/outward arrangement explicitly rather than imply that a locator returned only at readiness can bootstrap the attachment required for readiness.

## Decisions to keep explicit, without treating them as confirmed contradictions

- Decide whether every unchanged discovery refresh intentionally invalidates an exact-generation route. If so, specify the validated renewal or rematerialization flow; otherwise distinguish renewed evidence from changed target identity. Preserve the prohibition on automatic retargeting.
- Define adapter-specific enforcement of query scope for opaque provider queries. The contract's refusal to parse queries does not prohibit an adapter parser; the enforcement strategy needs its own adversarial examples.
- Clarify the intended document body representation and truncation behavior. A UTF-8 string could intentionally contain serialized JSON; the text should tell consumers what remains valid after truncation.
- Confirm connection uniqueness across tenants/sites/resources and the binding context for `subject: none`. Keep the deferred tenant-assignment decision separate from already required authority rules.
- Exact transport framing, proxy paths, backend choices, measured limits and further codecs may follow these semantics. Docker's explicitly unpinned vendor reference remains a verification obligation at authoring; this review did not independently validate vendor APIs.

## Verification and next work

All reviewed snapshot hashes match the checkout. A local relative-link audit found no broken links in the contracts and adapter documents. This was a textual review: no runtime, schema-generation or conformance tests were run, and no implementation correctness is claimed. Only ignored review artifacts were written; contracts, adapter docs, design, ESS and planning were not changed by the reviewer.

Revise the authority and lifecycle rules first (F01–F06), then align the affected profiles and adapter declarations (F07–F15). Put each counterexample into the textual conformance scenarios, then have the same two reviewers check the revisions before decomposition or implementation resumes. The report proposes those corrections; it does not silently choose unresolved product policy.
