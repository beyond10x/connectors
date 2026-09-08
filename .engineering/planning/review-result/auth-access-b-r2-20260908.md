---
format: aep.planning-md/1
id: review-result:auth-access-b-r2-20260908
kind: review-result
status: active
title: Auth access and permission budgets reviewer B recheck
relations:
- reviews: story:contracts-anonymous-auth
- reviews: story:contracts-permission-budgets
- reviews: story:contracts-acquisition-profiles
revision: 1
---
# Independent reviewer B — auth access / acquisition / permission budgets, recheck 1

Verdict: **needs revision**. Two residual P2 wording defects remain in the revised proposal. No runtime implementation finding is asserted. Baseline initial report AP-B-01–08 remains immutable in ../initial/report.md.

## AP-B-R01 — normative 401 retry rule still grants an unselected behavior (P2)

`contracts/auth/capability/v1alpha1/semantics.md:73` says a selected refresh-capable profile's 401 "triggers at most one coordinated refresh ... and one re-dispatch for reads". Its corrected scenario at line 95 instead requires a separately permitted re-dispatch and leaves exact read-retry support to its owner story. `contracts/auth/acquisition/v1alpha1/semantics.md:120` explicitly leaves the read-retry contract unsettled. The normative rule can therefore be read as selecting automatic business-read retry merely from refresh capability, before that owner's prerequisites exist. A fresh credential generation also invalidates existing permission evidence under evidence §4.4.

Correction: qualify the normative bullet itself: read re-dispatch is at most once and only under an explicitly selected read-retry binding, with fresh current-generation admission and the original remaining shared budget/deadline. Without that separately supported binding, no automatic re-dispatch. Do not implement or invent that owner's full retry protocol here.

## AP-B-R02 — authorization-code client-auth declaration is required by evidence but absent from its matrix (P2)

`contracts/auth/acquisition/v1alpha1/semantics.md:76` requires sourced endpoints, an explicit PKCE choice, registration kind, callback binding and token interpretation for authorization code, but does not require the explicit client-authentication choice. The adjacent client-credentials row does. `docs/evidence/auth-profile-budget-20260908/traces.md:46` Q07 claims a missing sourced PKCE/client-auth choice must refuse. Registration kind alone does not specify the authentication method used at the token endpoint, and PKCE is a different proof.

Correction: require the reviewed, sourced client-auth method/policy explicitly in the authorization-code row. Keep its actual vendor behavior and future reader binding as advertisement prerequisites. A generic ESS shape need not execute this predicate.

## Initial-finding disposition

The revised cluster addresses AP-B-01 by distinct anonymous and via_parent profiles, explicit selection and supporting-reader refusal; AP-B-02 by explicit material applicability, required-null no-account identity and parent-owned generation/evidence; AP-B-03 by exact target sets, 64/64/4 ceilings, whole-budget reservation and failure-before-resource-read; AP-B-04 by exact binding/generation/context cache equality, original freshness and independent current policy; AP-B-05 by explicit allowed/denied coverage, no success for unknown/all-denied and unchanged strict Page; AP-B-06 by purpose-specific bounded SSAR POST authority; AP-B-07 by deployment activation without acquisition and configured versus managed Grafana paths; AP-B-08 by specified/reserved flow matrix and no flag-only support, subject to AP-B-R02.

Root's announced narrow consistency corrections for management/static_config, discovery/source authentication, fixed route versus reconfiguration and Grafana rows were also inspected in the live draft where present. The original 21-file draft snapshot and its source-hashes.json preserve that earlier review context; finding-snapshot and finding-source-hashes.json freeze exact bytes for the residual claims above. These are deliberately separate stages because root is the sole source editor and corrected editorial inconsistencies during the read-only pass.

## Limits

Review covers proposed semantics and ESS value-model scope, not provider behavior, cache execution, coordinator persistence, runtime readiness/admission, private POST capability implementation or publication. Evidence is still being assembled; final approval is withheld pending the final corrected packet and independent evidence check. No source/planning mutation or runtime test suite was performed.
