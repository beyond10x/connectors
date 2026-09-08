needs-revision

Independent reviewer A initial semantic review of the two existing owners, at Connectors v2 baseline `dd08cfd60aa43740da21946666dc0fa89c1160dc`. Eight P2 findings: three owned solely by `story:contracts-log-continuation` (F12), five solely by `story:contracts-document-admission` (F14). This is a review of current normative meaning, not an implementation request or a finding that draft planning artifacts should already be complete.

Inputs were frozen with `git show` before analysis. `source-hashes.json` contains 38 baseline files; `supplemental-source-hashes.json` contains three additional baseline files. Nine old-repository files are pinned at `81459ac42ddd518d3942f4b079841e9e0ed6efc8` in `old-source-hashes.json`. Each manifest identifies the exact snapshot, SHA-256 and byte count. All manifest digests were independently checked against frozen bytes with zero mismatches. Citations below refer to those frozen repository-relative paths; `old:` denotes the pinned old repository, not a current vendor guarantee.

story:contracts-log-continuation — LD-A-01 (owner: story:contracts-log-continuation; P2): Timestamp-edge continuation with unordered ties does not establish progress or preserve multiplicity, so select exact inclusive/exclusive interval and tie-exhaustion rules with a bounded nonresumable partial outcome when the source cannot prove a safe next position. — contracts/datasources/logs/v1alpha1/semantics.md:67

The contract promises a re-readable cursor for 3 streams × 600 entries at a 1,000-line limit (§6:104), advances only to the last timestamp (:67), and leaves cross-stream ties unordered (:75). For 1,001 entries at T, inclusive replay can repeatedly return the same 1,000; excluding T skips the unseen entry. Filtering by line text or even timestamp/stream/text loses genuine duplicate occurrences; local sorting cannot reveal unseen ties. Define forward and backward interval boundaries, timestamp syntax/precision and edge arithmetic, ordering and duplicate multiplicity, the source evidence that establishes a completed boundary, and a strict progress measure for each cursor. Preserve useful continuation where a bounded source mechanism or immutable retained page can prove it; do not either promise arbitrary tie recovery or disable all ordinary pagination merely to pass the scenario. A saturated unprovable boundary needs `complete: false`, no fabricated continuation and an explicit bounded nonresumable interpretation. New independent reads are not guaranteed continuations. Old `projection.rs:187–235` flattens a bounded prefix and emits no cursor; the authored Loki source `specs/loki/http-api-2026-08-15.openapi.yaml:16–36` has only time/window/direction/limit parameters and does not prove a stable tie selector.

story:contracts-log-continuation — LD-A-02 (owner: story:contracts-log-continuation; P2): The profile returns next_cursor without defining a cursor input or a complete immutable continuation context, so specify resume framing, original-window identity, current authority checks, source consistency and invalidation instead of leaving callers to edit time edges. — contracts/datasources/logs/v1alpha1/semantics.md:32

The logql input example (:29–33) has no cursor, while output (:55) and the field rule (:67) return one. The latter binds only query/window/direction/connection/configuration, while design §9.1:436–446 requires source, scope, projection/contract revision and authorization context to be validated on every read. Specify whether resume repeats the original bounded input or supplies a separate closed variant, reject mixed/changed inputs, and freeze the first request's default `end=now`. Define bounds, redaction/projection, provider tenant and direct/mediated source binding; current authority is still required and a token is never a grant. Specify immutable buffered observation versus live provider re-query, late arrivals/deletions, cursor expiry/restart/reconfiguration and changed-scope outcomes without promising a provider snapshot. The SDK `Cursors` at :154–210 authenticates a caller-supplied context and opaque continuation string; it does not implement Loki continuation, ordering, source consistency or admission. Its presence cannot establish the SDK-obligation claim at logs:123.

story:contracts-log-continuation — LD-A-03 (owner: story:contracts-log-continuation; P2): The complete and truncation rules do not distinguish provider exhaustion, locally omitted entries, shortened line content and unknown dropped counts, so select a truthful bounded reduction and source-work limits for every emitted partial result. — contracts/datasources/logs/v1alpha1/semantics.md:65

The table says complete requires fewer than `limit` lines (:65), singular `truncation.by` can name limit/bytes/time/provider (:66), while §4:76 says every bound records itself and §5 sets line, stream and total byte bounds. A read may simultaneously shorten a line, omit whole entries for bytes, hit the stream cap and receive a provider partial response. A small emitted count is not exhaustion; a capped provider response cannot reveal how many unseen streams were dropped. Define which count is compared (provider versus emitted), exact versus unknown dropped counts, how simultaneous causes are represented, and whether line-content truncation is independent of entry-set completeness. Keep zero entries with partial/source-continuation distinct from a completed empty window. Bounds must include full stream labels and serialized framing, not only line text. If tie recovery/lookahead/partition work is selected, give it finite call, byte, retained-entry and original deadline ceilings; 1,000 public lines cannot authorize unbounded scanning. Align the example, selected schemas, Grafana bounds (:21), and shared 4 MiB/15 s provider/20 s execution constraints in compatibility §7. Old projection :234 only tests line/stream caps and is not proof of this richer completeness contract.

story:contracts-document-admission — LD-A-04 (owner: story:contracts-document-admission; P2): An unseen opaque ID cannot establish allowed project or space membership before every provider call, so replace the blanket boundary with an explicitly admitted trusted membership or scope-constrained lookup followed by a separately gated content request. — contracts/datasources/records/v1alpha1/semantics.md:63

The error rule (:59), scope rule (:63), fixture (:83) and Atlassian adapter :119 all require allowlist refusal before any provider request. Input contains only an ID, representation and byte limit (:34); a new opaque Confluence ID supplies no receiver-owned space evidence. A Jira-looking key is also only a request coordinate: old `jira.toml:326` required keys for hosted admission, but that does not prove a current project after alias resolution or movement. Select trusted receiver evidence, a provider-side scope-constrained lookup, or an explicitly admitted bounded metadata authorization lookup. Define the fixed connection/site/destination/method, exact minimal requested/accepted metadata, allowed IDs/types, request/call/response/deadline bounds, no returned-URL or caller-space override, unknown/malformed/unavailable/refused outcomes and which facts may be disclosed. Host lookup admission must precede this lookup; membership lookup is not an implicit identity probe. A known local forbidden target can still produce zero provider calls; an unknown ID must not silently obtain body access from that rule. Apply the same gate to body-bearing Confluence search, whose preserved expansion includes body.storage (records:24; Atlassian:42), rather than filtering forbidden bodies after fetching an unconstrained result.

story:contracts-document-admission — LD-A-05 (owner: story:contracts-document-admission; P2): A cached document version or former membership does not establish current scope after movement, so define the exact membership evidence binding, freshness and body-read race decision including revocation, identity aliases and current cache-result admission. — contracts/datasources/records/v1alpha1/semantics.md:68

Records §4:68 caches by connection/id/representation/version but declares no membership observation, TTL or movement rule; design §11:515–517 separately requires source/identity/scope/policy partitioning, current admission and explicit freshness. A page can move from ENG to forbidden space after cached membership or between metadata and body requests, while the content version and bytes need not supply membership proof. A moved Jira issue may also be addressed through an old key; the selected provider binding must establish canonical returned object/project identity rather than treat a key prefix as authority. Bind membership evidence to the exact object, provider authority/site, checking connection and applicable credential generation, resource scope and configuration/policy; changes and known invalidation defeat age-valid evidence. Define the observation/decision point promised by admission: a two-request check with a short TTL alone cannot promise no forbidden body read across a move. Select verified source-side constrained/conditional retrieval or explicitly bounded observation semantics plus required response checks; distinguish preventing provider access from preventing result disclosure. Unknown or stale proof must not fall back to cache/caller assertions. Current result admission must also be required before cached bytes are served, retaining original provenance. F05 evidence §4.2:94–104 supplies host dispatch fencing, not an atomic transaction with provider movement.

story:contracts-document-admission — LD-A-06 (owner: story:contracts-document-admission; P2): The shared UTF-8-string and prefix-truncation rule conflicts with the promised native Jira fields object and preservation of all custom fields, so select per-representation content shapes and a valid bounded truncation policy without silently changing representation. — contracts/datasources/records/v1alpha1/semantics.md:53

The old-evidence disposition says Jira's body is the `fields` object (:25), the body.content rule requires a UTF-8 string (:53), the fixture applies a UTF-8 prefix (:80), and another fixture promises every custom field under `fields` (:84). A prefix of serialized JSON is generally not a valid fields object; keeping the object without a selected structural truncation rule violates the universal byte-prefix prescription. Select a schema and byte definition separately for confluence.storage and jira.fields.v2, including null/missing provider fields, bounded complete objects versus an explicitly named serialized representation, and truncation/refusal behavior. Preserve the useful native custom-field capability within admitted bounds; do not silently omit fields or relabel a partial object as a full native response. Keep wiki text within a Jira fields value distinct from the whole fields representation. Reconcile the document example, Atlassian operation map, compatibility:144 and any ESS decision values so schemas cannot appear to validate a contradictory interpretation.

story:contracts-document-admission — LD-A-07 (owner: story:contracts-document-admission; P2): Exact full-body byte counts and unconditional prefix results are not established by a capped provider response, so define bounded source decoding and byte accounting, unknown-length or overflow outcomes, and aggregate serialized-result limits for detail and body-bearing lists. — contracts/datasources/records/v1alpha1/semantics.md:54

body.bytes is the full pre-truncation body length (:54), max_body_bytes may be 1 MiB (:74), the full result must fit 4 MiB (:76), and §6:80 always expects a UTF-8 prefix plus exact original length. Obtaining an exact length for a huge body can require consuming beyond the source ceiling, and HTTP Content-Length is not the UTF-8 length of a nested, escaped or compressed body. Define raw/decoded/provider-document/content/serialized-result byte accounting, finite parser/source-work bounds and a safe outcome where the full body or length is unavailable. If complete bounded acquisition is prerequisite to prefix truncation, say oversized provider documents fail rather than claim an exact partial count. Define aggregate handling when JSON escaping/envelope metadata makes a nominally ≤1 MiB content exceed 4 MiB, or a body-bearing list has many individually valid items (records:24; Atlassian:42). Do not drop fetched but un-emitted items while advancing a provider cursor past them. This remains the document owner's adjacent body-admission/truncation decision; it does not require implementing a range/streaming profile.

story:contracts-document-admission — LD-A-08 (owner: story:contracts-document-admission; P2): The confluence-page profile inherits an old content-by-ID endpoint that explicitly permits other content kinds, so validate the returned canonical identity, page kind, declared body representation and coherent revision before returning or caching a document. — contracts/datasources/records/v1alpha1/semantics.md:14

The named profile is confluence-page (:14), preserved from old content-by-id (:22), but pinned old `providers/confluence.toml:537` explicitly states that this operation does not constrain content kind and can return blogposts. The current rules validate requested representation, not returned type/representation/cardinality. A wrong-ID, non-page, missing-storage, mismatched representation or incoherent body/version response must not become a successful page or trusted membership cache entry. Select required response identity/kind/space/representation checks and their safe protocol/refusal outcomes; decide deliberately whether non-page types are excluded or receive a separately named profile. Preserve the existing allowed null provider-version case (:66/:81) without inventing a version, while requiring body and any returned version/provenance to describe the same accepted observation. The old provider declaration is evidence of the extraction mismatch, not proof of modern endpoint guarantees.

Scope and evidence limits: I read the two owner bodies, frozen family/adapter contracts and necessary service/auth/ESS/design boundaries, with pinned old provider declarations and projection evidence. The current `connectors-contracts` library declares Page/Provenance/EndpointObservation/Column/QueryResult, not implemented LogRead/Document types. Existing ESS OperationDeclaration has string profile and payload-schema fields; it does not execute a pagination or membership-admission reducer. Future finite decision values may make examples checkable but must not be presented as runtime/provider proof or invented provider-object lifecycle ownership. No runtime tests, live provider calls, external source lookup, planning mutation, tracked edit or peer report was used. Provider facts requiring official confirmation remain root's separate evidence task. Existing settled prior stories are constraints, not newly re-reviewed checkpoints; this report approves neither owner until its listed semantic requirements are settled.

```findings
- file: contracts/datasources/logs/v1alpha1/semantics.md
  line: 67
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: "LD-A-01 (owner: story:contracts-log-continuation; P2): Timestamp-edge continuation with unordered ties does not establish progress or preserve multiplicity, so select exact inclusive/exclusive interval and tie-exhaustion rules with a bounded nonresumable partial outcome when the source cannot prove a safe next position."
- file: contracts/datasources/logs/v1alpha1/semantics.md
  line: 32
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: "LD-A-02 (owner: story:contracts-log-continuation; P2): The profile returns next_cursor without defining a cursor input or a complete immutable continuation context, so specify resume framing, original-window identity, current authority checks, source consistency and invalidation instead of leaving callers to edit time edges."
- file: contracts/datasources/logs/v1alpha1/semantics.md
  line: 65
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: "LD-A-03 (owner: story:contracts-log-continuation; P2): The complete and truncation rules do not distinguish provider exhaustion, locally omitted entries, shortened line content and unknown dropped counts, so select a truthful bounded reduction and source-work limits for every emitted partial result."
- file: contracts/datasources/records/v1alpha1/semantics.md
  line: 63
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: "LD-A-04 (owner: story:contracts-document-admission; P2): An unseen opaque ID cannot establish allowed project or space membership before every provider call, so replace the blanket boundary with an explicitly admitted trusted membership or scope-constrained lookup followed by a separately gated content request."
- file: contracts/datasources/records/v1alpha1/semantics.md
  line: 68
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: "LD-A-05 (owner: story:contracts-document-admission; P2): A cached document version or former membership does not establish current scope after movement, so define the exact membership evidence binding, freshness and body-read race decision including revocation, identity aliases and current cache-result admission."
- file: contracts/datasources/records/v1alpha1/semantics.md
  line: 53
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: "LD-A-06 (owner: story:contracts-document-admission; P2): The shared UTF-8-string and prefix-truncation rule conflicts with the promised native Jira fields object and preservation of all custom fields, so select per-representation content shapes and a valid bounded truncation policy without silently changing representation."
- file: contracts/datasources/records/v1alpha1/semantics.md
  line: 54
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: "LD-A-07 (owner: story:contracts-document-admission; P2): Exact full-body byte counts and unconditional prefix results are not established by a capped provider response, so define bounded source decoding and byte accounting, unknown-length or overflow outcomes, and aggregate serialized-result limits for detail and body-bearing lists."
- file: contracts/datasources/records/v1alpha1/semantics.md
  line: 14
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: "LD-A-08 (owner: story:contracts-document-admission; P2): The confluence-page profile inherits an old content-by-ID endpoint that explicitly permits other content kinds, so validate the returned canonical identity, page kind, declared body representation and coherent revision before returning or caching a document."
```
