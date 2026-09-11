---
format: aep.planning-md/1
id: verification-report:findings-auth-access-b-r1-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:auth-access-b-r1-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 9a6f7802079b490caa0d81a23382204cd0b238a37c78c7fddea3398570a7f2b1
relations:
- verifies: review-result:auth-access-b-r1-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:auth-access-b-r1-20260908

This supplements [the immutable original](../review-result/auth-access-b-r1-20260908.md).
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
    "file": "contracts/auth/profile/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## AP-B-01 — P2: monitoring's none profiles cannot be expressed and conflate two different bindings\n\n**Evidence:** auth.profile §3 (`contracts/auth/profile/v1alpha1/semantics.md:63`–`:66`) has no anonymous purpose/scheme; subject:none is restricted to inbound/transport use (`:82`). Auth capability's closed list and HTTP variants have no credential-free direct HTTP capability (`contracts/auth/capability/v1alpha1/semantics.md:12`, `:38`). Grafana rows nonetheless use `<x>.none` for both direct and mediated children (`docs/adapters/grafana.md:49`, `:74`–`:75`), and its direct example has credential:null (`:94`). Mediated_route also shows prometheus.none (`contracts/discovery/mediated_route/v1alpha1/semantics.md:52`).\n\n**Correction:** choose explicit, distinct representations for deliberate anonymous **provider** access and parent-authenticated mediation. Every monitoring row must map to a permitted purpose/subject/scheme/acquisition/capability tuple and exact direct/via placement. Anonymous provider access does not disable service authentication or host admission. A missing bearer/basic secret, omitted requires_auth or unsupported profile must never select it. A mediated child has no local provider credential and no direct origin; the parent owns hop authentication and any configured downstream auth/tenant binding. This conveys no parent SaaS grant or general proxy authority. Keep safe discovery's inherited_from_source wording (`discovery/resources:47`, `:64`) explicitly limited to credential placement, never authority inheritance. New names/configuration shapes remain unavailable to unsupported strict readers.",
    "line": 63
  },
  {
    "file": "ess/domains/credential_evidence.yaml",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## AP-B-02 — P2: universal generation/identity requirements make credential-free bindings impossible\n\n**Evidence:** capability `:37`, `:58`, `:67`–`:68` require every capability to carry and place a validated credential generation. Evidence `:93`–`:96` likewise starts dispatch with a material capture. Existing ESS EvidenceSnapshot/DispatchAdmission require a CredentialGeneration (`ess/domains/credential_evidence.yaml:31`–`:37`, `:43`–`:59`). Connection permits null external_identity only before pending activation (`contracts/auth/connection/v1alpha1/semantics.md:64`). Anonymous direct children, parent-only children, and Docker's socket-peer binding have no child bearer/basic/certificate material from which to construct those facts.\n\n**Correction:** make requirement applicability explicit by selected binding. Retain all F05 safeguards for credential-bearing use; do not weaken them by making missing material silently optional. Credential-free child admission still binds current connection/configuration, destination/route, enablement, host authority and applicable verification, but must not fabricate a CredentialGeneration, CredentialSet, provider-account identity or successful credential check. The parent separately pins/verifies its own material and route authority. Define legitimate ready/null-identity behavior where no provider principal is established, distinct from unvalidated credential-bearing pending state. Socket peer remains a reviewed local transport binding with its own path/peer checks, not anonymous network fallback. Child 401/absence cannot trigger an invented child refresh or auth upgrade; any supported parent refresh remains parent-owned and subject to its separately selected retry rules.",
    "line": 31
  },
  {
    "file": "contracts/auth/evidence/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## AP-B-03 — P2: one permission call cannot authorize bounded namespace/resource fan-out\n\n**Evidence:** evidence §4 and §5 allow one permission query per invocation (`contracts/auth/evidence/v1alpha1/semantics.md:71`, `:125`), but its `[a,b]` scenario requires a distinct answer for each namespace (`:133`). Kubernetes additionally selects multiple resource kinds (`docs/adapters/kubernetes.md:81`) and requires per-verb/group/version/resource/namespace checks before lists/proxy forwarding (`:43`, `:69`). Two namespaces multiplied by resource kinds is already more than two authorization targets.\n\n**Correction:** define an exact authorization-target unit, including instance/connection/provider authority, credential generation and the complete effective authorization request (verb, API group/version, resource, namespace presence/value, name and subresource, plus any admitted impersonation context). Decide explicit first-profile maxima for distinct targets, issued permission queries and bounded provider work/concurrency. A cache hit may save a query but cannot bypass the target/fan-out limit. Count failed, cancelled and repeated queries honestly; do not refund an already issued request. Enumerate only admitted targets, deduplicate exact equal requests, and never replace namespaced checks with a broader wildcard/all-namespaces permission. Authorization, route validation and inventory/proxy calls consume the same existing end-to-end/provider budget; no per-namespace or gateway deadline reset. Document deterministic exhaustion behavior before any unchecked target is used.",
    "line": 71
  },
  {
    "file": "crates/integration-kubernetes/src/local_services.rs",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## AP-B-04 — P2: permission cache reuse needs a precise scope and truthful result classification\n\n**Evidence:** the budget story explicitly requires cached reuse, while evidence currently says one provider query per invocation and “before each list” (`:71`, `:78`). It also requires exact target/generation binding (`:84`, `:95`), forbids transfer across generations (`:115`), and imposes 300 s read/60 s mutation freshness (`:124`). Historical Kubernetes has distinct list-services, get-service and get-services/proxy review requests (`connectors-old/crates/integration-kubernetes/src/local_services.rs:89`–`:169`); collapsing them to a namespace key would be unsound. Its old boolean fallback at `:112`/`:141` is not a new proof that malformed/missing review status is denial.\n\n**Correction:** specify whether reuse is within one invocation, across provider pages and/or across invocations, and how it satisfies “before each” without an obligatory fresh network request. Equality must use all target and generation coordinates and any provider-visible context affecting the decision. Reuse retains original collection/expiry and respects the current operation's stricter bound; it cannot transfer after refresh/replacement, turn a list allowance into get/proxy/write authority, or grant a wider target. Host caller/scope/result authorization is checked independently and currently even when provider evidence is reusable. Distinguish positive allow, definite deny, unavailable/timeout, malformed/unknown and unchecked budget exhaustion; only positive fresh applicable evidence permits a provider action. A missing provider status is not a negative permission fact, and no denied/unknown cache entry becomes success merely to fit a budget.",
    "line": 89
  },
  {
    "file": "crates/connectors-contracts/src/lib.rs",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## AP-B-05 — P2: the required partial outcome has no selected reporting shape\n\n**Evidence:** evidence `:133`, Kubernetes `:43` and resource-discovery `:80`, `:95` promise namespace b is reported denied while a's results return. The current closed Page has only items/next_cursor/complete/provenance (`crates/connectors-contracts/src/lib.rs:14`–`:21`), and the proposed resource-discovery example adds generation but no denied/unchecked coverage (`contracts/discovery/resources/v1alpha1/semantics.md:40`–`:57`). complete:false alone cannot distinguish denied b from unexamined b or a source failure.\n\n**Correction:** select the minimal bounded per-profile outcome needed to report known denied, unavailable and unchecked targets without hiding them as empty collections. Say what complete and any continuation mean over the requested/admitted target set, and how budget exhaustion differs from permission denial. A continuation cannot carry an enduring permission grant or silently skip unchecked targets. If a selected reader cannot represent the truthful partial result, refuse that profile/outcome rather than adding unrecognized fields to the current Page, forging provider items or burying metadata in error text. This need not solve every discovery-coverage story or create a new global envelope; it does need an explicit selected payload disposition and examples for two namespaces, zero admitted targets, target overflow and query/deadline exhaustion.",
    "line": 14
  },
  {
    "file": "contracts/auth/capability/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## AP-B-06 — P2: the HTTP method gate conflicts with private authorization queries\n\n**Evidence:** capability §4 enumerates HTTP methods and refuses a “mutation method” unless the operation profile is mutation (`contracts/auth/capability/v1alpha1/semantics.md:70`). Kubernetes read/proxy evidence requires SSR/SSAR (`docs/adapters/kubernetes.md:43`, `:69`). The preserved implementation sends these through Kubernetes review creation with PostParams (`connectors-old/crates/integration-kubernetes/src/local_services.rs:28`–`:32`, `:108`–`:110`, `:137`–`:139`), while the authorized business request is a read. A blanket POST-is-business-mutation reading makes the proposed permission check unavailable.\n\n**Correction:** distinguish the private, explicitly admitted auth-query/validation request from the target business operation's method/effect classification. Permit only the reviewed identity/permission mechanism with its exact target, bytes/capability scope and budget; the exception must not grant adapter code arbitrary POST or bypass business mutation approval. Identity validation remains a separately admitted activation/revalidation action, never hidden inside an ordinary list. The HTTP verb alone is insufficient to label the provider auth query as a business write. This is a semantic routing/admission clarification, not a request to implement another transport now.",
    "line": 70
  },
  {
    "file": "contracts/auth/profile/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## AP-B-07 — P2: static_config has names and examples but no lifecycle/owner semantics\n\n**Evidence:** profile lists static_config (`contracts/auth/profile/v1alpha1/semantics.md:66`); acquisition's supported/reserved list omits it (`contracts/auth/acquisition/v1alpha1/semantics.md:12`). Kubernetes says deployment-supplied read-only custody/no acquisition (`docs/adapters/kubernetes.md:45`, `:63`–`:64`); Docker says no acquisition flow but labels socket/cert config static_config (`docs/adapters/docker.md:32`, `:55`–`:56`). Grafana simultaneously says configured first/read_only-or-versioned and static_entry (`docs/adapters/grafana.md:36`, `:39`–`:40`, `:73`). Static entry, unlike read-only deployment binding, writes through custody and creates/publishes managed state.\n\n**Correction:** define static_config as an explicitly selected deployment-supplied source/activation binding, separate from interactive acquisition. It reads admitted configuration/opaque refs or a declared nonsecret transport binding, creates no browser/protected-entry action or fictitious Acquisition/completed event, and performs no OAuth exchange merely because it occupies an acquisition.flow field. Schema/config validity alone is not readiness: material capture, admitted identity/validity/required verification, coherent publication, replacement detection and stable connection identity keep their existing rules. Restart/file replacement cannot resurrect a terminal revoked binding or silently reassign identity. Distinguish no-refresh static material from declared refreshable sets. Reconcile Grafana's configured/static_config first phase with any later managed/static_entry phase and its required writable custody. Update index and adapter rows without claiming unimplemented typed-flow support from today's legacy configuration reader.",
    "line": 66
  },
  {
    "file": "contracts/README.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## AP-B-08 — P2: OAuth endpoint and reserved-flow rules are not conditional on the selected flow\n\n**Evidence:** profile `:78` requires authorize_url and token_url for every OAuth flow, while acquisition `:124` correctly defines client credentials as a noninteractive token exchange. Acquisition marks exec_plugin/workload_identity/host_issued reserved (`:12`), but capability `:73`, Kubernetes `:65`, and profile `:83` can read as though a configuration enablement flag alone activates exec/password behavior. The index combines static configuration, plugin use and host-issued session authority (`contracts/README.md:57`–`:62`) without the ownership distinction.\n\n**Correction:** provide one closed supported/proposed/reserved matrix. Authorization code requires reviewed authorize/token endpoints, registration/callback and an explicit sourced PKCE/client-auth policy. Client credentials requires its reviewed token endpoint and configured client authentication, no authorization endpoint, browser or UI capability. Static entry/config require no OAuth endpoint; any token exchange/refresh must be separately declared and sourced. Keep current implementation, fully selected proposed bindings and merely reserved names distinct. An enablement flag cannot supply a missing implementation/profile/validation mechanism. If exec remains reserved as an acquisition flow while its private materializer is described, state that separation; it does not imply public auth.begin support. Host-issued session authority stays with its session owner, not a generic provider-connection acquisition. Password/workload flows remain unavailable unless their complete selected contract exists. Missing provider facts must refuse authoring/advertisement, not infer another provider's URLs, PKCE, response or refresh behavior.",
    "line": 57
  }
]
```

