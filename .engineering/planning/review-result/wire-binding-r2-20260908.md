---
format: aep.planning-md/1
id: review-result:wire-binding-r2-20260908
kind: review-result
status: active
title: Independent service binding recheck
relations:
- reviews: story:contracts-wire-compatibility
revision: 1
---
# Independent E02 recheck B, round 1

Verdict: **ship with the following semantic fixes**. No runtime work requested. Sources are captured in `source-hashes-r2.json`; root's unfinished verification record is not claimed as reviewed. No source or planning files modified, no other reviewer output read.

The central rewrite resolves the substantial original problems. WB-01 has explicit version paths; WB-02 accurately records old strict-reader diagnostics; WB-03 now has a typed original-effect/replay distinction and safe applied-but-undeliverable outcomes; WB-04 has a separate revision and invoke allowlist; WB-05 explicitly withdraws the incomplete HMAC protocol and gates F03 as unbound/unadvertisable. WB-06 now acknowledges pre-dispatch audit admission, separate outcome records, honest unavailable/incomplete states and preservation of effect knowledge. WB-07 separates authentication from post-policy executor admission; WB-08 selects one codec per interaction and separates all version axes. These are meaningful fixes, not implementation support claims. The remaining points are concrete contradictions within the new proposed binding and linked documents.

## WRB-01 — P1: successful describe response framing is still ambiguous

`contracts/service/compatibility.md` §2 chooses GET /v1alpha2/describe and tells the client to check the returned descriptor, while §4 defines a closed root Descriptor without status/result/audit fields. §5 then says **every extended application response** has nullable request_id, status, exactly one result/error, audit_ref and audit_status. `service/v1alpha2` §3.2 supplies the descriptor contract, without saying whether GET returns it bare or nested in `result`. Both interpretations are plausible, and their bytes are incompatible.

Specify successful GET framing explicitly, including whether descriptor version is nested or top-level, and the exact describe-error form. If bare Descriptor is selected, scope §5's full envelope to invoke and explicitly defined describe failures; if enveloped, give the complete describe success example and its request_id rule. Describe has no request-body correlation, so it must not inherit invoke's “echo valid request ID” rule. State whether successful describe needs audit or only its refusals do. The application status/HTTP status consistency rule should also be explicit for the new binding; otherwise two implementations can choose incompatible error treatment even while sharing the closed ErrorCode set.

Verification: serialize one successful and one refused GET with the chosen shape, then identify which reader schema consumes each. A valid bounded invoke, malformed invoke and GET must have distinct and deterministic correlation rules.

## WRB-02 — P2: generic mutation/page limits have two incompatible selectors

Compatibility §4 and catalog §3.2 correctly make the operation profile singular: generic read selects `generic-http`, generic mutation selects `mutation` plus `realization: generic`, and generic page selects `generic-http-page`. Compatibility §7 grants larger 256 KiB/40 s/30 s limits specifically to the **generic-http profile**, with mutation defaults 64 KiB/20 s/15 s. Catalog §5 grants those larger limits to **all generic execution**, while operations §5 still states 64 KiB/20 s without a selected-realization exception. A generic mutation/page therefore cannot tell which binding it must conform to from the exact singular profile identity.

Select one explicit rule across these documents. If every generic realization gets larger ceilings, name all three allowed combinations (generic-http, generic-http-page, mutation+generic) and amend the mutation limit row to reference the selected realization's explicit compatibility ceilings. If generic mutations intentionally retain the ordinary limits, catalog must say so and stop promising a 30 s provider budget for them. Preserve the rule that smaller hosts refuse rather than silently clip.

Verification: one read, one mutation+generic and one page descriptor, each with the single selected profile and exact positive integer limits; all owner tables agree.

## WRB-03 — P2: auth alternatives example violates the newly required scopes member

Compatibility §4 defines each requires_auth alternative as `{profile, scopes}` and ESS AuthAlternative makes scopes a required List<String>. `contracts/auth/profile/v1alpha1/semantics.md` §3 still shows its second alternative as `{ "profile": "jira.api_token" }` without scopes. Its §7 points to the new strict owner but the normative example would be refused by that owner.

Either add `"scopes": []` to the example (preferred, preserving exact required shape), or explicitly select omission/default semantics consistently in the owner and typed model. Do not silently invent a second auth alternatives shape. Verification should include both alternatives under the selected closed shape.

## WRB-04 — P2: linked migration/proposal prose retains withdrawn audit and executor claims

`docs/cli-migration-v1-to-v2.md` §4 still promises audit_ref “on every governed response, including refusals,” with it **absent** under static bearer. The new codec requires the field with **null** when unavailable/not_required and audit_status. `docs/stack-integration-proposal.md` §2 still says executor comes from verified authentication **before payload decoding**, contradicting the new explicit post-decode policy verification path for a supplied assertion. The stack proposal's §3 also recommends a per-route signature as though the old delegation design remains selected, rather than noting F03 now owns an unbound protocol.

Update these active guidance paragraphs to the new owner and explicitly mark older story/scenario references as historical proposal references where the rewritten governed document changed their numbering/meaning. The proposal need not claim to have created its future stories. Correct the CLI row to nullable audit_ref plus audit_status, with no invented persisted record; correct the executor principle to distinguish authenticated coordinates and separately verified assertion; carry the F03 unbound gate through the delegation advice.

## Items deliberately not reopened

- F03 mechanics, real audit persistence and authority proof remain unbound with explicit advertisement gates; runtime absence is not an E02 defect.
- ESS models public values and names its null/omission, bounds, cross-field, privacy and codec obligations UNMAPPED. It does not claim to run a decoder or durable transaction.
- A quarantined Indeterminate result can be returned with `replayed: true`: this describes returning the durable unknown observation, not replay permission or effect certainty. The lifecycle remains quarantined with no new dispatch.
- The current service implementation and strict v1alpha1 readers remain unchanged. No consumer installation or rollout is inferred from the chosen local v1alpha2 specification identifier.
