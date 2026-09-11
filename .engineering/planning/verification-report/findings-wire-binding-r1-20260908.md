---
format: aep.planning-md/1
id: verification-report:findings-wire-binding-r1-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:wire-binding-r1-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 939ad9aa5490b5c1a87c47324cd30f04a324d8b0c5c03b1fdc658fa3e3d5f09f
relations:
- verifies: review-result:wire-binding-r1-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:wire-binding-r1-20260908

This supplements [the immutable original](../review-result/wire-binding-r1-20260908.md).
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
    "file": ".engineering/planning/review-result/wire-binding-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### WB-01 — P1: no concrete version selection for describe or dual projections\n\n`service/v1alpha2` §1 and §4.10 keep `/v1/describe` and `/v1/invoke`, promise optional old projection, and require one selected version per client. GET has no envelope and the proposal never defines how an old/new request selects the descriptor codec. An old client cannot express the preference that would cause an old-shape projection. Its version check is too late to reject a new descriptor cleanly. The §6 promise “v1alpha1 client against governed instance → unsupported” is not established by current client behavior.\n\nMinimal semantic correction: freeze current routes and legacy codec unchanged; choose an explicit endpoint/binding selector for the proposed codec (a separate configured base/route is sufficient), with one precise default/no-selector rule, exact unsupported request/response treatment and no fallback/resend. A new client may explicitly implement the legacy codec, but must select it before requesting describe, and must not claim it received governance fields. If final version spelling remains operator-owned, use a named proposed binding and mark its release identifier pending instead of treating the filename as approval."
  },
  {
    "file": ".engineering/planning/review-result/wire-binding-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### WB-02 — P1: mixed-version failure claims contradict the actual strict decoder\n\n`service/v1alpha2` §4.10 says every v1alpha2 invocation to v1alpha1 yields invalid_input, and “the client reports it as unsupported.” A version-only request in the current old shape instead receives enveloped unsupported. Added-field requests receive bare invalid_input. Neither is distinguishable from other malformed requests by blindly remapping every invalid_input. Old-reader/new-error failures also depend on whether failure is enveloped or bare and which HTTP status is used.\n\nMinimal correction: compatibility vectors must separately cover same shape/different version, added descriptor and operation fields, added invocation fields, added success/error fields, new closed codes, and pre-envelope bare errors. Record server wire bytes/shape versus client-visible code separately; never promise a diagnostic the already released reader cannot generate. New readers must reject correlation/version mismatches and unknown fields rather than guess or resend."
  },
  {
    "file": ".engineering/planning/review-result/wire-binding-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### WB-03 — P1: mutation effects have no unambiguous complete response encoding\n\n`operations` §3.4, §4 and §8 require independently observable effect knowledge (`not_attempted`, `refused`, `applied`, `unknown`) and safe error cause, including known provider success/refusal when ledger persistence fails. Its illustrative examples use `effect: replayed`, which confuses replay provenance with effect classification. `service/v1alpha2` §3.4 merely says mutation outcome fields belong to the profile and supplies only result/error plus audit_ref. The codec therefore cannot presently express all settled F01 outcomes without inference. Both-audit-reference federation output is also promised without a field shape.\n\nMinimal correction: the E02 matrix must select exact typed mutation observation fields, distinguish original effect from replay metadata and original request/attempt identity, and define shape/status/code consistency. Unknown uses outcome_unknown even when diagnostic cause is timeout. A successful replay keeps the original `applied` classification, with replay represented separately. Known provider refusal must preserve its cause/classification when terminal write fails. Define gateway and leaf audit references separately (or mark the delegated binding unavailable until its exact schema is settled). Do not derive effect knowledge from HTTP status or generic timeout."
  },
  {
    "file": ".engineering/planning/review-result/wire-binding-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### WB-04 — P1: legacy projection filtering does not prevent direct downgrade invocation\n\n`service/v1alpha2` §4.10 hides operations only when approval is required or effects include external_write. `operations` §7 requires hiding mutation operations entirely. Today classification also admits process, local_system, send_external and session_establishment effects; absence of external_write is not evidence of a safe legacy read. Hiding an operation in describe alone does not prevent a caller constructing its ID directly, and no rule binds legacy invoke to the projected operation set plus its exact projection revision. “No other loss” in §7 is unsupported for errors, audit, connection selection, executor and descriptor curation.\n\nMinimal correction: default projection off. Permit only an explicit verified unchanged configured/read subset under static-bearer; require the legacy invoke handler to reject every operation outside that exact projected subset before any provider call, even with a guessed operation ID/current full descriptor revision. Legacy descriptors, revisions, schemas, error vocabulary and responses need an explicit codec and validation; mutation/managed/governed/session operations and semantics that cannot survive projection are refused, not silently stripped. Gateway advertisement must be the support intersection for the selected codec at every hop."
  },
  {
    "file": ".engineering/planning/review-result/wire-binding-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### WB-05 — P1, F03 dependency: delegated context cannot yet bind the request it authenticates\n\nThe proposed header (§3.6) signs context and request_id but not destination leaf, method/path, body/input digest, operation/connection or revision. A captured valid assertion can accompany different semantics under the same first-used request ID; “one request id” is not body integrity. Request ID is inside the body, yet the text claims verification/binding before any body decoding. GET describe has no downstream request ID field. Per-route key selection, canonical encoding, one-use replay storage/atomicity, duplicate header handling, clock uncertainty and key rotation are unspecified. The §5 one-use promise requires more than HMAC verification and a timestamp.\n\nMinimal correction: retain trusted-delegation requirements but explicitly mark this header sketch non-normative/unbound until F03 supplies a complete canonical assertion and verification protocol. That protocol needs receiver/audience, method/route and original-byte request binding (including downstream identity/operation/revision/approval/key where applicable), bounded pre-decode signature validation followed by correlation after strict decode, a distinct describe correlation route, and atomic bounded replay protection. No keyed mutation or caller-context federation may be advertised using today's static leaf credential. Do not import an independent tenant header; signed route assertions are trusted only after verification."
  },
  {
    "file": ".engineering/planning/review-result/wire-binding-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### WB-06 — P1: mandatory audit on every response is impossible under the declared failure cases\n\n`service/v1alpha2` §3.4, §4.7, §5 and §6 require an audit row and nonempty ref for every refusal, including failed admission and malformed envelopes. These cases have no verified tenant/principal or valid request_id/operation/input digest, yet §4.7 lists all as row fields. Audit write failure is itself a response case, so the always-present-ref requirement is unsatisfiable when the sink is unavailable. The read audit is described both as written before response and as failing before dispatch; a failure discovered only after success cannot retrospectively prove zero dispatch. Mutations additionally require a durable row before dispatch while the same row is described as append-only and final-outcome-bearing; outcome cannot be known then.\n\nMinimal correction: define pre-envelope refusal framing/correlation and unknown/unverified audit coordinates without trusting body data. Specify durable pre-dispatch audit admission and final outcome records separately (or precise update semantics), and a fail-closed audit-admission failure response that does not invent a persisted reference. After possible/known dispatch, audit completion failure must preserve the operation's actual known/unknown outcome rather than turn a performed mutation into unavailable/not_attempted. A transport failure may prevent any response. Record which audit guarantees are durable and which failure diagnostics are best-effort; no response can certify a nonexistent row."
  },
  {
    "file": ".engineering/planning/review-result/wire-binding-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "### WB-07 — P2: static-bearer context and executor ordering remain contradictory\n\nThe proposal correctly says a shared static bearer is not an individual caller (§1), but makes VerifiedContext tenant/principal/authority required and says only the credential supplies them (§3.1). Its local allow-all policy does not create trusted coordinates. The executor is similarly both supplied by admission and verified later against a grant after strict decode (§3.3/§4), while §2 historical notes suggest the body may introduce it. Mutation idempotency requires stable configured local absence and caller/origin identity, not rotation-sensitive token bytes.\n\nMinimal correction: define a receiver-owned static service principal and explicit optional local tenant/realm/authority semantics, scoped to the configured legacy/read profile. For governed calls, establish the authenticated principal independently; treat an executor field solely as a claim whose verified binding is constructed after policy/grant check, without allowing it to overwrite prior authority. If identity/audience verifiers mandate already verified executor identity, require exact match instead. Distinguish authentication result, supplied assertion, and verified execution context rather than claiming one complete context exists before decode."
  },
  {
    "file": "docs/design.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### WB-08 — P2: proposal documents falsely attribute a settled version decision to prior contracts\n\n`service/v1alpha2` §1 says indexing does not settle wire choice, but §7 says strict readers “make it a version bump, as the mutation profile §7 already decided.” The mutation profile calls optional descriptor fields additive and entertains upgrading readers first, then takes v1alpha2 as default; its §10 repeats the default. The CLI and stack proposal incorrectly cite `docs/design.md` §5 for “one wire version per client build,” while design actually requires explicit mutually supported version/profile selection and tested adapters. A new client can legitimately implement multiple explicit codecs without fallback.\n\nMinimal correction: one authoritative compatibility decision document should supersede these conflicting local defaults. Distinguish source-level optionality from old-reader wire compatibility; no added field is wire-compatible with the current closed reader. State one selected codec per interaction, not an invented ban on multi-codec client binaries. Keep proposed service envelope naming, adapter kind v2 and external consumer rollout separate. Record retained open release/organization decisions rather than implying the current task authorized them."
  }
]
```

