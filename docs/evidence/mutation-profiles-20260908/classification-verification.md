# Mutation classification verification — 2026-09-08

Owner: `story:contracts-mutation-classification` (F10/E06). Baseline: `c7cd05aab947d09992e55bc217aef5db3546f4ec`. This checkpoint changes semantic contracts and four private ESS value types. Runtime, public codecs, adapter-kind schemas and provider bindings are unchanged. Initial reviews also cover restart and visibility; those findings remain with their separate owners.

## Selected semantics and textual traces

The normative owners are [operations](../../../contracts/operations/v1alpha1/semantics.md#3-types), [sessions](../../../contracts/sessions/v1alpha1/semantics.md#4-rules-docsdesignmd406-418-settled-for-this-profile), [SIP](../../adapters/media-session.md#41-dial-effect-and-ready-session-result) and the [Atlassian operation map](../../adapters/atlassian.md#4-operation-map). The following are declared scenario expectations checked by textual review, **not executed conformance traces**. Unknown denotes lack of a definitive full effect outcome, and never proves absence of narrower known outreach.

| ID | Preconditions / sequence | Required observation |
|---|---|---|
| MC01 | Selected SIP declaration includes external_write/network/send_external/session_establishment and human_visible in semantic_effects | valid classification; normal current admission and approval still required |
| MC02 | Mutation declaration omits external_write | refuse declaration before advertisement or dispatch |
| MC03 | Read profile includes external_write or implementation can change business state | refuse incomplete/incompatible declaration; no read bypass |
| MC04 | Bounded HTTP provider query with network only | remains read; transport use is not the mutation discriminator |
| MC05 | Human visibility only, caller attempts mutation authority | descriptive hint cannot grant authority or satisfy discriminator |
| MC06 | Unknown executable effect or human_visible placed in effects | refuse closed vocabulary violation |
| MC07 | send_external placed in semantic_effects | refuse wrong vocabulary; old SIP metadata is deliberately remapped |
| MC08 | Known but unsupported effect, incomplete selected binding set, duplicate member | refuse before advertisement/execution; enum membership alone is insufficient |
| MC09 | Correct declaration, missing current grant or execution approval | current admission refuses; metadata is not dispatch permission |
| MC10 | Preflight or admission fails; gate definitely never opened | not_attempted with specific safe cause; no ready receipt; spent claim remains spent |
| MC11 | Definite protocol evidence proves no effect across entire attempted dial | refused with safe provider cause; no ready receipt |
| MC12 | Provisional ringing, then final rejection/CANCEL; no definitive establishment | partial/unknown with outcome_unknown; final status cannot prove no outreach |
| MC13 | Gate may open, no response or process crashes | unknown; no inferred handle, resend or false pre-gate result |
| MC14 | Exact SIP leg established, application connection fails | applied plus unavailable; no ready handle |
| MC15 | Exact SIP leg established, application authority/media readiness refuses | applied plus applicable safe session error; no ready handle |
| MC16 | SIP established, terminal wins before complete readiness | applied plus offer_rejected/applicable safe error; no ready receipt |
| MC17 | SIP/application/streams ready and admitted; ready decision wins | applied plus safely deliverable ready result; one historical receipt |
| MC18 | Ready decision wins; terminal observed before result encoding | applied plus applicable terminal error: revoked, lease_expired, session_not_ready for confirmed close/hangup, or session_lost only for continuity loss; no usable handle or revoked one-use authority |
| MC19 | Ready result delivered, then close/revoke/expiry | original applied outcome stays settled; separate first session terminal and cutoff govern use |
| MC20 | Host knows establishment, terminal ledger write fails | preserve live applied knowledge; diagnostics report storage failure; recovery without evidence is unknown |
| MC21 | Reply lost despite known/durable outcome | caller is uncertain; loss never permits automatic redial |
| MC22 | Owner restart or later session loss | no reattach; no retroactive rewrite of durable applied outcome; recovery without outcome proof is unknown |
| MC23 | Current result authority fails after effect | ordinary safe denial without mutation metadata; absence is not not_attempted |
| MC24 | Deliberate new sip.dial invocation after failure | fresh admission/approval and attempt; another call is possible, never automatic |
| MC25 | Private schema accepts read+external_write, missing discriminator, duplicates or unknown+ready receipt | semantic counterexamples remain rejected by normative rules; schema acceptance does not execute those rules |

The SIP commitment is newly selected normative meaning: trusted proof that the exact admitted SIP leg established. It is separate from full application/stream readiness. The predecessor README, frozen at `81459ac42ddd518d3942f4b079841e9e0ed6efc8`, proves sequencing and the single full-ready receipt only. This checkpoint does not claim a particular SIP response/status or sipx method proves commitment or no-effect refusal. A concrete adapter binding must supply that evidence rule and tests before advertisement. No provider operation was invoked.

## ESS and checks actually run

Pinned ESS: `.local/toolchains/ess/0.20.0/bin/ess` (0.20.0). Baseline validation/compilation: 13 files, 204 declarations. Revised: 13 files, 208 declarations. Four added values are ExecutableEffect, SemanticEffect, EffectDeclaration and SessionEstablishmentObservation, in the existing mutations domain. No new entity, lifecycle, entity field or ownership relation is asserted. OperationDeclaration metadata remains UNMAPPED; ready_session is an optional private receipt reference, not public null encoding, a live lease or a persistent Session/Attempt relation.

Two independent `ess generate --path ess --kind schema` runs produced **215 artifacts with identical ordered paths and bytes**; [projection hashes](classification-projection-hashes.json) cover all of them. The four selected schema files under classification-schema/types are unmodified compiler output. [27 values](classification-values.json) were checked against these schemas with Python jsonschema: **7 shape negatives** refused and **4 semantic counterexamples** intentionally passed shape checking. [Actual shape results](classification-shape-results.json) and the [one-off check transcript](classification-check-transcript.md) preserve what was executed. No classifier, provider protocol, clock, dispatch, readiness or terminal-race predicate was executed by this check.

The existing project gate ran once, exit 0:

```console
env TMPDIR="$PWD/.local/tmp" CARGO_BUILD_JOBS=2 cargo run --locked --offline -p connectors-build -- --ess "$PWD/.local/toolchains/ess/0.20.0/bin/ess" gate --msrv
```

[Full gate log](classification-gate.log): 50 existing Rust tests passed; Rust 1.88.0 workspace/all-target check passed; ESS compiled 222 scenarios including 34 authored ones with no refusals. These are existing regression and compilation checks, not implementation of the new contracts. The gate does not collect the sessions scenario directory, so separate author/synthesize commands checked it: **13 authored cases / 201 total compiled scenarios**, no refusals. Their exact JSON is preserved in gzip alongside hashes in [compiled evidence](classification-compiled-evidence.json). Author/synthesize do not execute sequential scenario traces.

## Review evidence boundary

Both initial reports and all their exact source snapshots are preserved under reviews/. [A's report](reviews/a-initial-report.md) and [B's report](reviews/b-initial-report.md) are unchanged. All **125 archived source entries** match their own byte/hash manifests; [audit](initial-source-audit.json). AEP holds the exact raw reports as immutable review-result records. Their classification findings are MP-A-01/02 and MP-B-01/02. Remaining restart/visibility findings are not closed by this checkpoint. Final reviewer dispositions and completion accounting are recorded separately so mutable planning does not change reviewed normative bytes.
