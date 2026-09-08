---
format: aep.planning-md/1
id: review-result:wire-binding-r1-20260908
kind: review-result
status: active
title: Independent service binding review, first pass
relations:
- reviews: story:contracts-wire-compatibility
revision: 1
---
# Independent service wire compatibility review B

Baseline: `3ba2d29870d577aac70c8a68ee901b3b7c6c99ac`, clean tracked checkout at inspection. Source hashes are in `source-hashes.json`. Reviewed source and existing test assertions; no new runtime tests or implementation written. I did not read reviewer A's report. Scope is E02 with related unresolved governance/delegation contradictions that must not be represented as completed bindings.

Verdict: **revise before freezing the wire semantics**. The implemented read-only service remains a usable, strict v1alpha1 profile. The proposed binding is not an executable or fully specified replacement. Strictness makes the current additions incompatible, but does not itself dictate the spelling of the successor version or approve rollout.

## Authoritative current behavior

- `WIRE_VERSION` is `v1alpha1`. `Operation`, `Descriptor`, `Invocation`, nested `Error`, and flattened response alternatives are closed shapes. `ErrorCode` is a closed 13-value enum (`crates/connectors-core/src/lib.rs:7–36,59–153`). Arbitrary JSON exists only within operation input/output schemas and values; it is not an extension bag for envelope authority.
- Authenticated `GET /v1/describe` has no version-bearing request envelope, query selector or negotiation header; the handler unconditionally returns the adapter descriptor. Authentication failures return a **bare Error**, not Response (`server.rs:37–41,66–70,173–175`). The client decodes the strict Descriptor **before** checking its version (`client/lib.rs:71–93`). Unknown descriptor/operation fields therefore become `upstream_protocol`; a shape-compatible different version becomes `unsupported`.
- After admission, invoke decodes the strict Invocation. Bad/unknown fields return bare `invalid_input`; successfully decoded requests then check version and return a correlated **v1alpha1 Response** with `unsupported` before dispatch (`server.rs:73–114`). Existing `wire_admission_and_freshness_refuse_before_dispatch` explicitly asserts a version-only `v99` request returns `Unsupported` and zero calls (`tests/service.rs:97–103`).
- Current client parses response fields before correlation/version; added response fields and unknown ErrorCode values are rejected. Failure may emerge as generic HTTP-derived `invalid_input`/`unauthorized`/`forbidden`/`capacity`, or `upstream_protocol`; this does not safely identify a version mismatch (`client/lib.rs:123–138,174–186`). The old client has no general mapping from `invalid_input` to `unsupported`.
- Federation forwards via a freshly generated leaf Invocation under configured service credentials; it does not carry caller context, approval, idempotency key, input admission evidence, or audit references. Its public projection clones current operations and prefixes IDs; revision guards and stale refresh do not retry invocation (`federation.rs:119–195`).
- Adapter specification kind v1/v2, service wire v1alpha1/proposed successor, family/profile versions, CLI/repository “v2”, and old operation v0alpha* versions are distinct version axes (`docs/design.md`, §5). No unchanged kind file grants wire compatibility.

## Findings

### WB-01 — P1: no concrete version selection for describe or dual projections

`service/v1alpha2` §1 and §4.10 keep `/v1/describe` and `/v1/invoke`, promise optional old projection, and require one selected version per client. GET has no envelope and the proposal never defines how an old/new request selects the descriptor codec. An old client cannot express the preference that would cause an old-shape projection. Its version check is too late to reject a new descriptor cleanly. The §6 promise “v1alpha1 client against governed instance → unsupported” is not established by current client behavior.

Minimal semantic correction: freeze current routes and legacy codec unchanged; choose an explicit endpoint/binding selector for the proposed codec (a separate configured base/route is sufficient), with one precise default/no-selector rule, exact unsupported request/response treatment and no fallback/resend. A new client may explicitly implement the legacy codec, but must select it before requesting describe, and must not claim it received governance fields. If final version spelling remains operator-owned, use a named proposed binding and mark its release identifier pending instead of treating the filename as approval.

### WB-02 — P1: mixed-version failure claims contradict the actual strict decoder

`service/v1alpha2` §4.10 says every v1alpha2 invocation to v1alpha1 yields invalid_input, and “the client reports it as unsupported.” A version-only request in the current old shape instead receives enveloped unsupported. Added-field requests receive bare invalid_input. Neither is distinguishable from other malformed requests by blindly remapping every invalid_input. Old-reader/new-error failures also depend on whether failure is enveloped or bare and which HTTP status is used.

Minimal correction: compatibility vectors must separately cover same shape/different version, added descriptor and operation fields, added invocation fields, added success/error fields, new closed codes, and pre-envelope bare errors. Record server wire bytes/shape versus client-visible code separately; never promise a diagnostic the already released reader cannot generate. New readers must reject correlation/version mismatches and unknown fields rather than guess or resend.

### WB-03 — P1: mutation effects have no unambiguous complete response encoding

`operations` §3.4, §4 and §8 require independently observable effect knowledge (`not_attempted`, `refused`, `applied`, `unknown`) and safe error cause, including known provider success/refusal when ledger persistence fails. Its illustrative examples use `effect: replayed`, which confuses replay provenance with effect classification. `service/v1alpha2` §3.4 merely says mutation outcome fields belong to the profile and supplies only result/error plus audit_ref. The codec therefore cannot presently express all settled F01 outcomes without inference. Both-audit-reference federation output is also promised without a field shape.

Minimal correction: the E02 matrix must select exact typed mutation observation fields, distinguish original effect from replay metadata and original request/attempt identity, and define shape/status/code consistency. Unknown uses outcome_unknown even when diagnostic cause is timeout. A successful replay keeps the original `applied` classification, with replay represented separately. Known provider refusal must preserve its cause/classification when terminal write fails. Define gateway and leaf audit references separately (or mark the delegated binding unavailable until its exact schema is settled). Do not derive effect knowledge from HTTP status or generic timeout.

### WB-04 — P1: legacy projection filtering does not prevent direct downgrade invocation

`service/v1alpha2` §4.10 hides operations only when approval is required or effects include external_write. `operations` §7 requires hiding mutation operations entirely. Today classification also admits process, local_system, send_external and session_establishment effects; absence of external_write is not evidence of a safe legacy read. Hiding an operation in describe alone does not prevent a caller constructing its ID directly, and no rule binds legacy invoke to the projected operation set plus its exact projection revision. “No other loss” in §7 is unsupported for errors, audit, connection selection, executor and descriptor curation.

Minimal correction: default projection off. Permit only an explicit verified unchanged configured/read subset under static-bearer; require the legacy invoke handler to reject every operation outside that exact projected subset before any provider call, even with a guessed operation ID/current full descriptor revision. Legacy descriptors, revisions, schemas, error vocabulary and responses need an explicit codec and validation; mutation/managed/governed/session operations and semantics that cannot survive projection are refused, not silently stripped. Gateway advertisement must be the support intersection for the selected codec at every hop.

### WB-05 — P1, F03 dependency: delegated context cannot yet bind the request it authenticates

The proposed header (§3.6) signs context and request_id but not destination leaf, method/path, body/input digest, operation/connection or revision. A captured valid assertion can accompany different semantics under the same first-used request ID; “one request id” is not body integrity. Request ID is inside the body, yet the text claims verification/binding before any body decoding. GET describe has no downstream request ID field. Per-route key selection, canonical encoding, one-use replay storage/atomicity, duplicate header handling, clock uncertainty and key rotation are unspecified. The §5 one-use promise requires more than HMAC verification and a timestamp.

Minimal correction: retain trusted-delegation requirements but explicitly mark this header sketch non-normative/unbound until F03 supplies a complete canonical assertion and verification protocol. That protocol needs receiver/audience, method/route and original-byte request binding (including downstream identity/operation/revision/approval/key where applicable), bounded pre-decode signature validation followed by correlation after strict decode, a distinct describe correlation route, and atomic bounded replay protection. No keyed mutation or caller-context federation may be advertised using today's static leaf credential. Do not import an independent tenant header; signed route assertions are trusted only after verification.

### WB-06 — P1: mandatory audit on every response is impossible under the declared failure cases

`service/v1alpha2` §3.4, §4.7, §5 and §6 require an audit row and nonempty ref for every refusal, including failed admission and malformed envelopes. These cases have no verified tenant/principal or valid request_id/operation/input digest, yet §4.7 lists all as row fields. Audit write failure is itself a response case, so the always-present-ref requirement is unsatisfiable when the sink is unavailable. The read audit is described both as written before response and as failing before dispatch; a failure discovered only after success cannot retrospectively prove zero dispatch. Mutations additionally require a durable row before dispatch while the same row is described as append-only and final-outcome-bearing; outcome cannot be known then.

Minimal correction: define pre-envelope refusal framing/correlation and unknown/unverified audit coordinates without trusting body data. Specify durable pre-dispatch audit admission and final outcome records separately (or precise update semantics), and a fail-closed audit-admission failure response that does not invent a persisted reference. After possible/known dispatch, audit completion failure must preserve the operation's actual known/unknown outcome rather than turn a performed mutation into unavailable/not_attempted. A transport failure may prevent any response. Record which audit guarantees are durable and which failure diagnostics are best-effort; no response can certify a nonexistent row.

### WB-07 — P2: static-bearer context and executor ordering remain contradictory

The proposal correctly says a shared static bearer is not an individual caller (§1), but makes VerifiedContext tenant/principal/authority required and says only the credential supplies them (§3.1). Its local allow-all policy does not create trusted coordinates. The executor is similarly both supplied by admission and verified later against a grant after strict decode (§3.3/§4), while §2 historical notes suggest the body may introduce it. Mutation idempotency requires stable configured local absence and caller/origin identity, not rotation-sensitive token bytes.

Minimal correction: define a receiver-owned static service principal and explicit optional local tenant/realm/authority semantics, scoped to the configured legacy/read profile. For governed calls, establish the authenticated principal independently; treat an executor field solely as a claim whose verified binding is constructed after policy/grant check, without allowing it to overwrite prior authority. If identity/audience verifiers mandate already verified executor identity, require exact match instead. Distinguish authentication result, supplied assertion, and verified execution context rather than claiming one complete context exists before decode.

### WB-08 — P2: proposal documents falsely attribute a settled version decision to prior contracts

`service/v1alpha2` §1 says indexing does not settle wire choice, but §7 says strict readers “make it a version bump, as the mutation profile §7 already decided.” The mutation profile calls optional descriptor fields additive and entertains upgrading readers first, then takes v1alpha2 as default; its §10 repeats the default. The CLI and stack proposal incorrectly cite `docs/design.md` §5 for “one wire version per client build,” while design actually requires explicit mutually supported version/profile selection and tested adapters. A new client can legitimately implement multiple explicit codecs without fallback.

Minimal correction: one authoritative compatibility decision document should supersede these conflicting local defaults. Distinguish source-level optionality from old-reader wire compatibility; no added field is wire-compatible with the current closed reader. State one selected codec per interaction, not an invented ban on multi-codec client binaries. Keep proposed service envelope naming, adapter kind v2 and external consumer rollout separate. Record retained open release/organization decisions rather than implying the current task authorized them.

## Required compatibility verification rows

1. Old decoder, unchanged legacy descriptor/outcome: accepted; schemas/limits/correlation preserved.
2. Old decoder, legacy shape with different version: describe unsupported; invoke server unsupported before dispatch; response mismatch upstream_protocol.
3. Old decoder, any added descriptor/operation field: upstream_protocol before old client reaches version comparison.
4. Old service, invocation with new unknown field: bare invalid_input before dispatch; do not remap all malformed input as version mismatch.
5. Old decoder, new response field/error code: decode refusal with actual status-derived fallback classification recorded; no result or retry authority inferred.
6. New reader explicitly selected legacy binding: legacy semantics only; unavailable capability/profile refused.
7. New binding/profile selected on unsupported endpoint: explicit refusal, no fallback/resend or credential switching.
8. Legacy projection hides an operation, and forged direct invoke names it: reject before dispatch.
9. Same operation under new/current projection revisions or caller policies: stale/authorization checks precede dispatch; one projection cannot authorize another.
10. Mutation success, known refusal, pre-dispatch failure, post-gate unknown, replay of each terminal outcome, uncertain waiter, and terminal-write failure: exact safe classification/cause/replay metadata remain distinct.
11. Gateway/leaf version/profile mismatch, dropped downstream response and separate audit provenance: honest refusal/unknown outcome, no resend and no authority substitution.
12. Admission/decode/audit-sink failure before a correlated response exists: safe defined transport refusal without fabricated trusted coordinates or persisted references.

E02 can settle these version/codec obligations without implementing runtime. F03 and the governance/audit modeling owners may retain unresolved mechanics only when the proposed features are explicitly unbound and unadvertisable, rather than simultaneously promised as a complete wire contract.
