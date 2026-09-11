---
format: aep.planning-md/1
id: verification-report:findings-contract-semantics-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:contract-semantics-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 0788dc95dd5a7794c9e56f45a1193c9f785039772642e56a9c2651e0d6b96788
relations:
- verifies: review-result:contract-semantics-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:contract-semantics-20260908

This supplements [the immutable original](../review-result/contract-semantics-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

Evidence: [input manifest](contracts-2026-09-08-complete-input/manifest.json), [architecture review](2026-09-08-contract-semantics-reviewer-architecture.md), [behavior review](2026-09-08-contract-semantics-reviewer-behavior.md), and [primary notes recorded before reading the independent verdicts](2026-09-08-contract-semantics-own-notes.md). Citations below point into that preserved snapshot. Its 23 files still match the checkout byte for byte. The checkout now contains the reviewed documents in local commit `db1c329`; the primary reviewer made no commits.

## Transcription method

15 findings remain in the scope of this report's final stated conclusion.
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
    "file": "contracts/operations/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "blocker",
    "message": "### F01 · P1 · Mutation recovery can report an applied effect as aborted\n\n[Recovery rule](contracts-2026-09-08-complete-input/contracts/operations/v1alpha1/semantics.md:111), [execution ordering](contracts-2026-09-08-complete-input/contracts/operations/v1alpha1/semantics.md:102), [crash scenario](contracts-2026-09-08-complete-input/contracts/operations/v1alpha1/semantics.md:131).\n\nRecovery uses approval spending to distinguish indeterminate from aborted, while the scenario requires aborted after a crash between spending and dispatch. That durable state is indistinguishable from dispatch followed by a crash before recording the outcome. With `approval: not_required`, an effect can also be dispatched without any spent approval. Reporting aborted can invite a duplicate effect.\n\nDefine recovery independently of approval presence. Aborted requires durable proof that dispatch was prevented; otherwise an unresolved attempt remains indeterminate. Conformance should cover crashes on both sides of dispatch, including mutations with no approval requirement. A durable local record alone cannot prove what a remote provider executed.\n\nAgreement: architecture A1, behavior B1, primary.",
    "line": 111
  },
  {
    "file": "contracts/operations/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "blocker",
    "message": "### F02 · P1 · Idempotency lacks an authority namespace and complete request identity\n\n[Lookup and digest](contracts-2026-09-08-complete-input/contracts/operations/v1alpha1/semantics.md:100), [concurrent requests](contracts-2026-09-08-complete-input/contracts/operations/v1alpha1/semantics.md:120), [keyed Atlassian writes](contracts-2026-09-08-complete-input/docs/adapters/atlassian.md:72).\n\nTwo admitted callers can use key `create-1` with identical input through different connections or operations. The text does not determine whether these are separate requests or a replay. A broad lookup could suppress the second effect or disclose the first result. Canonical input alone does not identify an authorized operation.\n\nDefine the key namespace separately from the stored request fingerprint: admitted caller scope, receiver-owned source, connection, operation identity, and the treatment of semantic/configuration revisions. Recheck current authorization before returning a stored result. Conformance should repeat the same key and body across each boundary, then revoke access before replay.\n\nAgreement: A2, B2, primary.",
    "line": 100
  },
  {
    "file": "contracts/service/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "blocker",
    "message": "### F03 · P1 · Federation changes the identities bound into unchanged approval evidence\n\n[Federation mapping](contracts-2026-09-08-complete-input/contracts/service/v1alpha1/semantics.md:109), [approval subjects](contracts-2026-09-08-complete-input/contracts/operations/v1alpha1/semantics.md:99), [unchanged forwarding](contracts-2026-09-08-complete-input/contracts/operations/v1alpha1/semantics.md:154).\n\nThe gateway prefixes operation names, derives its descriptor revision, and authenticates downstream with its own credential. Approval binds caller, operation, connection, input and revision but is forwarded unchanged. An approval for Alice's gateway-visible operation cannot automatically match the leaf's operation, revision and authenticated gateway caller. Implementors must otherwise reject legitimate requests or weaken verification.\n\nChoose authoritative approval coordinates and define how clients obtain them, how the gateway maps them, and who verifies and spends the approval. Preserve original admitted authority through an explicit trusted context or narrowly authorized delegation. Test a complete gateway-to-leaf mutation and reject attempts to reuse approval across origins or revisions.\n\nAgreement: A3 (P1), B5 (P2), primary. Consolidated as P1 because this is an authority boundary.",
    "line": 109
  },
  {
    "file": "contracts/auth/acquisition/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "blocker",
    "message": "### F04 · P1 · Refresh coordination and recovery exceed custody's promised atomicity\n\n[Required refresh lease](contracts-2026-09-08-complete-input/contracts/auth/acquisition/v1alpha1/semantics.md:78), [custody CAS boundary](contracts-2026-09-08-complete-input/contracts/auth/custody/v1alpha1/semantics.md:57), [rotating Atlassian refresh](contracts-2026-09-08-complete-input/docs/adapters/atlassian.md:56).\n\nAcquisition requires a cross-replica custody lease, while custody promises immutable versions and active-reference CAS as its only required atomicity. CAS cannot stop two external exchanges before publication. Even with a lease, replica A can send a rotating refresh and lose ownership before publishing; a successor sees the unchanged reference and has no specified recovery rule preventing another exchange.\n\nName the coordination owner and require exclusion before exchange, durable attempt state, stale-owner fencing, and recovery when the provider may have consumed the token. Lease takeover must not itself authorize repeating an uncertain exchange. Keep coordination separate from opaque secret storage if custody retains its narrow role. Test owner loss before send, after send and before publication.\n\nAgreement: A4, B3, primary consolidation.",
    "line": 78
  },
  {
    "file": "contracts/auth/connection/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "blocker",
    "message": "### F05 · P1 · Credential replacement can retain evidence about a different identity\n\n[File rotation](contracts-2026-09-08-complete-input/contracts/auth/connection/v1alpha1/semantics.md:100), [identity checks](contracts-2026-09-08-complete-input/contracts/auth/evidence/v1alpha1/semantics.md:65), [evidence freshness](contracts-2026-09-08-complete-input/contracts/auth/evidence/v1alpha1/semantics.md:73), [credential placement](contracts-2026-09-08-complete-input/contracts/auth/capability/v1alpha1/semantics.md:67).\n\nA token file changes from account A's credential to account B's. Connection identity and descriptor revision stay stable, the next dispatch uses new bytes, and age-valid identity/scope evidence can still describe A. Checks limited to acquisition and explicit repair leave this transition undefined.\n\nBind evidence to a credential generation and state which checks replacement invalidates. Admission and dispatch must use the same validated generation. A changed external identity requires explicit reassignment; ordinary rotation must preserve the established identity. Test replacement both between requests and between admission and dispatch, without exposing credential digests publicly.\n\nSource: B4, verified by primary.",
    "line": 100
  },
  {
    "file": "contracts/sessions/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "blocker",
    "message": "### F06 · P1 · Revocation bounds entry into closing without bounding continued traffic\n\n[Two-second revocation rule](contracts-2026-09-08-complete-input/contracts/sessions/v1alpha1/semantics.md:84), [conformance](contracts-2026-09-08-complete-input/contracts/sessions/v1alpha1/semantics.md:101), [media readiness gate](contracts-2026-09-08-complete-input/contracts/media/v1alpha1/semantics.md:67).\n\nA session can enter `closing` within two seconds yet keep forwarding audio while waiting for peer teardown. The maximum call duration in the adapter plan does not establish prompt cessation. A state label alone does not bound revoked access, particularly on a direct media path.\n\nSpecify the cutoff for new requests, incoming and outgoing data, and queued data at revocation/closing. Declare bounded drain and cleanup behavior, including unresponsive peers and direct paths. Conformance must observe traffic cessation, not only the control-state transition.\n\nAgreement: B7, primary.",
    "line": 84
  },
  {
    "file": "contracts/auth/connection/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### F07 · P2 · Connection readiness conflicts with operation-specific scope admission\n\n[Insufficient scope state](contracts-2026-09-08-complete-input/contracts/auth/connection/v1alpha1/semantics.md:58), [non-ready refusal](contracts-2026-09-08-complete-input/contracts/auth/connection/v1alpha1/semantics.md:74), [read permitted despite missing write scope](contracts-2026-09-08-complete-input/contracts/auth/evidence/v1alpha1/semantics.md:88).\n\nThe connection contract turns missing scopes for an enabled write into a non-ready connection that refuses invocation. The evidence scenario requires reads to proceed on that same credential. These rules give different results for the same request.\n\nSeparate connection viability from per-operation eligibility, or define an equally explicit readiness reduction. Name failures that block every operation and failures that affect only particular requirements. The read-without-write-scope scenario must have one outcome across both contracts.\n\nAgreement: A6, primary.",
    "line": 58
  },
  {
    "file": "contracts/auth/evidence/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### F08 · P2 · The permission-check budget cannot satisfy namespace fan-out\n\n[Exact namespace checks](contracts-2026-09-08-complete-input/contracts/auth/evidence/v1alpha1/semantics.md:74), [one-call budget](contracts-2026-09-08-complete-input/contracts/auth/evidence/v1alpha1/semantics.md:81), [two-namespace scenario](contracts-2026-09-08-complete-input/contracts/auth/evidence/v1alpha1/semantics.md:89), [Kubernetes plan](contracts-2026-09-08-complete-input/docs/adapters/kubernetes.md:43).\n\nFresh evidence for namespaces `a` and `b` requires separate authorization checks, but one invocation permits at most one permission-check provider call. The required partial-success scenario cannot satisfy both rules when evidence is absent.\n\nDefine the budget unit: for example, one check per distinct authorization target under a bounded invocation budget, or operations restricted to one target. Specify exhaustion behavior. Test two uncached namespaces with one allowed and one denied. The exact numeric ceiling can follow the semantic decision.\n\nAgreement: A7, primary.",
    "line": 74
  },
  {
    "file": "docs/adapters/grafana.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### F09 · P2 · Monitoring's explicitly unauthenticated profiles are outside the closed auth vocabulary\n\n[Monitoring profile declarations](contracts-2026-09-08-complete-input/docs/adapters/grafana.md:74), [closed scheme list](contracts-2026-09-08-complete-input/contracts/auth/profile/v1alpha1/semantics.md:64), [mandatory credential placement](contracts-2026-09-08-complete-input/contracts/auth/capability/v1alpha1/semantics.md:67).\n\nThe plan permits direct `credential: null` and names `none` for direct and mediated profiles. The closed scheme vocabulary has no such case, and HTTP capabilities require credential resolution and placement. These declarations cannot be expressed consistently.\n\nDefine explicit unauthenticated HTTP configuration and distinguish it from a child whose parent authenticates a mediated hop. Neither may arise as fallback from a missing credential. Validate all monitoring profile rows and test that a broken bearer binding refuses rather than becoming anonymous.\n\nAgreement: A5, primary.",
    "line": 74
  },
  {
    "file": "docs/adapters/media-session.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### F10 · P2 · SIP dial violates the mutation effect discriminator\n\n[SIP declaration](contracts-2026-09-08-complete-input/docs/adapters/media-session.md:17), [operation map](contracts-2026-09-08-complete-input/docs/adapters/media-session.md:37), [effect vocabulary](contracts-2026-09-08-complete-input/contracts/operations/v1alpha1/semantics.md:51), [required validation](contracts-2026-09-08-complete-input/contracts/operations/v1alpha1/semantics.md:155).\n\nDial is a mutation with `session_establishment` and `send_external` but lacks the required `external_write` discriminator. `human_visible` is also placed in `effects` although it belongs to `semantic_effects`. A conforming validator must reject a motivating mutation use case.\n\nDecide whether `external_write` intentionally covers call establishment or whether specified external-effect classes independently qualify for mutation. Align the metadata fields and validator rule. Add SIP dial as a conformance example of the chosen classification.\n\nAgreement: A10, B10, primary.",
    "line": 17
  },
  {
    "file": "docs/adapters/docker.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### F11 · P2 · Restart idempotency declarations disagree or omit the stabilizing condition\n\n[Docker summary](contracts-2026-09-08-complete-input/docs/adapters/docker.md:25), [Docker restart map](contracts-2026-09-08-complete-input/docs/adapters/docker.md:47), [Kubernetes restart declaration](contracts-2026-09-08-complete-input/docs/adapters/kubernetes.md:55).\n\nDocker assigns natural idempotency to the grouped lifecycle operations, then explicitly assigns `none` to restart. Kubernetes calls restart naturally idempotent because it writes an annotation timestamp, without defining whether repeated input fixes that timestamp or generates a new one.\n\nMake Docker's declarations agree. For Kubernetes, state the exact stable intent and preconditions that justify the label, including repeat-result semantics. The old implementation also uses UID/resourceVersion preconditions, so this review does **not** assert that replay inevitably produces two rollouts. A timestamp alone does not establish the guarantee. Test identical input before and after the first effect, including a lost response and intervening resource change.\n\nSource: B6; primary qualified the Kubernetes claim against the cited old source (`../connectors/crates/integration-kubernetes/src/local_workloads.rs:296`, `hosted.rs:839`).",
    "line": 25
  },
  {
    "file": "contracts/datasources/logs/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### F12 · P2 · Timestamp-only log continuation can skip entries or make no progress\n\n[Continuation](contracts-2026-09-08-complete-input/contracts/datasources/logs/v1alpha1/semantics.md:67), [unordered ties](contracts-2026-09-08-complete-input/contracts/datasources/logs/v1alpha1/semantics.md:75), [continuation scenario](contracts-2026-09-08-complete-input/contracts/datasources/logs/v1alpha1/semantics.md:94).\n\nWith 1,001 entries at the same timestamp and a 1,000-entry limit, an exclusive next window skips the remaining entry; an inclusive window can return the same page indefinitely. Identical lines cannot safely stand in for event identities.\n\nSpecify interval inclusivity, duplicate handling and progress guarantees. Require continuation that handles ties, or explicitly report non-resumable partial output when it cannot be guaranteed within bounds. Conformance must exceed the page limit at one timestamp, including identical lines across streams.\n\nAgreement: A8, B8, primary.",
    "line": 67
  },
  {
    "file": "contracts/discovery/resources/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### F13 · P2 · A partial discovery refresh can falsely imply withdrawal\n\n[Implicit withdrawal](contracts-2026-09-08-complete-input/contracts/discovery/resources/v1alpha1/semantics.md:68), [atomic replacement and denied namespaces](contracts-2026-09-08-complete-input/contracts/discovery/resources/v1alpha1/semantics.md:78), [resource bound](contracts-2026-09-08-complete-input/contracts/discovery/resources/v1alpha1/semantics.md:87).\n\nPublishing a refresh truncated by the resource cap or missing a denied namespace removes previously observed resources from the generation. The withdrawal rule then treats them as absent despite incomplete coverage. Refusing publication instead produces another plausible implementation with different behavior.\n\nOnly complete coverage of a defined scope may establish absence. Model partial/stale/unknown coverage separately from confirmed withdrawal. Current permission denial must still stop unauthorized route use; retaining an observation is not a grant. Test a previously complete set followed by cap exhaustion, namespace denial and provider failure.\n\nAgreement: B9, primary.",
    "line": 68
  },
  {
    "file": "contracts/datasources/records/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### F14 · P2 · Document allowlist admission needs an unavailable membership fact\n\n[Opaque document ID input](contracts-2026-09-08-complete-input/contracts/datasources/records/v1alpha1/semantics.md:34), [scope check before any provider call](contracts-2026-09-08-complete-input/contracts/datasources/records/v1alpha1/semantics.md:63), [Atlassian space admission](contracts-2026-09-08-complete-input/docs/adapters/atlassian.md:106).\n\nAn unseen Confluence page ID does not tell the receiver which configured space contains it. The plan supplies a space allowlist but no trusted membership mapping, while requiring admission before any provider call. The receiver cannot perform the stated local check from the declared input.\n\nRequire receiver-owned membership evidence, an admitted scope-constrained lookup, or an explicitly bounded authorization lookup before reading the document. Define unknown and stale membership behavior, and reject caller-asserted membership as authority. Test an unseen page ID and a page moved between spaces.\n\nAgreement: A9, primary.",
    "line": 34
  },
  {
    "file": "contracts/service/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### F15 · P2 · Read refresh retry conflicts with the inherited service behavior\n\n[Base retry prohibition](contracts-2026-09-08-complete-input/contracts/service/v1alpha1/semantics.md:43), [automatic refresh and redispatch](contracts-2026-09-08-complete-input/contracts/auth/capability/v1alpha1/semantics.md:70), [retry conformance](contracts-2026-09-08-complete-input/contracts/auth/capability/v1alpha1/semantics.md:91), [compatibility claim](contracts-2026-09-08-complete-input/contracts/auth/capability/v1alpha1/semantics.md:98).\n\nThe existing service contract promises no automatic retry and distinguishable provider 401 responses. The capability contract mandates refresh and one redispatch for reads while describing the bearer capability as a rename with no wire change. No explicit profile rule determines which observable behavior applies.\n\nPreserve the old behavior for existing profiles, or define an explicitly selected profile that permits refresh retry. Specify the shared deadline, maximum exchanges, error outcome and admission/evidence checks for redispatch. Conformance should observe one call under the old profile and the declared bounded sequence under the new one.\n\nSource: primary; also present in behavior review's earlier pass, before its completed-set top-ten selection.",
    "line": 43
  }
]
```

