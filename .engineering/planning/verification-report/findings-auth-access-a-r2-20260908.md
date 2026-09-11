---
format: aep.planning-md/1
id: verification-report:findings-auth-access-a-r2-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:auth-access-a-r2-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 14beab12c1911754e1bca08809676184644e2fcd960b46011d2850327938ce76
relations:
- verifies: review-result:auth-access-a-r2-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:auth-access-a-r2-20260908

This supplements [the immutable original](../review-result/auth-access-a-r2-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

**Verdict: needs revision. Remaining findings: P0 0 / P1 0 / P2 3 / P3 0.** This is an immutable intermediate review of the revised normative packet, before the final evidence packet. It does not supersede the initial review. No other reviewer output was consulted, no tracked/planning file was changed, and no runtime test was run.

The author is still assembling the typed/textual verification packet. This pass reviewed normative declarations and the auth_access model without executing a reducer, provider query, cache, coordinator, transport, or runtime test. Generic schema shapes intentionally leave cross-field access combinations, required/forbidden flow fields, numeric limits, current authority/time, budget consumption and completeness predicates UNMAPPED. A final verdict must separately assess the completed evidence and the three source corrections above.

## Transcription method

3 findings remain in the scope of this report's final stated conclusion.
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
    "file": "docs/adapters/grafana.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## AP-A-R2-01 — P2 — The active monitoring contract map still selects the retired none profile\n\n**Sources:** `docs/adapters/grafana.md:49-50`, versus `:74-79` and `contracts/auth/profile/v1alpha1/semantics.md:101-110`.\n\nThe contract map currently selects `<x>.none` for both direct and mediated access and lists only bearer/basic for direct capabilities. Section 5 now says `<x>.none` is not an accepted alias and selects `.anonymous`/`http-anonymous` and `.via_parent`/`mediated-http` separately. The active adapter declaration table and its detailed profile table therefore prescribe different accepted profiles and credential placement.\n\n**Required correction:** update the active contract map to the four selected profiles and their distinct capabilities/placement. Keep historical names only in explicitly historical evidence. Check the adjacent configured-first/acquisition/custody rows for the same truthful per-path applicability.",
    "line": 49
  },
  {
    "file": "contracts/auth/capability/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## AP-A-R2-02 — P2 — The conformance trace still refreshes every read after 401\n\n**Sources:** `contracts/auth/capability/v1alpha1/semantics.md:95`, versus `:68` and `:73`.\n\nThe conformance scenario says every read 401 causes one refresh and re-dispatch, and every mutation 401 causes one refresh. The revised normative rule permits that only for a selected refresh-capable credential profile, explicitly excludes anonymous and static_config implicit acquisition/refresh, and forbids child authority to silently refresh/resend mediated traffic. A conforming anonymous/static-config implementation would fail the currently universal scenario.\n\n**Required correction:** bind the existing refresh trace to an explicitly selected refresh-capable credential profile and add refusal/no-refresh traces for anonymous, static_config and mediated child traffic. This does not settle the separate general read-retry story.",
    "line": 95
  },
  {
    "file": "contracts/auth/connection/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## AP-A-R2-03 — P2 — Binding revisions can be read as permission to change a connection's fixed route or ownership\n\n**Sources:** `contracts/auth/connection/v1alpha1/semantics.md:80,84-85`, `contracts/auth/profile/v1alpha1/semantics.md:108-110`, `contracts/discovery/mediated_route/v1alpha1/semantics.md:60`.\n\nConnection rules keep the route fixed at creation and mediation fixes its parent/resource at materialization. The newly added multiplicity rule calls destination/parent/tenant/owner changes explicit binding revisions, while profile §4.2 describes an access-mode change as a binding/configuration change that invalidates old evidence. Neither selects whether an existing ref can thereby move from anonymous to mediated, switch its parent/target, or change owner. Mere evidence invalidation does not settle stable connection identity and can be interpreted as an implicit reassignment path.\n\n**Required correction:** distinguish revalidation/revision of the same admitted binding from changing its semantic identity. Preserve fixed route/parent/target and ownership by requiring a newly admitted connection for such changes in this profile, or point to a separately defined admitted reassignment binding and refuse it while absent. A configuration edit must not silently repurpose a live ref. No persistence implementation or broad reassignment protocol is requested here.",
    "line": 80
  }
]
```

