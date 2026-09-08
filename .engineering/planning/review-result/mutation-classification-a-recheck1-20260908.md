---
format: aep.planning-md/1
id: review-result:mutation-classification-a-recheck1-20260908
kind: review-result
status: active
title: Mutation classification reviewer A recheck 1
relations:
- reviews: story:contracts-mutation-classification
revision: 1
---
# Independent classification recheck A — F10/E06

Verdict: **approve story:contracts-mutation-classification only**. **Zero residual/new classification findings** (0 P0/P1/P2/P3). Initial classification findings **MP-A-01 and MP-A-02 are addressed** in the reviewed bytes. This is not approval of the restart/idempotency or visibility stories, or of the full three-story cluster.

## Frozen scope and evidence

Before substantive analysis, 51 exact normative/evidence inputs were frozen under `sources/` and `old-source/`; `source-hashes.json` records their SHA-256 values and sizes. The freeze includes operations, sessions, media/SIP, Atlassian, service owners, design, ESS, unchanged legacy types/server/adapter schemas, the classification verification packet and its compressed compilation evidence, and reviewer A's own initial archive. Old SIP sources are pinned at `81459ac42ddd518d3942f4b079841e9e0ed6efc8`.

Two 215-file projection trees were additionally frozen before byte comparison, with 430 entries in `projection-source-hashes.json`. No peer report/archive was opened. Mutable AEP, dispositions, checkpoint and completion bookkeeping are outside the verdict. This review made no tracked edits and did not rerun runtime suites or contact a provider.

## Semantic assessment

**MP-A-01 resolved.** Operations §3 now requires external_write for possible external business changes, including call initiation/session establishment, and rejects incompatible read declarations. It distinguishes transport-only network use and protocol-internal setup from business effects; closed distinct executable and semantic arrays are required by the selected extended descriptor. Unsupported/incomplete effect declarations remain an advertisement/execution refusal. SIP's disposition, contract table and operation table consistently use `[external_write, network, send_external, session_establishment]` with `human_visible` confined to semantic_effects. The pinned predecessor's different fields are explicitly remapped, not claimed to be the current declaration. The Atlassian map separates executable categories from descriptive hints and explains its read-summary shorthand. None of this metadata grants admission, approval or dispatch authority.

**MP-A-02 resolved.** Operations §4.1, sessions §§4/4.1 and SIP §4.1 now distinguish three facts: definitive exact SIP-leg establishment, a full ready application/stream receipt, and current session authority/liveness. Confirmed establishment supplies applied even if application binding, readiness or safe result delivery later fails. Partial ringing without definitive establishment/no-effect evidence remains unknown, preserving the fact that narrower outreach may already have happened. A final rejection or CANCEL/BYE does not prove no effect. Pre-gate non-dispatch and complete no-effect refusal retain their separate criteria.

Readiness and terminal decisions are serialized; a terminal winning first prevents a ready receipt, while a later terminal cannot retract historical receipt/effect knowledge. Current disclosure and safe delivery can still suppress a usable handle, with applied-plus-error when knowledge warrants it. First terminal, existing data cutoff/teardown and no-reattach rules remain intact. Response loss, recovery without durable knowledge and later session loss cannot authorize redial or revive approval. Durable mutation outcomes retain the existing first-settled rule; neither session bookkeeping nor cleanup rewrites them.

The concrete SIP evidence predicate is deliberately a future binding prerequisite. The old voice-runtime README supports the separate phases and one full-ready receipt; the revised text correctly does not infer that a particular SIP status, sipx method, socket or generic transport success proves commitment. This limitation is sufficient for the requested normative selection and is not a claim of an implemented provider guarantee.

## Independent verification

`independent-checks.json` and `independent-model-checks.json` record the checks:

- **27/27 generated-shape expectations** matched recorded results: **7 rejected shapes**, **4 intentionally accepted semantic counterexamples**. Their interpretation is correctly limited to shapes, including private optional omission and the unexecuted applied/ready-receipt relationship.
- Both complete projections contain **215 identical paths/bytes**, matching the recorded manifest; all **4 selected schema copies** are unchanged compiler output.
- The mutations domain adds exactly the four stated value types. Existing entity fields, identities, relationships, lifecycles and commands are unchanged; all other ESS files remain unchanged against the frozen initial baseline. Legacy core/server and adapter-kind schema/semantics bytes reviewed here are unchanged.
- All three compressed compilation artifacts match their exact uncompressed hashes/sizes. Separate session artifacts contain **13 authored / 201 total compiled scenarios**. The frozen gate records **50 passing existing Rust tests**, successful MSRV 1.88 check, **13 ESS files / 208 declarations** and **222 scenarios including 34 authored**, with no refusals.
- All **25 textual classification scenarios** agree with the selected rules. They are accurately described as textual expectations rather than executed classification, SIP, storage, authority, readiness or race tests.
- Reviewer A's **57 archived initial source entries** match their own manifests and the recorded aggregate audit. The reported **125-entry total** is supported by the root's preserved transcript/audit; the other reviewer's archive was deliberately not independently reopened in this lane. This is a reviewer-isolation limit, not an allegation of incorrect provenance.

## Remaining scope

Initial **MP-A-03–06** remain with `story:contracts-restart-idempotency`, including the broad none wording still present at operations §3, Docker per-operation and stable-target claims, and Kubernetes precondition/replay semantics. **MP-A-07** remains with `story:contracts-mutation-visibility`. Their presence in shared files does not receive approval from this classification-only verdict and is not a demand to close them in this checkpoint.

Runtime classifier enforcement, complete authored descriptor support, exact SIP effect/refusal proof, real provider/dispatch/clock/storage/session races and source-specific binding conformance remain explicit prerequisites before advertising the proposed operation. The evidence and four private value shapes do not claim to implement them.
