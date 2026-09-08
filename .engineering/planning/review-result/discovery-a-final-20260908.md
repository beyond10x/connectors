---
format: aep.planning-md/1
id: review-result:discovery-a-final-20260908
kind: review-result
status: active
title: Discovery coverage and composition reviewer A final approval
relations:
- reviews: story:contracts-discovery-coverage
- reviews: story:contracts-host-composition
revision: 1
---
# Discovery coverage and host composition — reviewer A final

**Verdict: APPROVE. Residual findings: 0 (P0 0 / P1 0 / P2 0 / P3 0).** F13 and E03 are coherent at this specification checkpoint. Approval is of the proposed semantics and accurately limited evidence, not of a running discovery, routing or composition implementation.

## Reviewed snapshot and independence

The 59 explicitly selected normative/evidence inputs were frozen before analysis in `sources/`; exact SHA256 values are recorded in `source-hashes.json`. Every frozen hash was checked, and all still matched its corresponding working-tree input when this report was written. The allowlist excludes other reviewer reports, aggregate dispositions, mutable AEP workflow and checkpoint/closure bookkeeping. No other reviewer output was inspected. No tracked/planning file was edited and no runtime test was run by this reviewer.

`independent-checks.json` records the separate read-only evidence inspection. `supplemental-source-hashes.json` retains two compiled IR artifacts and the pinned old Kubernetes design used to verify the narrowly changed all-namespaces claim; its external-reference note is explicitly a reviewer paraphrase, not a raw webpage capture.

## Initial and recheck dispositions

DC-A-01–08 have sufficient final dispositions: exact qualified coverage and whole-selection comparison; separate authorization, provider collection and public page completeness; one fenced host publisher with predecessor CAS and non-reusable epochs; historical retention distinct from present route authority; fixed incarnation/type/semantic-target equality; explicit same-target revalidation; a concrete composition executable above generic host/SDK and independent adapter libraries; one-process private ports with explicit placement refusals; and truthful ESS value modeling with persistent ownership left UNMAPPED.

All three first-recheck defects are corrected:

1. **DC-A-R2-01:** auth/evidence, service compatibility, Kubernetes and Grafana summaries agree with per-partition absence authority. A fully exhausted comparable partition may prove its own absent rows even when another partition makes the overall view partial. Incomplete partitions provide no negative evidence, while trustworthy current positives may be retained as observed. The type-change/withdrawal trace now states the complete comparable scan prerequisite.
2. **DC-A-R2-02:** merge precedence preserves a retained withdrawn incarnation as withdrawn through denied, capped, unavailable and unvisited scans. Partial refresh cannot demote it to stale or revive it. Reappearance gets a new id; bounded eviction never turns absence of retained history into deletion or reusable identity authority.
3. **DC-A-R2-03:** collection distinguishes the original 15-second provider-work deadline from the original 20-second outer execution/publication/response deadline, subject to lower admitted limits. Provider expiry stops sends and discards late responses; trustworthy positives may publish only while the outer boundary remains live. Outer expiry prevents new publication/success. PublicationFacts.deadline_current expressly means the outer deadline; it never renews provider time.

The additional immutable-history correction is coherent: the view and cursors retire when their earliest retained historical row expires. A fresh collection is required; pages are not silently filtered and history is not extended. Positive evidence still has its independent, shorter validity and supplies no lasting route permission.

## Targeted Kubernetes namespace check

The final all-namespaces Service mode is explicit configuration, mutually exclusive with finite per-namespace selection. Its exact SSAR tuple remains bounded by the same query and scan budgets; missing/invalid configuration, denied namespaces and caller input cannot activate it or substitute evidence across modes. The distinction between empty namespace for namespaced SSAR and for a truly cluster-scoped resource agrees with the official [Kubernetes ResourceAttributes reference](https://kubernetes.io/docs/reference/kubernetes-api/definitions/resource-attributes-v1-authorization/), independently inspected for this field only. The pinned old design at 81459ac4, lines 73–79, also explicitly selected configured empty namespaces only when the cluster-wide review permits them.

This verifies the field/configuration distinction. It does not verify a live cluster, provider API compatibility, exhaustion/coherent-list predicate or physical cluster identity; E32 remains separately owned.

## Independent evidence assessment

- Reproduced all **63 schema expectations**, including **seven rejected shapes**, against the retained generated JSON Schemas; results exactly matched type-results.json.
- Confirmed the four deliberately accepted semantic counterexamples: complete coverage with denial, namespace-kind without its namespace coordinate, negative publication generation, and the publication decision shape considered alongside an invalid predecessor fact. Generic shapes do not evaluate the cross-value publication predicate; the evidence says so explicitly.
- Matched the **13 selected schema hashes**, and independently compared both retained full projection directories with the manifest: **204 identical artifact hashes** in each.
- Read and audited all **46 textual traces**. They cover partial/comparable absence, exact namespace selection, caps and both deadlines, publication races and ambiguity, immutable paging/retention, terminal withdrawal/reappearance, target equality/revalidation, current authorization, independent composition wiring, and unsupported placement. They are declared expected consequences, not executed semantic traces.
- Inspected the recorded full gate: **50 existing Rust test passes**, Rust **1.88** MSRV success, **13 ESS files / 197 declarations**, **222 compiled scenarios including 34 authored expectations**, zero authoring refusals, and final all-checks-passed marker.
- Inspected separate session artifacts: **13 authored / 201 synthesized expectations**. No sequential runtime execution is claimed.
- Verified that compiled.json and compiled-final.json are byte-identical (SHA256 c2be566be32bb28db36af3b27cad9963d0013cfdfab3e8ad29ab6aca9bdecbd5), supporting the statement that the final explanatory ESS deadline comment changed no compiled value definitions.

The verification record and inspection transcript accurately distinguish existing implementation checks, generic type validation, independent semantic review and future runtime obligations. The discovery domain contains 13 value declarations, not a persistent observation/route/composition entity or an executable reducer.

## Limits of approval

Exact source/target equality, provider exhaustion and coherent pagination, current clocks/authority, CAS and acknowledged view/private-index publication, restart/epoch recovery, finite retention, immutable cursor admission, route revalidation, live port installation and process identity remain concrete implementation/advertisement obligations. Schema Optional omission and Timestamp projections are not the complete required-null/millisecond public codec. Persistent ownership/cardinality remains separately modeled work.

Discovery-profiles E07/E14/E32, broader persistence consolidation and remaining specification work are not closed by this review. No runtime code, current adapter-kind schema, external deployment or provider implementation is approved or claimed by these two completed semantic revisions.
