---
format: aep.planning-md/1
id: review-result:auth-access-a-r2-20260908
kind: review-result
status: active
title: Auth access and permission budgets reviewer A r2
relations:
- reviews: story:contracts-anonymous-auth
- reviews: story:contracts-permission-budgets
- reviews: story:contracts-acquisition-profiles
revision: 1
---
# Auth-profile and permission-budget cluster — reviewer A first recheck

**Verdict: needs revision. Remaining findings: P0 0 / P1 0 / P2 3 / P3 0.** This is an immutable intermediate review of the revised normative packet, before the final evidence packet. It does not supersede the initial review. No other reviewer output was consulted, no tracked/planning file was changed, and no runtime test was run.

The 28 captured working-tree inputs are retained in `sources/` and `source-hashes.json`. They include the new auth_access ESS value model. Planning snapshots record current workflow prose only; later status/journal changes do not alter the reviewed normative decisions.

## Initial finding disposition

AP-A-01/02: substantive anonymous and mediated selections are now separately specified, with explicit profile combinations, independent host/child/parent admission, no credential fallback or inherited child identity/scopes. The remaining active Grafana map conflicts with that correction (R2-01 below).

AP-A-03: credential-generation obligations now have an explicit material-bearing applicability branch; anonymous/mediated children keep required-null identity without fake generations, and socket transport admission is explicitly UNMAPPED. Binding change identity must still be reconciled (R2-03).

AP-A-04/05: exact target coordinates and checking-binding equality, target/call/concurrency ceilings of 64/64/4, full missing-call reservation, current policy/time/generation checks, shared deadlines, no query retry, and all-checks-before-resource-work are specified. Unknown/unavailable fails before business work; denied subsets require declared coverage and cannot establish discovery withdrawal. The new coverage is explicitly not an extension accepted by strict Page. This is a coherent conservative selection pending evidence review, not proof of an implemented budget/check/cache engine.

AP-A-06/07/08: static_config activation is separate from acquisition sessions; OAuth required/forbidden declarations are flow-specific; client credentials has no browser fields or refresh-token behavior; reserved names/configuration flags cannot advertise support. New declarations do not claim current compiler/host support. Some older unqualified examples still need the direct correction below.

## AP-A-R2-01 — P2 — The active monitoring contract map still selects the retired none profile

**Sources:** `docs/adapters/grafana.md:49-50`, versus `:74-79` and `contracts/auth/profile/v1alpha1/semantics.md:101-110`.

The contract map currently selects `<x>.none` for both direct and mediated access and lists only bearer/basic for direct capabilities. Section 5 now says `<x>.none` is not an accepted alias and selects `.anonymous`/`http-anonymous` and `.via_parent`/`mediated-http` separately. The active adapter declaration table and its detailed profile table therefore prescribe different accepted profiles and credential placement.

**Required correction:** update the active contract map to the four selected profiles and their distinct capabilities/placement. Keep historical names only in explicitly historical evidence. Check the adjacent configured-first/acquisition/custody rows for the same truthful per-path applicability.

## AP-A-R2-02 — P2 — The conformance trace still refreshes every read after 401

**Sources:** `contracts/auth/capability/v1alpha1/semantics.md:95`, versus `:68` and `:73`.

The conformance scenario says every read 401 causes one refresh and re-dispatch, and every mutation 401 causes one refresh. The revised normative rule permits that only for a selected refresh-capable credential profile, explicitly excludes anonymous and static_config implicit acquisition/refresh, and forbids child authority to silently refresh/resend mediated traffic. A conforming anonymous/static-config implementation would fail the currently universal scenario.

**Required correction:** bind the existing refresh trace to an explicitly selected refresh-capable credential profile and add refusal/no-refresh traces for anonymous, static_config and mediated child traffic. This does not settle the separate general read-retry story.

## AP-A-R2-03 — P2 — Binding revisions can be read as permission to change a connection's fixed route or ownership

**Sources:** `contracts/auth/connection/v1alpha1/semantics.md:80,84-85`, `contracts/auth/profile/v1alpha1/semantics.md:108-110`, `contracts/discovery/mediated_route/v1alpha1/semantics.md:60`.

Connection rules keep the route fixed at creation and mediation fixes its parent/resource at materialization. The newly added multiplicity rule calls destination/parent/tenant/owner changes explicit binding revisions, while profile §4.2 describes an access-mode change as a binding/configuration change that invalidates old evidence. Neither selects whether an existing ref can thereby move from anonymous to mediated, switch its parent/target, or change owner. Mere evidence invalidation does not settle stable connection identity and can be interpreted as an implicit reassignment path.

**Required correction:** distinguish revalidation/revision of the same admitted binding from changing its semantic identity. Preserve fixed route/parent/target and ownership by requiring a newly admitted connection for such changes in this profile, or point to a separately defined admitted reassignment binding and refuse it while absent. A configuration edit must not silently repurpose a live ref. No persistence implementation or broad reassignment protocol is requested here.

## Evidence limits and next review

The author is still assembling the typed/textual verification packet. This pass reviewed normative declarations and the auth_access model without executing a reducer, provider query, cache, coordinator, transport, or runtime test. Generic schema shapes intentionally leave cross-field access combinations, required/forbidden flow fields, numeric limits, current authority/time, budget consumption and completeness predicates UNMAPPED. A final verdict must separately assess the completed evidence and the three source corrections above.
