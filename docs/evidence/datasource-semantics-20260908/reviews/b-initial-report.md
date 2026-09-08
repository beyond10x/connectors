needs-revision

Baseline: `dd08cfd60aa43740da21946666dc0fa89c1160dc`; independent initial review of F12 and F14 only. Eight findings: four P1 and four P2; no P0. These are unresolved baseline contract defects, not objections to the stories already identifying the work.

story:contracts-log-continuation — LD-B-01 (P1; owner story:contracts-log-continuation): Replace timestamp-only continuation with a selected occurrence-preserving progress rule or explicit non-resumable partial output, including saturated ties and repeated identical entries. — contracts/datasources/logs/v1alpha1/semantics.md:67
story:contracts-log-continuation — LD-B-02 (P2; owner story:contracts-log-continuation): Define exact interval endpoints, timestamp syntax and precision, fixed default end time, and ordering for each selected log profile before deriving any continuation boundary. — contracts/datasources/logs/v1alpha1/semantics.md:74
story:contracts-log-continuation — LD-B-03 (P2; owner story:contracts-log-continuation): Specify completeness and truncation from provider exhaustion and every local bound, with safe handling of wrong result kinds, malformed entries and simultaneous truncation causes. — contracts/datasources/logs/v1alpha1/semantics.md:65
story:contracts-log-continuation — LD-B-04 (P1; owner story:contracts-log-continuation): Select a trustworthy native-query scope admission mechanism and bind its source, authorization and projection inputs to every initial read, continuation and cached result. — contracts/datasources/logs/v1alpha1/semantics.md:73
story:contracts-document-admission — LD-B-05 (P1; owner story:contracts-document-admission): Replace the impossible opaque-ID zero-provider-call rule with a selected bounded membership-admission path that distinguishes locally known refusal from an explicitly admitted metadata lookup. — contracts/datasources/records/v1alpha1/semantics.md:63
story:contracts-document-admission — LD-B-06 (P1; owner story:contracts-document-admission): Define the membership-to-body observation guarantee and cache reauthorization rule so moved documents and stale membership cannot inherit content access from an earlier lookup or body version. — contracts/datasources/records/v1alpha1/semantics.md:68
story:contracts-document-admission — LD-B-07 (P2; owner story:contracts-document-admission): Choose one faithful Jira fields representation and truncation encoding, then align the document schema, conformance scenario, compatibility projection and actual ESS modeling claim. — contracts/datasources/records/v1alpha1/semantics.md:53
story:contracts-document-admission — LD-B-08 (P2; owner story:contracts-document-admission): Define exact body-byte accounting and whole-response refusal rules within bounded provider work, including oversized source bodies and list projections carrying multiple documents. — contracts/datasources/records/v1alpha1/semantics.md:54

Evidence and required corrections:

1. **LD-B-01.** With 1,001 occurrences at timestamp T and a 1,000 limit, an exclusive next edge skips the remainder; an inclusive edge can repeat the same 1,000 indefinitely. A hash of timestamp/labels/text is not occurrence identity: two genuine identical entries in the same stream must remain two entries, and redaction/truncation must not collapse them. Cross-stream ties are explicitly unordered (§4:75), yet §6:104 requires a cursor for a capped multistream response. Select and source any provider tie mechanism; otherwise bound any retained observation and expose its actual partial/non-resumable boundary without claiming exhaustive provider pagination. If an immutable retained view is chosen, expiry, eviction and key loss must invalidate it, never trigger a new query disguised as continuation. The existing SDK signs an opaque continuation (`crates/connectors-sdk/src/lib.rs:154–210`); it proves neither upstream progress nor completeness. Historical `project_loki` only flattens and caps (`historical/crates/integration-monitoring/src/projection.rs:187–235`).

2. **LD-B-02.** The contract states only `end-start <= maximum`, supplies no endpoint inclusivity, and permits `end=now` without saying when that value becomes fixed. §3:62 requires a string timestamp while §6:109 requires null for missing pod timestamps; a null entry has no defined place in the promised timestamp ordering. Pod input is relative `since_seconds`/tail, Docker input uses seconds, and neither example accepts the output's direction/end window. Define each profile's actual supported selector/window, integer range and checked arithmetic, precise provider conversion, fixed observation bounds and null/order behavior. A future cursor must retain the original absolute window and admitted ordering; converting or advancing by an invented timestamp precision is unsafe. Do not promise a common provider interval that a selected pod/container endpoint cannot enforce.

3. **LD-B-03.** A provider count below the requested limit establishes no local no-loss result when stream or byte bounds dropped data; §3:65 supplies no complete rule for those cases. §4:76 requires every active bound to record itself, but §3:66 supplies one `truncation.by`. Distinguish line-content clipping from omitted occurrences and provider exhaustion from an arbitrary cap, including exact-cap and empty responses. State whether multiple causes accumulate or follow a deterministic precedence without hiding incompleteness. The old projector silently skips malformed stream/value shapes and defaults a missing result type; those historical choices cannot support a new exhaustive log claim. The pinned old Loki surface explicitly accepts stream or metric data (`historical/specs/loki/http-api-2026-08-15.openapi.yaml:14,39`), so a selected logs profile must reject an unsupported result kind rather than turn it into an empty complete log result. Preserve trustworthy partial data only under an explicit rule.

4. **LD-B-04.** §4:73 simultaneously leaves native LogQL uninterpreted and requires allowed selectors to be enforced before dispatch, but selects no proof for that containment. String occurrence checks cannot establish selector semantics when matching text can appear in line filters, quoted values or other query syntax. Select a reviewed query-admission/selector interpretation or a sourced provider-enforced isolation boundary, while preserving the intended native query capability and admitted query bytes. The listed cache key (§4:79) and cursor binding (§3:67) omit relevant projection/line bounds and authorization context required by `docs/design.md:446,515`; the tenant header's configuration fence (§4.1) alone does not cover credential/permission/parent-route changes. Specify current admission and exact evidence reuse for every read and continuation, original observation time, and invalidation or refusal under current scope changes. Native query/profile recognition must not become an undisclosed metadata-only or fixed-query substitute.

5. **LD-B-05.** An unseen Confluence page ID supplies no local space proof. The old page GET asks for body/version, not space (`historical/providers/confluence.toml:510–537`). The Jira historical catalog expressly requires an issue key for hosted project admission (`historical/providers/jira.toml:326–335`); its hosted implementation checks the key prefix before dispatch (`historical/crates/integration-jira/src/backend/operations.rs:332–365`) and exact returned key after the read (`:388–404`). This does not prove that a numeric ID or moved alias is in a configured project. Preserve intended by-ID full document reads through an explicit selected path: current caller/connection/profile admission first; bounded fixed-origin membership lookup only when independently permitted; exact returned object/site/scope identity before dependent access. Define closed lookup response/expansion fields, count and byte/deadline budget, no body-bearing expansion or arbitrary returned-link fetch, and deterministic unknown/not-found/denied/unavailable handling. Keep zero-provider-call guarantees only where the receiver already has enough trusted local information to refuse.

6. **LD-B-06.** A page/issue can move between a successful metadata lookup and the subsequent body request, and a cached body version is not a sourced membership-change token. Select the actual guarantee: if forbidden body retrieval itself is prohibited, establish a provider-backed atomic scope-constrained or conditional body read tied to the proven membership, rather than claiming that two separate observations are atomic. If a bounded retrieval with a checked disclosure boundary is the supported guarantee, state that admission explicitly and do not call it proof that no forbidden body was fetched. A post-fetch scope check can protect release but cannot undo retrieval. Define authoritative same-object membership/body/version correlation, missing or conflicting metadata refusal, what happens when scope changes during the read, and cache membership freshness/current authorization without trusting caller space/project assertions or refreshing observation time. A content version alone must not bypass current policy, identity, configuration or membership admission. The current design already requires current cache admission (`docs/design.md:515–518`); this profile has not selected how to establish it for moving opaque IDs.

7. **LD-B-07.** The historical-disposition row says Jira's body is the full `fields` object (§2:25), the common field rule says `body.content` is a UTF-8 string (§3:53), and the fixture requires all fields under `fields` (§6:84). Choose a precise declared JSON value or a precisely encoded string representation; a truncated JSON-text prefix cannot remain a valid structured object by assertion. Retain custom fields and their provider values within the selected representation, with explicit provider version/null semantics rather than an invented global Jira version. Align the compatibility row (`contracts/service/compatibility.md:144`) and selected input/output schemas. §9:102 also calls a `representations` list an existing OperationDeclaration value, while the actual entity contains only `profile` plus schema/mapping fields (`ess/domains/declarations.yaml:87–103`); mark the new declaration/value obligations accurately instead of alleging an existing typed field. This is specification/model truthfulness, not a demand for a runtime codec or a new content entity.

8. **LD-B-08.** `body.bytes` claims the full pre-truncation length (§3:54), while §5:74 assumes a 1 MiB output body ceiling below the 4 MiB upstream response limit suffices. A much larger provider body cannot be fully counted after the transport has rejected or stopped that response; JSON escaping/envelope bytes also differ from decoded UTF-8 body bytes. Define the measured representation and stage, valid request bounds and overhead accounting, and a safe oversized-response refusal instead of fabricating an exact total. Ensure version, body and count refer to the same bounded provider observation. The Confluence search disposition permits up to 100 body-bearing items (§2:24; old `providers/confluence.toml:720–740`), so per-item 1 MiB limits alone do not bound source work or the complete result. Select per-read/global limits, body-bearing search membership checks and explicit truncation/refusal/continuation without hidden per-item lookups that reset budgets. Preserve the declared body capability rather than silently switching search or document GET to metadata.

What was read: both owner-story bodies, both datasource contracts, relevant Grafana/Atlassian summaries, service compatibility and existing cursor code, auth admission/evidence and design datasource/cache/ownership rules, and relevant declaration types; exact baseline evidence is frozen in 45 files in `source-hashes.json`. Ten exact historical files are frozen in `historical-hashes.json` at `81459ac42ddd518d3942f4b079841e9e0ed6efc8`; relevant provider declarations, Loki projection/spec, Kubernetes log declaration and Jira admission/projection paths were inspected using read-only `git show`, `rg` and line-numbered reads. Source snapshots include context that was not exhaustively re-reviewed.

Limits: no runtime suite, ESS compilation, provider calls, network/vendor research or peer report was used in this initial pass. Provider behaviors needed for an actual selected continuation/membership proof remain subject to the root's independent official-source research. The frozen Loki OpenAPI identifies itself as repository-authored (`:3–7`), not independent current vendor evidence. Historical hosted Jira narrows detail fields (`historical/crates/integration-jira/src/backend/datasource.rs:109,450–520`) while the catalog declares full fields; this report distinguishes those paths and does not require copying the historical projection. No claim is made that public codecs, log/document value types, retained views or admission mechanisms are currently implemented. Adjacent completed auth, mutation, discovery and persistence protocols are interaction constraints only, not reopened stories.

```findings
- file: contracts/datasources/logs/v1alpha1/semantics.md
  line: 67
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: 'LD-B-01 (P1; owner story:contracts-log-continuation): Replace timestamp-only continuation with a selected occurrence-preserving
    progress rule or explicit non-resumable partial output, including saturated ties and repeated identical entries.'
- file: contracts/datasources/logs/v1alpha1/semantics.md
  line: 74
  category: semantics
  severity: warning
  verdict: needs-revision
  origin: pre-existing
  message: 'LD-B-02 (P2; owner story:contracts-log-continuation): Define exact interval endpoints, timestamp syntax and precision,
    fixed default end time, and ordering for each selected log profile before deriving any continuation boundary.'
- file: contracts/datasources/logs/v1alpha1/semantics.md
  line: 65
  category: semantics
  severity: warning
  verdict: needs-revision
  origin: pre-existing
  message: 'LD-B-03 (P2; owner story:contracts-log-continuation): Specify completeness and truncation from provider exhaustion
    and every local bound, with safe handling of wrong result kinds, malformed entries and simultaneous truncation causes.'
- file: contracts/datasources/logs/v1alpha1/semantics.md
  line: 73
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: 'LD-B-04 (P1; owner story:contracts-log-continuation): Select a trustworthy native-query scope admission mechanism
    and bind its source, authorization and projection inputs to every initial read, continuation and cached result.'
- file: contracts/datasources/records/v1alpha1/semantics.md
  line: 63
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: 'LD-B-05 (P1; owner story:contracts-document-admission): Replace the impossible opaque-ID zero-provider-call rule
    with a selected bounded membership-admission path that distinguishes locally known refusal from an explicitly admitted
    metadata lookup.'
- file: contracts/datasources/records/v1alpha1/semantics.md
  line: 68
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: 'LD-B-06 (P1; owner story:contracts-document-admission): Define the membership-to-body observation guarantee and
    cache reauthorization rule so moved documents and stale membership cannot inherit content access from an earlier lookup
    or body version.'
- file: contracts/datasources/records/v1alpha1/semantics.md
  line: 53
  category: semantics
  severity: warning
  verdict: needs-revision
  origin: pre-existing
  message: 'LD-B-07 (P2; owner story:contracts-document-admission): Choose one faithful Jira fields representation and truncation
    encoding, then align the document schema, conformance scenario, compatibility projection and actual ESS modeling claim.'
- file: contracts/datasources/records/v1alpha1/semantics.md
  line: 54
  category: semantics
  severity: warning
  verdict: needs-revision
  origin: pre-existing
  message: 'LD-B-08 (P2; owner story:contracts-document-admission): Define exact body-byte accounting and whole-response refusal
    rules within bounded provider work, including oversized source bodies and list projections carrying multiple documents.'
```
