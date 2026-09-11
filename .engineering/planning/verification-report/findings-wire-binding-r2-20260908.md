---
format: aep.planning-md/1
id: verification-report:findings-wire-binding-r2-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:wire-binding-r2-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 41404b8b4bec1582b167c1abd1904a946eaf487a01a2d81add9a83d0153af51d
relations:
- verifies: review-result:wire-binding-r2-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:wire-binding-r2-20260908

This supplements [the immutable original](../review-result/wire-binding-r2-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

The original report's verdict and scope remain authoritative; no new verdict is assigned.

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
    "severity": "blocker",
    "message": "## WRB-01 — P1: successful describe response framing is still ambiguous\n\n`contracts/service/compatibility.md` §2 chooses GET /v1alpha2/describe and tells the client to check the returned descriptor, while §4 defines a closed root Descriptor without status/result/audit fields. §5 then says **every extended application response** has nullable request_id, status, exactly one result/error, audit_ref and audit_status. `service/v1alpha2` §3.2 supplies the descriptor contract, without saying whether GET returns it bare or nested in `result`. Both interpretations are plausible, and their bytes are incompatible.\n\nSpecify successful GET framing explicitly, including whether descriptor version is nested or top-level, and the exact describe-error form. If bare Descriptor is selected, scope §5's full envelope to invoke and explicitly defined describe failures; if enveloped, give the complete describe success example and its request_id rule. Describe has no request-body correlation, so it must not inherit invoke's “echo valid request ID” rule. State whether successful describe needs audit or only its refusals do. The application status/HTTP status consistency rule should also be explicit for the new binding; otherwise two implementations can choose incompatible error treatment even while sharing the closed ErrorCode set.\n\nVerification: serialize one successful and one refused GET with the chosen shape, then identify which reader schema consumes each. A valid bounded invoke, malformed invoke and GET must have distinct and deterministic correlation rules."
  },
  {
    "file": ".engineering/planning/review-result/wire-binding-r2-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## WRB-02 — P2: generic mutation/page limits have two incompatible selectors\n\nCompatibility §4 and catalog §3.2 correctly make the operation profile singular: generic read selects `generic-http`, generic mutation selects `mutation` plus `realization: generic`, and generic page selects `generic-http-page`. Compatibility §7 grants larger 256 KiB/40 s/30 s limits specifically to the **generic-http profile**, with mutation defaults 64 KiB/20 s/15 s. Catalog §5 grants those larger limits to **all generic execution**, while operations §5 still states 64 KiB/20 s without a selected-realization exception. A generic mutation/page therefore cannot tell which binding it must conform to from the exact singular profile identity.\n\nSelect one explicit rule across these documents. If every generic realization gets larger ceilings, name all three allowed combinations (generic-http, generic-http-page, mutation+generic) and amend the mutation limit row to reference the selected realization's explicit compatibility ceilings. If generic mutations intentionally retain the ordinary limits, catalog must say so and stop promising a 30 s provider budget for them. Preserve the rule that smaller hosts refuse rather than silently clip.\n\nVerification: one read, one mutation+generic and one page descriptor, each with the single selected profile and exact positive integer limits; all owner tables agree."
  },
  {
    "file": "contracts/auth/profile/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## WRB-03 — P2: auth alternatives example violates the newly required scopes member\n\nCompatibility §4 defines each requires_auth alternative as `{profile, scopes}` and ESS AuthAlternative makes scopes a required List<String>. `contracts/auth/profile/v1alpha1/semantics.md` §3 still shows its second alternative as `{ \"profile\": \"jira.api_token\" }` without scopes. Its §7 points to the new strict owner but the normative example would be refused by that owner.\n\nEither add `\"scopes\": []` to the example (preferred, preserving exact required shape), or explicitly select omission/default semantics consistently in the owner and typed model. Do not silently invent a second auth alternatives shape. Verification should include both alternatives under the selected closed shape."
  },
  {
    "file": "docs/cli-migration-v1-to-v2.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## WRB-04 — P2: linked migration/proposal prose retains withdrawn audit and executor claims\n\n`docs/cli-migration-v1-to-v2.md` §4 still promises audit_ref “on every governed response, including refusals,” with it **absent** under static bearer. The new codec requires the field with **null** when unavailable/not_required and audit_status. `docs/stack-integration-proposal.md` §2 still says executor comes from verified authentication **before payload decoding**, contradicting the new explicit post-decode policy verification path for a supplied assertion. The stack proposal's §3 also recommends a per-route signature as though the old delegation design remains selected, rather than noting F03 now owns an unbound protocol.\n\nUpdate these active guidance paragraphs to the new owner and explicitly mark older story/scenario references as historical proposal references where the rewritten governed document changed their numbering/meaning. The proposal need not claim to have created its future stories. Correct the CLI row to nullable audit_ref plus audit_status, with no invented persisted record; correct the executor principle to distinguish authenticated coordinates and separately verified assertion; carry the F03 unbound gate through the delegation advice."
  }
]
```

