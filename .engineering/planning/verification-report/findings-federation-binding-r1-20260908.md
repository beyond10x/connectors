---
format: aep.planning-md/1
id: verification-report:findings-federation-binding-r1-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:federation-binding-r1-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 7e267cd6add18e6f270309a17f116b3f1f33bb3a58a2bd8f02bbd3408b6dc9fd
relations:
- verifies: review-result:federation-binding-r1-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:federation-binding-r1-20260908

This supplements [the immutable original](../review-result/federation-binding-r1-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

The original report's verdict and scope remain authoritative; no new verdict is assigned.

## Transcription method

8 findings remain in the scope of this report's final stated conclusion.
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
    "file": ".engineering/planning/review-result/federation-binding-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### FB-01 — P1: one canonical approval subject, independently visible before approval\n\nAn alias such as gateway gitlab__issue.create and a gateway descriptor revision are not the leaf's issue.create and leaf revision. Choose one exact, versioned canonical subject that both the issuer and leaf can verify. It must bind trusted caller identity (tenant/realm/principal/executor as applicable), trusted origin, receiver instance, source-owned operation/family/profile, resolved connection and effect-relevant connection/configuration revisions, canonical input digest and canonicalization identifier, plus the applicable operation/descriptor revision. Approval issuer, reference, expiry and approval mode remain explicit. Public source-qualified names and safe references may be disclosed only under current caller/result admission.\n\nA host-owned **approval preparation read** can expose this subject for an implicitly selected or managed connection. It must have a closed admitted payload and safe result, no provider dispatch, no secret entry, no approval spend and no attempt reservation. A returned subject is information for approval, not execution authority. The issuer must approve exactly those values; the eventual receiver recomputes and compares them. A tampered caller copy cannot become canonical because it arrived through the gateway."
  },
  {
    "file": ".engineering/planning/review-result/federation-binding-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### FB-02 — P1: distinguish presentation freshness from signed effect identity\n\nRecord gateway-visible alias/projection revision separately from leaf operation/connection/descriptor/configuration coordinates. For each revision, state whether it is part of signed approval identity or a checked routing freshness condition. A fresh gateway mapping cannot rewrite a signed approval, rebind a connection or redirect an attempt to another receiver. Alias/configuration changes are rechecked before forwarding, and leaf semantic changes are rechecked before approval spending. A route becoming stale yields explicit refusal/current descriptor retrieval, never automatic resend or signature repair. If unchanged leaf approval can survive an unrelated gateway-only refresh, say so precisely and retain the current projection check; otherwise refuse and require new approval. Do not leave this to an implementation inference."
  },
  {
    "file": ".engineering/planning/review-result/federation-binding-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### FB-03 — P1: authenticated gateway facts are scoped trust, not cryptographic proof of arbitrary callers\n\nLeaf receiver configuration fixes the permitted gateway issuer/key set, exact receiver/audience, tenant/realm/subject classes, route scope and allowed authority presentation. A signature establishes that this configured issuer asserted a context; it does not independently prove the issuer honestly authenticated the human. State that trust explicitly. The leaf must still check current authority and its own narrowing policy, operation/connection access and result disclosure. Effective permission is the intersection, never the gateway's broad service identity substituted for the caller. A revoked/missing/unavailable authority or policy fails closed. Authentication, supplied executor assertion and admitted executor binding remain distinct; signing caller-written coordinates does not validate them."
  },
  {
    "file": ".engineering/planning/review-result/federation-binding-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### FB-04 — P1: bind exact receiver and actual request bytes, including bodyless describe\n\nThe new assertion must bind receiver instance/audience, method and exact selected route/target, the raw bounded body digest, selected operation/connection/revisions and the full immutable approved subject or its injective canonical digest. The raw digest covers approval, key, executor and all envelope members actually sent; no middleware may reserialize or decompress those bytes after the signature boundary without an explicitly specified verified transformation.\n\nSeparate outer cryptographic checks from semantic checks: bound headers/raw bytes and verify the signed authentication envelope before application decoding; then strictly decode the invocation and compare body correlation and resolved semantic coordinates before policy/redemption/dispatch. Request ID alone is not content binding. Duplicate headers, duplicate JSON members, unknown fields, unsupported typ/alg, unknown key, alternate base64 forms, compressed/body transformations and route/query ambiguities need explicit refusal rules and byte vectors.\n\nGET describe has no invocation envelope. Give it a fresh private signed correlation/nonce and an exact empty-body/method/path binding, with its own purpose or operation discriminator. Its public E02 response request_id remains **null**. An invoke assertion cannot authorize describe or vice versa. Describe remains caller-scoped policy projection and creates no business approval or execution grant."
  },
  {
    "file": ".engineering/planning/review-result/federation-binding-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### FB-05 — P1: nonce redemption and approval spending have different owners and meanings\n\nThe selected leaf owns atomic durable **transport nonce** consumption for its delegation receiver. A stable key such as (issuer, receiver, jti) must not change on signing-key rotation; kid and credential bytes are not new authority namespaces. Scope and injective encoding must be explicit, with one uniqueness authority across all admitted replicas. Missing, unavailable or ambiguously acknowledged storage cannot admit another use. Capacity refuses new authority rather than evicting live nonces. A duplicate nonce is authentication refusal, not a cached successful invocation.\n\nOnly the execution leaf owns **approval verification/spending** and the mutation attempt/key ledger. The gateway may precheck, prepare and forward the unchanged evidence, but never spends it. Consuming a nonce proves no approval spend or provider effect. A deliberate caller retry receives a new transport nonce and may observe the original keyed attempt under current admission without spending approval again. It does not re-dispatch. A captured identical signed transport request is refused even when its contained idempotency key would otherwise identify a replay."
  },
  {
    "file": ".engineering/planning/review-result/federation-binding-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### FB-06 — P1: time, key rotation and replay retention must describe one acceptance window\n\nChoose exact integer units, accepted bounds, iat/nbf/exp relation, maximum life and bounded clock uncertainty; use checked arithmetic and define equality at expiry. Receivers unable to establish the bound fail closed. Nonce retention covers every instant at which any still-admitted key could validate the assertion, including allowed skew. A wall-clock jump, restart, changed key ID or process replacement cannot shorten retention or make old authority new. Signing-key overlap is bounded receiver configuration, private/public key custody is separate, and key revocation wins current admission. No provider/Identity/service bearer is reused as signing material. Delegation lifetime never extends approval expiry, authority lifetime or the remaining execution budget."
  },
  {
    "file": ".engineering/planning/review-result/federation-binding-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### FB-07 — P1: preserve the settled leaf mutation fence through every failure window\n\nTrace at least: gateway admission/preparation -> leaf authentication and nonce acceptance -> leaf current policy/result admission -> keyed lookup/replay decision -> verify new approval/preflight -> acknowledged audit admission and durable Prepared attempt -> leaf spend -> durable open gate -> at most one provider send -> truthful terminal observation. Exact ordering of early audit and transport redemption may differ if specified, but neither may grant a provider send. Keep winner-recheck races from F02, and do not require original approval still live for an admitted exact replay.\n\nNonce consumed then policy refused: no provider effect and no approval spending. Approval spent then pre-gate abort: approval stays spent and effect is not_attempted. Gate acknowledgement uncertain or gateway loses the downstream response: effect unknown unless a valid definitive original observation is available. Recovery may fence Prepared but cannot infer dispatch from spend status or reacquire permission from Dispatching. No gateway deadline, token refresh, route refresh or new signature permits automatic resend."
  },
  {
    "file": ".engineering/planning/review-result/federation-binding-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### FB-08 — P1: failure correlation and disclosure must preserve E02 semantics\n\nThe gateway returns its own public request correlation and audit facts, preserves original attempt/request identity and exact effect classification from an authenticated valid leaf response, and retains separate source_audit provenance. A leaf pre-envelope failure is not a fabricated business attempt. If a forwarded mutation may have reached the leaf and its reply is absent/invalid, the gateway cannot manufacture not_attempted from its local send failure, nonce state or generic timeout. Current caller/result denial omits original mutation metadata, including replay existence, as E02 already requires. No path or diagnostic discloses private signing keys, credential material or hidden leaf connection inventory."
  }
]
```

