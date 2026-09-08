---
format: aep.planning-md/1
id: review-result:mutation-classification-b-recheck1-20260908
kind: review-result
status: active
title: Mutation classification reviewer B recheck 1
relations:
- reviews: story:contracts-mutation-classification
revision: 1
---
# Independent reviewer B — mutation classification recheck 1

Verdict: **NEEDS REVISION: one residual P2 finding, MP-B-C01** (P0/P1/P2/P3: 0/0/1/0).

Scope is only `story:contracts-mutation-classification` (F10/E06; initial MP-B-01/02). Restart/idempotency and mutation visibility remain pending with their separate owners; this report neither rechecks nor approves those stories. The complete supplied classification packet was frozen before review: 39 normative/source/evidence files under `snapshot/`, identified by `source-hashes.json`. All frozen bytes match their manifest; all live files still matched at evidence-audit completion. Other reviewer reports/archives and mutable AEP/ledger/dispositions/checkpoint material were excluded from review.

## Initial finding disposition

**MP-B-01 is closed.** Operations §3 selects external_write as the discriminator for business mutations, including intentional outbound communication/session establishment, while network alone may remain a read. Separate required, closed, distinct-member effect arrays prevent human_visible from being executable authority. SIP consistently uses external_write/network/send_external/session_establishment and descriptive human_visible; its old vocabulary is accurately disclosed as remapped. Atlassian separates semantic hints from executable effects, explains read summaries, and includes network for HTTP operations. Completeness/support/profile compatibility are explicit future validation predicates, not inferred from enum membership. The current adapter-kind/public codecs remain unchanged; required extended descriptor fields are no longer described as optional legacy additions.

**MP-B-02's main outcome defect is closed, with the narrow new error-mapping residual below.** Operations §4.1, sessions §4 and SIP §4.1 independently select exact confirmed SIP-leg establishment as applied effect knowledge and full application/stream readiness as the ready-receipt boundary. Provisional ringing without establishment remains partial/unknown; a final rejection, CANCEL/BYE or absent handle does not prove no business effect. A proved establishment followed by application/authority/media failure retains applied with a safe error and no fabricated handle. The serialized ready/terminal decision preserves a historical receipt without granting future liveness. Current result authority, safe encoding, lease/revocation, storage/caller/recovery viewpoints, first terminal facts, no reattach and no automatic redial remain intact. Detailed SIP protocol proof is accurately an unimplemented binding prerequisite; neither the old sequencing nor ESS acceptance is claimed to provide it.

## MP-B-C01 — Known session termination is mislabeled as session loss

**P2. Sole owner: story:contracts-mutation-classification.**

The new SIP table (`docs/adapters/media-session.md:85`) selects **session_lost for every terminal** observed after readiness and before result encoding. The verification record's MC18 repeats it. That trigger includes confirmed local_close/remote_hangup, revoked and lease_expired, not only continuity loss.

The sessions owner explicitly distinguishes lost continuity from known terminal reasons and confirmed closed resource release (`contracts/sessions/v1alpha1/semantics.md:47`, §§4–4.1). The new delivery boundary should not discard a known terminal cause by presenting every such session as lost. Mutation applied knowledge is correctly retained and must remain so.

**Required correction:** retain applied and withhold the usable handle; choose the applicable existing safe session error for the observed terminal fact (for example revoked, lease_expired or session_not_ready). Use session_lost only when continuity is actually lost. Preserve the first terminal and historical receipt, and update MC18. No new code, state, wire field, protocol or runtime test is required. The exact finding was frozen separately in `finding-MP-B-C01.md` before any source correction.

## Evidence independently checked

`evidence-audit.json` and `auxiliary-hashes.json` record the bounded checks performed:

- Re-evaluated all **27** supplied values against the four copied Draft 2020-12 schemas; results exactly match the reported decisions and error messages: **7 rejected shapes, 4 accepted semantic counterexamples**. Missing discriminator, read+write, duplicate effects and unknown+ready are honestly unexecuted cross-field predicates.
- Both existing projections match all **215** ordered paths, sizes and SHA256 values; all four selected schema copies match exactly. Archived revised IR adds precisely the four stated mutations value types; old types/entities/commands/lifecycles remain unchanged (only domains/types change).
- Decompressed and checked all three committed compilation archives against their exact claimed hashes and byte counts. The separate session set contains **13 authored / 201 synthesized** entries, with matching provenance and each authored entry unchanged in the suite.
- Inspected the supplied gate log: **50 existing Rust tests**, Rust **1.88.0** workspace/all-target check, **13 ESS files / 208 declarations**, and **222 compiled scenarios including 34 authored**, all successful. No runtime suite or gate was rerun by this reviewer.
- Read all **25** textual classification expectations against the frozen normative text; MC18 carries the residual above. These remain declared expectations, not executed classification/SIP/readiness/terminal traces.
- Rechecked reviewer B's own archived initial packet: **68 source entries** match their manifests and the archived B report matches the immutable initial report. The other reviewer's archive/report was not inspected. The verification document's aggregate 125-source claim remains the root's recorded audit; this independent review does not claim to have repeated its peer-archive portion.

## Remaining scope limits

The four ESS types are private semantic values. OperationDeclaration metadata, public encoding, trusted SIP effect evidence, full readiness, live authority, atomic outcome/terminal ordering and receipt disclosure are not implemented or proven by those shapes. No new persistent Session/Attempt relation or mutation lifecycle is invented. Legacy core/host/client/schema behavior is unchanged. Existing none/natural definitions, Docker/Kubernetes repeat semantics and the visibility matrix remain with pending restart/visibility stories and do not expand this classification recheck. After MP-B-C01 is corrected, a narrow confirmation of the changed normative row and MC18 is sufficient for this review's remaining concern.
