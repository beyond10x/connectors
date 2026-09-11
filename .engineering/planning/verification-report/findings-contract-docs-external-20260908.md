---
format: aep.planning-md/1
id: verification-report:findings-contract-docs-external-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:contract-docs-external-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 266a32f997fd0e2994bb8efe59ce2a4749acb74a9551a100eeb0936242bac40c
relations:
- verifies: review-result:contract-docs-external-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:contract-docs-external-20260908

This supplements [the immutable original](../review-result/contract-docs-external-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

The original report's verdict and scope remain authoritative; no new verdict is assigned.

## Transcription method

33 findings remain in the scope of this report's final stated conclusion.
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
    "file": "contracts/operations/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "blocker",
    "message": "| 1 | blocking | contracts/operations/v1alpha1/semantics.md:90, :111 | Post-dispatch lost answer is promised as `outcome_unknown, never Timeout`, but the implemented host and transport emit `Timeout` regardless of dispatch state and §8 lists no obligation on them | doc: \"After dispatch, a lost answer is `outcome_unknown`, never `Timeout`\"; crates/connectors-host/src/server.rs:142-148 `timeout(Duration::from_secs(20), invoke_at…).map_err(\\|_\\| Error::new(ErrorCode::Timeout, \"operation deadline exceeded\"))`; crates/connectors-host/src/http.rs:145-146 `if e.is_timeout() { Error::new(ErrorCode::Timeout, …)` | Add §8 rows for server.rs (service deadline after step 9 → `outcome_unknown`) and http.rs (transport timeout after send → `outcome_unknown`), and drop the `Capacity`-after-deadline rule at :120 |",
    "line": 90
  },
  {
    "file": "contracts/auth/capability/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "blocker",
    "message": "| 2 | blocking | contracts/auth/capability/v1alpha1/semantics.md:99; contracts/auth/connection/v1alpha1/semantics.md:111; contracts/sessions/v1alpha1/semantics.md:71; contracts/discovery/mediated_route/v1alpha1/semantics.md:55; contracts/auth/profile/v1alpha1/semantics.md:110 | Five documents add error codes or descriptor fields while two claim no wire change; the implemented wire refuses both | capability:99 \"No wire change\"; connection:111 \"changes no wire bytes\"; new codes `connection_not_ready`, `route_unavailable`, `session_lost`…, `route_refused`; profile:110 \"Descriptor exposes profiles\"; crates/connectors-core/src/lib.rs:13-27 closed `pub enum ErrorCode`, :31 and :71 `#[serde(deny_unknown_fields)]` on `Error` and `Descriptor` | State in each doc that new codes/fields ride on the `v1alpha2` wire bump from operations §7, and decide whether `operations/v1alpha1` contract id bumps with it |",
    "line": 99
  },
  {
    "file": "contracts/discovery/mediated_route/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "blocker",
    "message": "| 3 | blocking | contracts/discovery/mediated_route/v1alpha1/semantics.md:67, :99; docs/adapters/grafana.md:11 | Parent and child adapters are co-located in one host process with composition loading in the generic host crate, contradicting the no-host-to-adapter-dependency invariant | mediated:99 \"Composition rule: child adapter placed in the same host as the parent \\| `crates/connectors-host/src/server.rs` composition loading\"; grafana.md:11 \"runs in the same host process as the Grafana adapter\"; docs/design.md:253 \"There is no host-to-concrete-adapter dependency\"; :158 \"A host does not import every concrete adapter\" | Place the composition in a `compositions/` executable that links both adapter libraries, or define `forward` as a wire binding; keep `connectors-host` adapter-free |",
    "line": 67
  },
  {
    "file": "contracts/auth/connection/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| 4 | should-fix | contracts/auth/connection/v1alpha1/semantics.md:64, :135; contracts/auth/acquisition/v1alpha1/semantics.md:30 | Host-persisted connection/acquisition management is exposed as ordinary adapter operations, against decision 1.1 #4 and design 785 | connection:64 \"all are ordinary `operations/v1alpha1` operations on the adapter\"; connection:118 the store is a \"new host module\"; docs/design.md:35 \"the host supplies persistence, credential custody…\"; :785 \"through an admitted management contract. Do not expose … as ordinary operation discovery data\" | Make `connections.*`/`auth.*` a host management contract (own routes), forwarded by federation as such; record the fork in §10 with both options |",
    "line": 64
  },
  {
    "file": "contracts/auth/capability/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| 5 | should-fix | contracts/auth/capability/v1alpha1/semantics.md:70; contracts/auth/acquisition/v1alpha1/semantics.md:77 | Automatic refresh plus re-dispatch on provider 401 for reads contradicts the implemented contract's blanket rule and is not recorded as a disposition change | capability:70 \"a `401` … triggers at most one coordinated refresh … and one re-dispatch for reads\"; contracts/service/v1alpha1/semantics.md:43 \"No automatic retry. Provider 401/403 … remain distinguishable\" | Add an evidence row \"change: reads may re-dispatch once after refresh (design.md:377 safe outcomes)\" and amend service §Wire boundary in the same change |",
    "line": 70
  },
  {
    "file": "docs/adapters/media-session.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| 6 | should-fix | docs/adapters/media-session.md:37, :63; contracts/sessions/v1alpha1/semantics.md:65 | `sip.dial` is a `mutation` whose `effects` lack `external_write` and contain a `semantic_effects` value, so the operations compiler rule would refuse it | media-session:37 \"`effects: [session_establishment, send_external, human_visible]`\"; operations:51-52 `human_visible` is under `semantic_effects`; operations:155 \"compiler refuses `profile: mutation` without `effects` containing `external_write`\" | Extend the discriminator to `external_write` or `session_establishment`, move `human_visible` to `semantic_effects` in media-session.md |",
    "line": 37
  },
  {
    "file": "contracts/discovery/resources/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| 7 | should-fix | contracts/discovery/resources/v1alpha1/semantics.md:31-34; docs/adapters/grafana.md:61; docs/adapters/kubernetes.md:56 | Contract defines one operation `resources.observe` with a per-request `profile` input, while adapters name `datasources.observe`/`services.observe` and the descriptor fixes profile per operation | resources:34 `{ \"profile\": \"grafana-datasources\", … }`; crates/connectors-core/src/lib.rs:61-68 `pub struct Operation { … pub profile: String … }` | Drop the `profile` input; one operation per profile with adapter-chosen ids listed in resources §3 |",
    "line": 31
  },
  {
    "file": "docs/adapters/kubernetes.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| 8 | should-fix | docs/adapters/kubernetes.md:55 | `deployment.rollout_restart` declared `idempotency: natural` although the preserved old behavior writes a fresh timestamp per call, so every dispatch starts a new rollout | ../connectors/crates/integration-kubernetes/src/hosted.rs:843 `\"kubectl.kubernetes.io/restartedAt\": now_unix_ms().to_string()`; operations:54 `natural`: \"the provider operation is idempotent by its own semantics\" | Declare `idempotency: keyed` (caller key → fixed annotation value) or `none` |",
    "line": 55
  },
  {
    "file": "docs/adapters/kubernetes.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| 9 | should-fix | docs/adapters/kubernetes.md:63-65; docs/adapters/docker.md:55-56; docs/adapters/grafana.md:74; contracts/README.md:47-50 | Acquisition flow `static_config` is used by three adapters and the index but defined by no acquisition profile | acquisition:12 \"Profiles \\| `static_entry`, `oauth2_authorization_code`, `oauth2_client_credentials`; reserved: `workload_identity`, `exec_plugin`, `host_issued`\"; README:22 lists the same three | Add a `static_config` profile to acquisition §1/§4 (deployment-supplied refs, no flow) |",
    "line": 63
  },
  {
    "file": "docs/adapters/grafana.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| 10 | should-fix | docs/adapters/grafana.md:49, :74-75; contracts/discovery/mediated_route/v1alpha1/semantics.md:52 | Profiles `<x>.none` / `prometheus.none` have no admissible `scheme` or `purpose` | profile:62 purposes; :64 \"`scheme` \\| `http_bearer`, `http_basic`, `http_signing`, `mtls`, `socket_peer`, `exec_plugin`, `sip_digest`, `session_authority`\" — no `none` | Add `scheme: none`/`purpose: unauthenticated` to profile §3 or state that unauthenticated connections carry no profile |",
    "line": 49
  },
  {
    "file": "contracts/operations/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| 11 | should-fix | contracts/operations/v1alpha1/semantics.md:88, :102 | `not_attempted` is an error code, yet step 7 assigns one failure both `Unavailable` and `not_attempted`; `Error` carries one `code` | :102 \"refuses with `Unavailable` and `not_attempted` semantics\"; crates/connectors-core/src/lib.rs:31-37 `pub struct Error { pub code: ErrorCode, … }` | Make attempted/not-attempted an `Error` field (v1alpha2), not a code |",
    "line": 88
  },
  {
    "file": "contracts/operations/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| 12 | should-fix | contracts/operations/v1alpha1/semantics.md:98; docs/adapters/atlassian.md:111 | \"Forbidden, not NotFound\" for configuration-disabled mutations conflicts with non-advertisement of disabled operations and the implemented lookup | atlassian:111 \"Writes are disabled unless listed\"; service:19 \"Only implemented contracts/operations are advertised\"; crates/connectors-core/src/lib.rs:86 `Error::new(ErrorCode::NotFound, \"operation is not provided\")` | Either advertise disabled mutations with a `disabled` flag and return `Forbidden`, or delete the rule |",
    "line": 98
  },
  {
    "file": "contracts/README.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| 13 | should-fix | contracts/README.md:29 | Deferred-family list omits `configuration` although the design layout has it and the new docs depend on configuration/management revisions | README:29 \"Deferred families with no document yet: `execution`, `events`, `resources`, `catalog`\"; docs/design.md:180 \"│   ├── configuration/\"; design §11 revision/activation rules; connection:41 \"config or management revision\" | Add `configuration` to the deferred list and to design.md:1284 (\"5 families deferred\") |",
    "line": 29
  },
  {
    "file": "docs/adapters/kubernetes.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| 14 | should-fix | docs/adapters/kubernetes.md:78; contracts/discovery/resources/v1alpha1/semantics.md:125 | Recognizer marker `argocd` contradicts the cited amendment, which requires exact `argocd-server` name plus label | ../connectors/docs/design/10-…:175 \"gains `argocd-server`\"; :184 \"Even `contains(\\\"argocd-server\\\")` takes the metrics Service. So an exact-identity arm runs first\" | Use `argocd-server` (name and `app.kubernetes.io/name` label) |",
    "line": 78
  },
  {
    "file": "contracts/media/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 15 | nit | contracts/media/v1alpha1/semantics.md:72 | Citation names the wrong file | \"(`docs/design.md:315` in old design 05…)\"; connectors_v2 docs/design.md:315 is \"## 6. Service bootstrap and discovery\"; intended ../connectors/docs/design/05-…:315-318 | Cite `../connectors/docs/design/05-native-sip-and-rtvbp.md:315-318` |",
    "line": 72
  },
  {
    "file": "contracts/sessions/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 16 | nit | contracts/sessions/v1alpha1/semantics.md:21; docs/adapters/media-session.md:17 | Response fields cited to `b10x.toml:741-760` sit outside that range | ../connectors/providers/b10x.toml:768-771 `[operations.response_schema] … required = [\"call\",\"session\",\"state\"] … enum = [\"established\"]` | Cite `741-771` |",
    "line": 21
  },
  {
    "file": "contracts/operations/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 17 | nit | contracts/operations/v1alpha1/semantics.md:22 | `jira.toml:376` cited for `effects = [\"write\",\"network\"]`; that line is the id | ../connectors/providers/jira.toml:376 `id = \"jira-issue-create\"`; effects at :370 | Cite `:369-376` |",
    "line": 22
  },
  {
    "file": "docs/adapters/grafana.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 18 | nit | docs/adapters/grafana.md:110 | \"all GET\" is false for the Grafana source | ../connectors/specs/grafana/http-api-2026-08-14.openapi.yaml:105-106 `/api/ds/query: post:` | \"all selected operations GET; the source's one POST (`datasource_query`) is unselected\" |",
    "line": 110
  },
  {
    "file": "contracts/auth/acquisition/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 19 | nit | contracts/auth/acquisition/v1alpha1/semantics.md:47 vs :61 | Same failure named two ways | :47 `scope_insufficient`; :61 and connection:58 `insufficient_scope` | Use `insufficient_scope` |",
    "line": 47
  },
  {
    "file": "contracts/media/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 20 | nit | contracts/media/v1alpha1/semantics.md:62 | \"Terminal reasons added to sessions\" lists one already present and one absent from sessions | sessions:50 already has `media_overload`; lacks `media_incompatible` | Add `media_incompatible` to sessions:50; drop `media_overload` from \"added\" |",
    "line": 62
  },
  {
    "file": "contracts/auth/connection/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 21 | nit | contracts/auth/connection/v1alpha1/semantics.md:54-62 vs :127 | Status table lacks `pending`; ESS lifecycle lacks `disabled` | :127 \"`pending → ready ↔ …`\"; table rows `ready…disabled…revoked` | Align both lists |",
    "line": 54
  },
  {
    "file": "contracts/README.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 22 | nit | contracts/README.md:36-39 | Adapter→contract table omits contracts the adapter docs require | kubernetes.md:44-45 and docker.md:29-30 list `auth.connection`, `auth.custody`; grafana.md:40 `auth.custody`; media-session.md:64 records list | Regenerate the table from the adapter docs |",
    "line": 36
  },
  {
    "file": "contracts/README.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 23 | nit | contracts/README.md:52; docs/adapters/media-session.md:75-76 | \"redemption ledger\" is not an `auth.evidence` check | evidence:12 checks are `custody_reachable … verify_operation` | Use `custody_reachable` or name it as `inbound-verifier.redeem` (capability) |",
    "line": 52
  },
  {
    "file": "docs/adapters/media-session.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 24 | nit | docs/adapters/media-session.md:65-67 | `session.close/signal/interrupt` listed as operation ids | sessions:61 `close` and media:58-59 `signal`/`interrupt` are `duplex_transport` control messages | Split the table into operations and control messages |",
    "line": 65
  },
  {
    "file": "docs/adapters/docker.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 25 | nit | docs/adapters/docker.md:22, :25 | Vendor behavior asserted as fact in a doc that marks endpoints unverified | :22 \"Docker list endpoints are unpaged\"; :25 \"start of a running container is a no-op on the daemon side\" | Mark both \"to verify at pinning\" |",
    "line": 22
  },
  {
    "file": "docs/adapters/media-session.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 26 | nit | docs/adapters/media-session.md:86, :89; docs/adapters/atlassian.md:105-106; docs/adapters/kubernetes.md:76-78; docs/adapters/docker.md:67 | Configuration outlines carry unlabeled numbers | `max_sessions: 8`, `max_call_seconds: 3600`, `[40000, 40100]`, `page_limit: 100`, `max_body_bytes: 262144`, `resource_limit: 500`, `max_bytes: 131072`; docs/design.md:1146 | Prefix each outline \"illustrative values\" |",
    "line": 86
  },
  {
    "file": "contracts/auth/profile/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 27 | nit | contracts/auth/profile/v1alpha1/semantics.md:64-65, :103; contracts/auth/capability/v1alpha1/semantics.md:12, :26; contracts/media/v1alpha1/semantics.md:13; contracts/datasources/series/v1alpha1/semantics.md:12; contracts/discovery/resources/v1alpha1/semantics.md:62 | Reserved vocabulary for providers/uses outside the six areas (`http_signing`, `oauth2_password`, `hold`/`transfer`, `promql-instant/labels`, `address` locator) | docs/design.md:49 \"add abstractions when multiple implementations demonstrate the same semantics\" | Remove reserved names; add them with the adapter that needs them |",
    "line": 64
  },
  {
    "file": "docs/design.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 28 | nit | connection:118; operations:153; acquisition:116; resources:110; mediated_route:98; evidence:101-103; custody:94 | Seven \"new host module\" stores declared across seven docs with no owning document | docs/design.md:47 \"one owner for each behavior\"; :961 narrow operations per store | One host-persistence section (in custody or a host doc) listing every port and its atomicity |",
    "line": 47
  },
  {
    "file": "contracts/auth/profile/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 29 | nit | contracts/auth/profile/v1alpha1/semantics.md:77 | Compiler rule demands `authorize_url` for every OAuth flow; client-credentials has none | :77 \"refuses a profile whose `acquisition` names an OAuth flow without `authorize_url`/`token_url`\" | Require `token_url` only for `oauth2_client_credentials` |",
    "line": 77
  },
  {
    "file": "contracts/datasources/logs/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 30 | nit | contracts/datasources/logs/v1alpha1/semantics.md:24 vs docs/adapters/grafana.md:94 | Tenant header configured under two names | logs:24 \"`tenant_header`\"; grafana.md:94 `\"extra_headers\": { \"X-Scope-OrgID\": … }` | Pick one |",
    "line": 24
  },
  {
    "file": "docs/adapters/kubernetes.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 31 | nit | docs/adapters/kubernetes.md:17 | \"`resources.list` … already returns status\" is unverified; the implemented output items are untyped | adapters/kubernetes/spec/adapter.json:111-116 `\"items\": { \"type\": \"array\", \"items\": { \"type\": \"object\" } }` | Say \"returns the object as listed; projection to be declared\" |",
    "line": 17
  },
  {
    "file": "docs/adapters/kubernetes.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 32 | nit | docs/adapters/kubernetes.md (whole) | design.md:939 lists \"cluster identity\" for Kubernetes; not addressed or deferred | docs/design.md:939 \"Watches/gaps, namespaces, cluster identity, admitted process sessions, mediation/tunnels\" | Add to §1 (instance ↔ API-server origin) or §9 |"
  },
  {
    "file": "contracts/operations/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| 33 | nit | contracts/operations/v1alpha1/semantics.md:32, :57 | Struct ranges truncated | `Operation` is core lib.rs:60-68 (cited 59-66); `Invocation` is :90-98 (cited 90-97) | Cite `60-68`, `90-98` |",
    "line": 32
  }
]
```

