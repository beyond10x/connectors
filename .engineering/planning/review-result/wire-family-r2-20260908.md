---
format: aep.planning-md/1
id: review-result:wire-family-r2-20260908
kind: review-result
status: active
title: Independent family compatibility recheck
relations:
- reviews: story:contracts-wire-compatibility
revision: 1
---
# Independent compatibility recheck A — round 1

Date: 2026-09-08. Read-only recheck of the proposed E02 changes against source snapshots in `sources-r2/`; exact identities are in `source-hashes-r2.json` (27 files). No other review was read and no tracked source was changed. Runtime intentionally remains unimplemented. This report is immutable review evidence; subsequent fixes need a separate recheck.

Verdict: **ship with specification fixes**. The new common binding resolves the architectural compatibility gaps from the first review. Four remaining textual inconsistencies should be corrected before declaring E02 complete. These are specification defects, not observed runtime failures.

## Disposition of the original eight findings

| Original finding | Current evidence | Disposition |
|---|---|---|
| 1. Closed additions and no-change claims | compatibility §§1–4 define separate codecs, explicit paths and original-reader refusal; family §7 links supersede broad additive claims | Resolved, subject to the exact family inventory corrections below |
| 2. Full mutation classification/replay encoding | compatibility §5 separates classification, original identity, replay boolean, secondary cause and current disclosure admission; operations §3 uses it | Resolved at the proposed textual/value level; no codec execution claimed |
| 3. Catalog profile/auth composition | catalog §3 now selects singular profile and realization, with alternatives-array auth; compatibility §4 makes that normative | Resolved in principle; one auth example still violates the new required shape (A-R2-02) |
| 4. Session/media versioned binding | compatibility §6 explicitly makes duplex/control/data support unadvertisable until complete selected bindings exist, with new session errors separate from terminal reasons | Resolved as a transport/version disposition, without claiming those bindings exist |
| 5. Mediated route private/public boundary | mediated_route §3 now says private same-composition port; §7 and matrix map public advertising and child-visible errors without a generic forwarding endpoint | Resolved |
| 6. Generic HTTP bounds | new declared Operation.limits and generic 40/30/5 s execution/provider/connect ceilings | Resolved for generic reads; generic mutation/page selector remains ambiguous (A-R2-03) |
| 7. Separate artifact/schema compatibility | compatibility §1 and family/catalog links separate authoring-kind/configuration/bundle/index readers and refuse unknown forms | Resolved as independent-version disposition; no implicit format/schema upgrade |
| 8. Preserve exact old configured semantics | compatibility §3 excludes new validation/refresh/retry behavior from unchanged legacy projection and enforces a distinct projection revision/invoke set | Resolved |

## Remaining findings

### A-R2-01 — P2: The family matrix describes different record and series shapes from its cited owners

`contracts/service/compatibility.md:123` describes document `ref/mime/title/content/metadata` and query/filter/page shapes. The current records §3 actually defines input `id/representation/max_body_bytes` and output `item`, `body.{representation,bytes,content,truncated}`, `complete`, `provenance`, including provider version fields (`contracts/datasources/records/v1alpha1/semantics.md:29`). The matrix should not replace this with a different implied schema while claiming complete field disposition.

`compatibility.md:125` lists `promql-instant` alongside `promql-range` and describes tagged special/numeric values. The series owner still explicitly reserves `promql-instant` and `promql-labels` (`contracts/datasources/series/v1alpha1/semantics.md:12`), and encodes samples as numeric timestamps plus **string values**, including `NaN`/`+Inf`, without special-value tags (`:53`). The newly added family §7 repeats the incorrect tags wording (`:94`). The matrix omits the specifically named `warnings`, `series_truncated`, `samples_truncated` and window fields in favor of a generic truncation label.

Required fix: inventory the actual selected records/series inputs/results, preserve string sample values, and explicitly keep instant/labels reserved. Where `min_step_s`/`max_window_s` are advertised from the series declaration, identify their supported location (for example the selected input schema/profile), rather than implying new members of the closed five-field Operation.limits object.

Expected verification: compare the matrix rows mechanically/textually against the cited family §3 and profile tables; no invented field or promoted reserved profile remains.

### A-R2-02 — P2: A normative auth example omits the now-required scopes array

`compatibility.md:60` chooses one closed alternatives-array representation `{profile, scopes}` with scopes an array; the ESS `AuthAlternative` also requires it. `contracts/auth/profile/v1alpha1/semantics.md:72` still includes `{ "profile": "jira.api_token" }` without scopes. The compatibility prose is authoritative, but the provider declaration example would be rejected by its new representation and no omission/default rule is given.

Required fix: add `"scopes": []` to that alternative (or explicitly select and model an omission/default rule consistently). Keep catalog, authored profile and public descriptor shapes aligned.

Expected verification: every shown `requires_auth` alternative satisfies the selected required-field rule.

### A-R2-03 — P2: Generic mutation/page deadline selection is ambiguous after singular-profile normalization

`compatibility.md:63` correctly selects generic mutation as `profile: mutation` plus `realization: generic`, and generic paging as `profile: generic-http-page`. But `compatibility.md:136` assigns 20/15/5 s to configured/read **and mutation operation ceilings**, then grants 40/30/5 s specifically to the **generic-http profile**. `contracts/catalog/v1alpha1/semantics.md:147`–`:154` assigns the larger limits to generic execution generally. `contracts/service/v1alpha2/semantics.md:110` similarly contrasts ordinary versus generic profile without spelling out how a singular mutation profile selects the latter.

Required fix: explicitly determine whether the larger limits are selected by `realization: generic` for generic read, mutation and page operations, or choose another consistent rule. Update compatibility, service and catalog prose together. A caller/host should derive one ceiling for a generic mutation rather than guess whether its profile or realization wins.

Expected verification: show three descriptor cases—generic read, generic mutation, generic page—and one handwritten mutation; each has exactly one consistent request/result/execution/provider/connect tuple.

### A-R2-04 — P2: Acquisition §3 still presents secret completion as a forwardable management operation

The compatibility matrix correctly says protected entry/completion evidence is **not generic unary invoke input** (`compatibility.md:119`). However `contracts/auth/acquisition/v1alpha1/semantics.md:30` still introduces its list as host-admitted management operations on the adapter service, forwardable through federation, and lists `auth.complete(acquisition_ref, evidence)` with no dedicated-binding qualifier (`:34`). Its §7 later points to the correct compatibility rule. This leaves a security-relevant contradictory operation signature at the main type definition.

Required fix: qualify the §3 introduction/signature so begin/status are ordinary safe management payloads, completion is the dedicated protected callback/entry coordinator operation, and refresh remains private. This need not choose the future callback codec or resolve the sibling management-boundary story; it should merely avoid advertising secret-bearing completion through ordinary invoke.

Expected verification: a reader following the main operation list cannot infer that `auth.complete` secret evidence is an ordinary client-visible/federated unary input.

## Boundaries of this recheck

The rewritten service envelope's audit precondition/failure framing, static-principal distinction, executor assertion validation and unbound F03 delegation are materially clearer. Proposed ESS values correctly keep nullable-versus-omitted emission, cross-field predicates, decoder behavior and persisted ownership as explicit obligations. This recheck does not treat type validation or a common matrix as proof of the remaining semantic stories, delegated authority, callback protection, real audit persistence, session cutoff or live media framing.
