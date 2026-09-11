---
format: aep.planning-md/1
id: verification-report:findings-wire-family-r2-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:wire-family-r2-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: f2d9a72e4c4f6794399534574de74448768188e789ac5973c9c561acfe59f4bb
relations:
- verifies: review-result:wire-family-r2-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:wire-family-r2-20260908

This supplements [the immutable original](../review-result/wire-family-r2-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

## Remaining findings

## Transcription method

4 findings remain in the scope of this report's final stated conclusion.
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
    "file": "contracts/service/compatibility.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### A-R2-01 — P2: The family matrix describes different record and series shapes from its cited owners\n\n`contracts/service/compatibility.md:123` describes document `ref/mime/title/content/metadata` and query/filter/page shapes. The current records §3 actually defines input `id/representation/max_body_bytes` and output `item`, `body.{representation,bytes,content,truncated}`, `complete`, `provenance`, including provider version fields (`contracts/datasources/records/v1alpha1/semantics.md:29`). The matrix should not replace this with a different implied schema while claiming complete field disposition.\n\n`compatibility.md:125` lists `promql-instant` alongside `promql-range` and describes tagged special/numeric values. The series owner still explicitly reserves `promql-instant` and `promql-labels` (`contracts/datasources/series/v1alpha1/semantics.md:12`), and encodes samples as numeric timestamps plus **string values**, including `NaN`/`+Inf`, without special-value tags (`:53`). The newly added family §7 repeats the incorrect tags wording (`:94`). The matrix omits the specifically named `warnings`, `series_truncated`, `samples_truncated` and window fields in favor of a generic truncation label.\n\nRequired fix: inventory the actual selected records/series inputs/results, preserve string sample values, and explicitly keep instant/labels reserved. Where `min_step_s`/`max_window_s` are advertised from the series declaration, identify their supported location (for example the selected input schema/profile), rather than implying new members of the closed five-field Operation.limits object.\n\nExpected verification: compare the matrix rows mechanically/textually against the cited family §3 and profile tables; no invented field or promoted reserved profile remains.",
    "line": 123
  },
  {
    "file": "contracts/auth/profile/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### A-R2-02 — P2: A normative auth example omits the now-required scopes array\n\n`compatibility.md:60` chooses one closed alternatives-array representation `{profile, scopes}` with scopes an array; the ESS `AuthAlternative` also requires it. `contracts/auth/profile/v1alpha1/semantics.md:72` still includes `{ \"profile\": \"jira.api_token\" }` without scopes. The compatibility prose is authoritative, but the provider declaration example would be rejected by its new representation and no omission/default rule is given.\n\nRequired fix: add `\"scopes\": []` to that alternative (or explicitly select and model an omission/default rule consistently). Keep catalog, authored profile and public descriptor shapes aligned.\n\nExpected verification: every shown `requires_auth` alternative satisfies the selected required-field rule.",
    "line": 72
  },
  {
    "file": "contracts/catalog/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### A-R2-03 — P2: Generic mutation/page deadline selection is ambiguous after singular-profile normalization\n\n`compatibility.md:63` correctly selects generic mutation as `profile: mutation` plus `realization: generic`, and generic paging as `profile: generic-http-page`. But `compatibility.md:136` assigns 20/15/5 s to configured/read **and mutation operation ceilings**, then grants 40/30/5 s specifically to the **generic-http profile**. `contracts/catalog/v1alpha1/semantics.md:147`–`:154` assigns the larger limits to generic execution generally. `contracts/service/v1alpha2/semantics.md:110` similarly contrasts ordinary versus generic profile without spelling out how a singular mutation profile selects the latter.\n\nRequired fix: explicitly determine whether the larger limits are selected by `realization: generic` for generic read, mutation and page operations, or choose another consistent rule. Update compatibility, service and catalog prose together. A caller/host should derive one ceiling for a generic mutation rather than guess whether its profile or realization wins.\n\nExpected verification: show three descriptor cases—generic read, generic mutation, generic page—and one handwritten mutation; each has exactly one consistent request/result/execution/provider/connect tuple.",
    "line": 147
  },
  {
    "file": "contracts/auth/acquisition/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### A-R2-04 — P2: Acquisition §3 still presents secret completion as a forwardable management operation\n\nThe compatibility matrix correctly says protected entry/completion evidence is **not generic unary invoke input** (`compatibility.md:119`). However `contracts/auth/acquisition/v1alpha1/semantics.md:30` still introduces its list as host-admitted management operations on the adapter service, forwardable through federation, and lists `auth.complete(acquisition_ref, evidence)` with no dedicated-binding qualifier (`:34`). Its §7 later points to the correct compatibility rule. This leaves a security-relevant contradictory operation signature at the main type definition.\n\nRequired fix: qualify the §3 introduction/signature so begin/status are ordinary safe management payloads, completion is the dedicated protected callback/entry coordinator operation, and refresh remains private. This need not choose the future callback codec or resolve the sibling management-boundary story; it should merely avoid advertising secret-bearing completion through ordinary invoke.\n\nExpected verification: a reader following the main operation list cannot infer that `auth.complete` secret evidence is an ordinary client-visible/federated unary input.",
    "line": 30
  }
]
```

