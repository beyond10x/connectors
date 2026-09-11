---
format: aep.planning-md/1
id: verification-report:findings-auth-access-a-r1-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:auth-access-a-r1-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 06a79a30e8b9e13611926f8e0f064bcecb5716e1f97c88fbd4c00b97635ceb1c
relations:
- verifies: review-result:auth-access-a-r1-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:auth-access-a-r1-20260908

This supplements [the immutable original](../review-result/auth-access-a-r1-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

The original report's verdict and scope remain authoritative; no new verdict is assigned.

## Transcription method

8 findings remain in the scope of this report's final stated conclusion.
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
    "file": ".engineering/planning/review-result/auth-access-a-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "### AP-A-01 — P2 — Deliberate direct anonymous access has no closed representation\n\n**Sources:** auth.profile:63–66, auth.capability:12/38, Grafana adapter:49–50/74/94, service compatibility:60.\n\nMonitoring promises a `<x>.none` direct profile and shows `credential: null`, but the closed purpose/scheme/capability vocabularies contain no anonymous HTTP mode. The example also lacks an explicit selected auth mode, so null alone could be mistaken for missing bearer/basic material. E02 correctly says absent requires_auth is not anonymous access.\n\n**Required correction:** select a named direct-anonymous profile with explicit purpose/subject/scheme/flow/capability and placement semantics. Its receiver-owned configuration deliberately permits no provider credential and must reject contradictory supplied auth material. A bearer/basic/signing/mTLS connection missing its required material refuses under that profile; it never falls through to anonymous, another credential alternative, a different endpoint or an ambient credential. Anonymous provider access still requires the current Connector caller/connection/resource policy, admitted destination, limits and declared evidence. Align each direct monitoring row and configuration example with this representation and its future strict reader; do not claim current HttpConfig or legacy projection already accepts it."
  },
  {
    "file": ".engineering/planning/review-result/auth-access-a-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "### AP-A-02 — P2 — Parent authentication is conflated with direct anonymity\n\n**Sources:** Grafana adapter:49/75/100–110; mediated_route:49–64; discovery/resources:47/64/75.\n\nBoth direct unauthenticated traffic and traffic authenticated by a Grafana/Kubernetes parent use `<x>.none`. They have different authority and readiness dependencies. The route's “identical profiles/descriptor” claim at59 is also too broad once the child has a distinct parent-authenticated profile.\n\n**Required correction:** select an explicit parent-authenticated child profile, separately from direct anonymous. Bind its allowed use to exactly one admitted same-composition parent/resource/route revision. The child receives no parent credential, secret reference or general parent capability; the parent places its own credential only for the reviewed hop and keeps any provider-managed downstream configuration private. Both parent use and child operation admission remain current and separate; no grant or provider identity is inherited simply because the parent authenticates. A missing/degraded/revoked parent, stale target mapping, changed type or unsupported authentication requirement refuses with no direct fallback. Preserve identical child business request/result semantics, while allowing truthful auth/route/requirement metadata differences. Discovery's source-auth requirement is a candidate fact, not admission or proof that an arbitrary target supports that mode."
  },
  {
    "file": ".engineering/planning/review-result/auth-access-a-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "### AP-A-03 — P2 — Universal credential-generation obligations cannot describe non-material modes\n\n**Sources:** auth.capability:37/58/66–68; auth.profile:82/93; auth.connection:64/94–112; ess/credential_evidence:43–59; ess/credentials:26–47; Docker adapter:55–56.\n\nEvery capability is said to contain a validated credential-generation pin, even mediated HTTP and owner-checked local sockets. `subject: none` is restricted to instance/ingress attachment, while the new anonymous/parent modes need stable Connections without a local provider principal. The existing DispatchAdmission type always references CredentialGeneration. Fabricating an empty credential, generation, external account or successful identity check would undermine the evidence semantics.\n\n**Required correction:** make credential placement and evidence applicability explicit by mode. Material-bearing modes retain all F05 generation/identity/pinning safeguards. Direct anonymous has no credential acquisition/refresh or synthetic provider identity; parent authentication depends on the parent's current admitted material plus a distinct child route/target binding. Socket access retains its real path/peer/transport checks rather than becoming anonymous HTTP or pretending a path is secret material. Define what a ready non-material Connection exposes for external_identity and which local checks are not applicable; non-applicable does not mean successfully verified. Give any new admission/binding value a truthful typed home or explicit UNMAPPED disposition instead of coercing it into a CredentialGeneration. No persistent owner or executable verifier need be invented in this slice."
  },
  {
    "file": ".engineering/planning/review-result/auth-access-a-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "### AP-A-04 — P2 — Permission-check units and budgets do not cover namespace fan-out\n\n**Sources:** auth.evidence:71/78/95/115/124–125; Kubernetes adapter:43/69–71/80–84; mediated_route:64/75–77.\n\nThe first profile permits at most one permission query per business invocation, while two uncached namespaces already need two exact checks. The illustrated four resource kinds across two namespaces may need eight distinct authorization targets; namespace alone is not the unit. Proxy authorization is another exact get/services/proxy target.\n\n**Required correction:** define the check key and finite ceilings for selected distinct authorization targets, new provider query attempts and concurrency. The key includes the exact admitted cluster/provider authority, connection/profile/material generation, verb, API group/version, resource, namespace and applicable name/subresource. Bound cached-target fan-out too; cache hits cannot permit unbounded CPU/selection work. Count attempted queries, including failed/timed-out ones, consistently. All permission, resource and mediated-parent work shares the original remaining operation/provider budget; nested forwards/pages cannot silently mint a fresh allowance. Use exact still-valid cached results only, retaining original expiry and current host admission; generation/publication/binding changes invalidate reuse. A cached answer for another namespace/resource/name/verb, missing evidence or an unknown response cannot authorize a target."
  },
  {
    "file": ".engineering/planning/review-result/auth-access-a-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "### AP-A-05 — P2 — Budget exhaustion needs truthful partial and publication semantics\n\n**Sources:** auth.evidence:78/125/133; Kubernetes adapter:43; discovery/resources:53–68/78–80/95/101; service compatibility:142; datasource.records:87–88.\n\nThe contracts say denied namespaces are skipped and reported, but do not define where fan-out exhaustion or unavailable checks appear. `resources.observe` also treats absence from a newer generation as withdrawal. Publishing a budget-truncated namespace set as complete could therefore withdraw resources that were never checked.\n\n**Required correction:** select deterministic exhaustion behavior before dispatching unchecked targets. State whether an over-limit target set refuses upfront or produces an explicitly supported partial result. In any supported partial result, distinguish allowed, positively denied and unchecked/unavailable targets; timeout/budget exhaustion is not denial or an empty namespace. Define all-denied and zero-checked cases, deterministic selection/order, cursor continuation and whether a provider list may begin before all its required authorization is known. A partial snapshot cannot establish absence/withdrawal for unobserved targets; refuse generation publication if the selected coverage binding cannot represent that fact. Exact coverage/partial fields require their own declared payload reader and bounded result size, not silent additions to the current closed Page. This can preserve the separate discovery-coverage owner's work without inventing a complete replacement coverage model here."
  },
  {
    "file": ".engineering/planning/review-result/auth-access-a-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "### AP-A-06 — P2 — static_config is named but has no acquisition boundary\n\n**Sources:** auth.profile:66; auth.acquisition:12/33/69–75/122–124; Kubernetes adapter:44–45/63–65; Docker adapter:29–32/55–56; Grafana adapter:74; contracts/README provider table.\n\nStatic configuration appears as a flow value and in adapter auth rows, but auth.acquisition defines only static entry and two OAuth flows. The generic begin/complete rules assume coordinator state, registration, callback/entry evidence and newly written custody; Kubernetes/Docker explicitly say their material is deployment-supplied and no acquisition flow is needed.\n\n**Required correction:** define static_config as a deployment-bound source/activation path distinct from interactive static_entry. Configuration selects admitted endpoint/socket/material references and auth mode; it does not allocate browser state, mint a completion URL, require OAuth registration or copy deployment secrets into writable custody. Required coherent capture, separately admitted validation and F05 publication remain before business use. State that auth.begin cannot turn this mode into an implicit interactive acquisition, and define missing reference/disabled helper/invalid binding refusals. Socket metadata and anonymous configuration need no fabricated secret reference. Reconcile adapter and index rows so “no interactive acquisition” is compatible with a supported static configuration classification."
  },
  {
    "file": ".engineering/planning/review-result/auth-access-a-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "### AP-A-07 — P2 — OAuth endpoint and registration requirements are not flow-specific\n\n**Sources:** auth.profile:40–47/78/107; auth.acquisition:69–75/124/183.\n\nThe compiler rule and conformance text require authorize_url and token_url for every OAuth flow, contradicting client credentials, which has no authorization browser endpoint. Generic registration/callback steps also do not distinguish static entry/configuration from OAuth.\n\n**Required correction:** publish a closed flow-by-field applicability matrix. Authorization code requires its reviewed authorization/token endpoints, supported registration/client-auth choice, redirect/callback correlation, expiry/one use and explicitly selected PKCE behavior. Client credentials requires the reviewed token endpoint and applicable registration/client authentication, but no invented authorize endpoint, redirect, consent UI or PKCE phase. Static entry/configuration do not acquire OAuth endpoint requirements. Mark forbidden/irrelevant fields and contradictory combinations explicitly instead of silently ignoring them. PKCE, refresh-token behavior, identity and granted-scope interpretation remain provider-profile evidence, not values inferred from a different provider or applied indiscriminately to every flow. Update both normative validation prose and conformance expectations."
  },
  {
    "file": ".engineering/planning/review-result/auth-access-a-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "### AP-A-08 — P2 — Supported, reserved, enabled and implemented flows need one matrix\n\n**Sources:** auth.acquisition:12; auth.profile:66/78/83/107/141; Kubernetes adapter:65/71; contracts/README provider/index rows; service compatibility:56/137–140.\n\nAcquisition labels exec_plugin and host_issued reserved, while Kubernetes lists a configuration-enabled exec path and sessions owns host-issued authority. The password flow says configuration may enable it without selecting a complete supported binding. A flow name or enablement toggle alone does not supply handlers, declared effects, endpoint rules or a safe public contract.\n\n**Required correction:** distinguish selected proposed semantics, reserved vocabulary, per-instance enablement and implemented/advertisable support. Map every adapter row to one explicit status and applicable owner. If exec_plugin remains reserved in acquisition, do not advertise an acquisition handler because the helper flag is enabled; preserve its existing admitted activation/material-capture restrictions wherever its concrete capability is later implemented. Host-issued session authority retains the sessions owner rather than becoming generic auth.begin merely by enum membership. Legacy password/workload identity and other reserved names remain unavailable until their own complete bindings exist; an enable flag cannot override unsupported status. Synchronize safe descriptor projection and the index without editing current adapter schemas or claiming runtime support."
  }
]
```

