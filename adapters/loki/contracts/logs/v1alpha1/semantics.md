# Loki logql-range/v1alpha1

- **Status:** proposed, not implemented.
- **Family:** datasources. Siblings: [records](../../../../../contracts/datasources/records/v1alpha1/semantics.md), [series](../../../../../contracts/datasources/series/v1alpha1/semantics.md), relational (in [service v1alpha1](../../../../../contracts/service/v1alpha1/semantics.md)).
- **Recorded:** 2026-09-08; ownership relocated 2026-09-09.
- **Shared dependency:** [datasource.logs/v1alpha1](../../../../../contracts/datasources/logs/v1alpha1/semantics.md). This file owns all Loki-specific semantics.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `datasource.logs/v1alpha1` |
| Profile | `logql-range` |
| Shared with records | provenance, completeness, cursor binding rules (`docs/design.md:434-446`); not the record page shape |

A log read returns a bounded observation of the provider's available logs, with explicit selection, order and truncation. Loki preserves native LogQL and returned stream labels. A result proves neither archival retention, absence of late arrivals, a provider-wide snapshot nor continuity across provider rotation/restart.

Loki continuation pages one immutable retained observation. It never reissues a provider query with an adjusted timestamp edge. The selected provider API supplies neither occurrence identity nor a tie cursor; timestamp/text/label hashes cannot distinguish two genuine identical entries. A saturated provider response therefore ends with a truthful non-resumable partial result after its retained entries have been delivered. The [provider evidence record](../../../../../docs/evidence/datasource-semantics-20260908/provider-evidence.md) identifies the exact sources and selection limits.

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| `loki-query-range`: one operation from the pinned Loki HTTP API spec; direct origin or an admitted mediated capability | `../connectors/providers/loki.toml` | preserve as `logql-range` |
| Old Loki projection: ≤ 500 streams, ≤ 1,000 lines, line text cut at 8 KiB with `truncated` per line, deterministic pattern redaction with `redacted` flag, `result_type`, per-line `timestamp`, `labels`, `line`; overall `truncated` | `../connectors/crates/integration-monitoring/src/projection.rs`, `project_loki` | preserve the shape and bounds as first-profile defaults; redaction becomes a configurable host-side filter, not a contract promise (arbitrary text cannot be proven secret-free, old design 08 amendment) |
| Loki tenant/header binding where applicable | `docs/design.md:500` | preserve as receiver-owned `http.extra_headers` configuration (§4.1), never caller input; the earlier `tenant_header` wording was a conceptual label, not a second supported key |

The official Loki v3.7.0 HTTP reference defines an inclusive start and exclusive end. Its pinned LogCLI implementation refuses a batch entirely occupied by the retained timestamp overlap (`query.go:177–200`). The predecessor's small Loki OpenAPI is repository-authored evidence, not a vendor specification. These protocol declarations do not prove conformance of a deployed provider, proxy or future decoder.

## 3. Types

Input, `logql-range`:

```json
{ "query": "{app=\"api\"} |= \"error\"", "start_unix_ns": "1788822000000000000", "end_unix_ns": "1788825600000000000", "direction": "backward", "limit": 200, "collection_limit": 1000, "max_line_bytes": 8192 }
```

Only query and start are required. Direction defaults to backward; limit and collection_limit default to 1,000; max_line_bytes defaults to 8,192. **limit is the public page size; collection_limit is the single provider query's entry cap.** Both are integers in 1–1,000 with limit <= collection_limit; max_line_bytes is an integer in 1–8,192. This explicit distinction permits small public pages while keeping total collection bounded.

Resume input is exactly `{"cursor":"opaque value"}`. Mixed initial/resume fields, explicit null aliases, unknown members, provider interval/step/since overrides and caller origin/route/header selectors are InvalidInput. Resume cannot change the original query, time window, page size or projection.

Output:

```json
{
  "lines": [ { "timestamp_unix_ns": "1788825599000000000", "stream": { "app": "api" }, "line": "…", "line_truncated": false, "redacted": false, "source": null } ],
  "selection": { "kind": "loki-range", "start_unix_ns": "…", "end_unix_ns": "…", "direction": "backward" },
  "order": "timestamp-backward",
  "complete": false,
  "truncation": { "causes": ["provider_limit"], "occurrences_dropped": null, "stream_groups_dropped": null },
  "next_cursor": null,
  "provenance": { "instance": "…", "resource": "loki:…", "observed_at_unix_ms": 0, "source_revision": null }
}
```

| Field | Rule |
|---|---|
| `timestamp_unix_ns` | canonical nonnull decimal string; conversion preserves actual source precision |
| `stream` | returned Loki labels |
| `source` | null |
| `redacted` / `line_truncated` | required Boolean flags for configured redaction and UTF-8-safe line clipping, independently |
| `selection` / `order` | closed profile variants below; no fabricated universal window |
| `complete` | source exhaustion AND no omitted occurrences AND final retained page; line-content clipping is separate (§5) |
| `truncation` | all applicable causes, plus exact counts only where the total is known (§5) |
| `next_cursor` | Loki only, iff another retained offset exists; null at a terminal partial result even when unseen provider entries may remain |

Loki selection is `{kind:loki-range,start_unix_ns,end_unix_ns,direction}` and uses order=timestamp-forward/backward. An output label changed by native LogQL is a value, not proof of an original selector or authority. Safe errors and byte accounting are in §5.

## 4. Rules

Current host principal, visible operation, source/connection, scope and applicable credential/route evidence are admitted before dispatch. Known revocation and current binding fences also govern publication and result disclosure. These are host ordering guarantees, not an atomic transaction with future provider permission changes. No stale-on-error, cross-principal cache reuse, live follow or automatic read redispatch is selected. F15 owns any later refresh/retry profile under its original shared budget.

### 4.1 Provider tenant binding

The direct Loki connection owns its admitted provider tenant. When required by the
deployment, configure `http.extra_headers: {"X-Scope-OrgID":"tenant-a"}` under the
[shared HTTP header rules](../../../../../contracts/auth/capability/v1alpha1/http-headers.md).
`tenant_header` was an earlier descriptive label, not a supported key. No caller
input may choose or override the tenant. Header placement and revision fencing are
future binding obligations; current HttpConfig does not implement the extension.

A mediated connection consumes the shared mediated-HTTP capability and requires a
parent route that establishes the configured provider tenant. It sends no direct
header override through that capability and refuses a missing guarantee without
fallback. Loki knows no concrete parent implementation.

### 4.2 Native LogQL scope

Configuration selects `query_scope.required_equalities`, a duplicate-free array of exact `{label,value}` pairs. Missing scope configuration is invalid; an explicitly admitted empty array means the entire configured provider tenant is allowed. Valid provider label names and nonempty required values avoid empty-label/missing-label ambiguity.

A reviewed parser conforming to Loki v3.7.0 must accept the complete native log-selector or log-pipeline expression and inspect every initial stream selector. Each must contain every required equality with the exact decoded label/value and equality operator. Extra matchers and native filters/parsers/formatters remain supported. A regex, text inside a string, a pipeline label filter or a rewritten output label cannot discharge an initial equality. Metric/sample syntax is not a log query: literal/vector expressions also implement the provider's LogSelectorExpr interface, so interface membership alone is insufficient. The selected root must actually be a stream selector or pipeline.

Parse failure, unsupported/non-log syntax or exceeding 16 KiB UTF-8, 4,096 syntax nodes or depth 64 is InvalidInput. A valid log query missing the required scope is Forbidden. Neither dispatches. The admitted original query bytes are URL-encoded and forwarded unchanged: no reserialization, injected filters, fixed-query substitute or remote parsing probe. A missing/unverified parser cannot advertise the scoped profile. The same predicate applies through mediation and to declared verification queries.

### 4.3 Loki collection and ordering

Input nanoseconds are canonical `0` or `[1-9][0-9]*`, at most 9,223,372,036,854,775,807. Reject signs, whitespace, leading zeroes, decimal/exponent forms and overflow. Resolve omitted end once from the trusted receiver clock; require start < end and checked end-start <=86,400,000,000,000. The single GET fixes both exact decimal times, direction, `limit=collection_limit` and JSON output. No provider default time or sampling interval is used. Start is inclusive and end exclusive in both directions.

Validate the entire bounded JSON response before publishing. Require successful status, resultType=streams, string labels, valid two-element timestamp/text tuples, in-window representable timestamps and provider order within each group. Reject duplicate JSON keys, missing result type, metric results, malformed entries, unsupported tuple extensions, wrong ordering/window and counts above collection_limit as Unavailable. No malformed-entry skipping or incomplete-JSON prefix may establish an observation. This selected plain Loki success format has no generic partial field; unknown warning/partial extensions must fail safely instead of being ignored.

Retain the first 500 provider stream groups in response order, counting all omitted groups and occurrences from the fully validated response. Groups with identical returned labels remain separate groups for this bound; native pipelines can make their labels equal. Flatten retained groups with private `(group ordinal,entry ordinal)` coordinates. Sort by timestamp in the requested direction; ties use ascending coordinates. Never deduplicate equal timestamp/labels/text, even after redaction or clipping. Coordinates identify positions in this captured observation only.

Apply any admitted deterministic redaction once, flag changes, and then take the longest UTF-8 prefix within max_line_bytes. Clipping shortens content, not the occurrence count. Intern group labels and retain the immutable resulting occurrences, index and original private context under the storage ceiling. If the whole observation cannot fit, return Unavailable before the first page; do not drop stored pieces while claiming complete continuation.

Provider exhaustion requires a complete successful validated response with fewer than collection_limit occurrences, no known partial execution, and a provider/proxy binding verified to honor that effective limit and surface incomplete execution. A hidden smaller limit invalidates this proof. Exact-cap responses remain conservative partial even if exactly that many source entries existed. A validated empty response can establish exhaustion under the same conditions. Stats and emitted page size cannot establish it.

### 4.4 Retained paging and current authority

The host's logical ReadCursorCachePort owns this bounded ephemeral observation and context; it is not another provider entity or discovery publication. Offset zero starts the initial page. A page is the longest consecutive slice fitting the fixed public limit and complete serialized response ceiling, including repeated labels, escaping, selection, cursor and provenance. Reserve sufficient cursor/envelope bytes when deciding the slice. If even metadata plus one occurrence cannot fit, refuse without a cursor. Only a verified empty observation yields an empty page; a nonterminal page is never empty.

A next cursor exists iff a later retained offset exists. It authenticates an unpredictable observation reference and offset, bound to instance, operation/profile/contract/projection revision, connection and configured provider authority/tenant, direct or mediated source and parent/route/credential/private fences, principal/tenant/scope/policy, exact query/window/direction, collection/page/line limits and redaction configuration. No credential, origin or private route locator is exposed in the opaque token. Current admission precedes disclosure of cursor validity and cached content; then every context coordinate must match. A cursor grants no authority.

Repeating an admitted cursor returns the same slice without provider I/O, with a strictly advancing next offset when present. All pages preserve the original provider observation time and source_revision:null. Each request has its own ordinary handling deadline but cannot extend the fixed observation expiry, at most 300 seconds from initial observation. Expiry, eviction, restart/key loss or changed context is StaleCursor after current admission; unavailable authority/cache infrastructure is Unavailable. Never regenerate a view, resolve a fresh now, query a different edge or fall back to direct access. A new initial read is a separate observation and may repeat or change entries.

Cross-request query caching remains disabled by default; the retained observation is the explicitly selected exception needed for its own continuation. Current known revocation and binding fences govern publication and disclosure, including every cached page. Successful earlier admission or unchanged query text cannot override them.

## 5. Limits and completeness

| Bound | Value | Source |
|---|---|---|
| Loki occurrences / stream groups | 1,000 / 500 per observation | selected from old projection |
| Line bytes | 8 KiB maximum after redaction, before result JSON escaping | old projection ceiling |
| Loki labels | <=128 labels/group, <=128 UTF-8 bytes/name, <=4 KiB/value and <=32 KiB serialized group; violation refuses, never clips labels | selected metadata bound |
| Loki provider response | independently <=4 MiB encoded entity and decoded/decompressed JSON; <=16 KiB headers | selected bounded full decode |
| Retained observation | <=4 MiB accounted storage including interned labels, occurrences/index and private context | selected ephemeral retention |
| Retained log pool | <=64 MiB / 256 observations per service instance, <=16 MiB / 64 observations per admitted principal/tenant partition; smaller configured limits allowed | selected host capacity |
| Public response | <=4 MiB including all framing, escaping and cursor | ordinary service binding |
| Window / relative selector | <=24 h where declared in the selected input | selected profile ceiling |
| Provider work | one log GET; no retry/follow | selected call budget |
| Deadline | existing 20 s outer / 15 s shared provider budget, including admitted mediated work | ordinary service binding |
| Cursor TTL | <=300 s from the original observation, never renewed by pages | selected retention |

These are receiver ceilings, not vendor defaults or measured throughput. Finite reads/cancellation do not prove immediate remote cancellation or bound a provider's internal scan/CPU work. Deployment-side query limits remain independently admitted requirements.

Count any admitted mediated metadata/permission work inside the same 15 s provider / 20 s execution budgets, including at-most-5 s connects; the selected route binding must declare its own finite call budget. No activation/identity probe or refresh is silently added to the read. Retention admission accounts backing allocations and context/index overhead atomically across concurrent requests; expire first, then least-recently-used eviction or refuse a new observation if it still cannot fit. Eviction makes affected cursors stale and never regenerates or extends their observations.

`complete = source_exhausted AND no_occurrences_omitted AND final_retained_page`. Line clipping is separate and may coexist with complete:true.

Truncation causes are duplicate-free in this order: provider_limit, provider_partial, stream_limit, page_limit, response_bytes, source_bytes, line_bytes. Record every applicable cause: provider_limit means saturated Loki collection; stream_limit means omitted groups; page_limit/response_bytes explain a retained remainder; source_bytes means a valid bounded streaming cutoff; line_bytes means clipping on this page. Provider_partial requires an explicitly supported partial indication; an unknown extension refuses. Retained remainders are not dropped occurrences. A terminal incomplete observation may have no cursor while provider_limit/stream_limit persists.

Occurrences_dropped and stream_groups_dropped are exact nonnegative totals only when fully observed evidence establishes them, otherwise null for each affected total. Provider saturation leaves unseen totals unknown, not zero; an exact local omission count is not the total including unknown provider losses. Line clipping alone drops zero occurrences. Redaction has its own flag and does not guarantee secret-free content.

Safe base errors apply. Provider 400 for an admitted native query is InvalidInput without raw provider text. Exhausting the original execution or shared provider budget is Timeout, before or during collection; timeout is not clean EOF or a byte-cutoff success. Other failed, malformed, oversized or interrupted provider observations are Unavailable. No raw provider errors, query text, origins or credentials enter ordinary diagnostics.

## 6. Conformance scenarios (`docs/design.md:988`)

- 1,001 occurrences at timestamp T, collection_limit=1,000 and public limit=200: retain every returned occurrence including identical lines, deliver five pages, finish incomplete with no cursor and unknown unseen totals. No timestamp-edge query follows.
- 999 source occurrences under collection_limit=1,000 and public limit=200: four full pages and one 199-entry page; only the final page is complete when nothing was omitted. Repeating a cursor returns its identical slice.
- Exact-cap source is conservatively partial; validated empty source can be complete. A provider returning 1,800 entries against a requested cap of 1,000 is a protocol failure, not a basis for promising recovery of 800 unseen entries.
- 501 fully observed groups below the occurrence cap: preserve 500 groups, count known omissions, report stream_limit and final incomplete. Byte/page/line/provider causes accumulate without relabeling unknown losses as zero.
- Fixture line of 20 KiB → `line_truncated: true`, `line` is an 8 KiB UTF-8-safe prefix.
- Window exceeding the maximum → `InvalidInput`, no dispatch.
- Loki 400 (bad LogQL) → `InvalidInput`; the error message contains no provider body text.
- Tenant header configured → present on the fixture request; caller input cannot set or override it.
- Required equality appears only in quoted line text, regex or a pipeline filter → scope refusal before dispatch. A native admitted selector/pipeline remains byte-for-byte unchanged.
- Metric result, missing type, malformed tuple, wrong window/order, oversized JSON or hidden lower provider cap cannot become complete empty logs.
- Expiry, eviction, policy/credential/parent-route/redaction changes or key loss refuse cursor reuse without a requery. A denied caller learns no private cursor status.

These are specification scenarios, not implemented fixture results. Detailed dispositions and executable shape evidence must record their actual verification boundary; ESS shape acceptance does not execute provider, ordering or authority decisions.

## 7. Compatibility

- New profile binding; no old wire contract for logs existed beyond operation results.
- [Service compatibility](../../../../../contracts/service/compatibility.md) is authoritative for the binding. Log schemas can inhabit the ordinary result slot, but require explicit profile support; framing compatibility does not confer log or cursor semantics on existing typed consumers. Tenant headers remain receiver configuration.


## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| `LogLine`, `LogRead` public codecs | future `connectors-contracts` binding; the current crate has no such implemented types |
| Immutable observation retention and admitted offset cursor | future SDK/host binding; the existing `crates/connectors-sdk/src/lib.rs:161` helper authenticates a token but proves neither retention, progress, exhaustion nor scope |
| Native LogQL admission, bounded decoders and current disclosure fences | future adapter/host binding; no implementation provided by this contract |
| Receiver-owned `http.extra_headers` placement and validation (§4.1) | future `HttpConfig` / `HttpCapability` binding; current host configuration does not implement this extension |

## 9. ESS entities

LogQL range selection, direction and required selector equalities are typed in the
[Loki-owned model](../../../spec/ess/domains/reads.yaml). Shared collection/page facts and
decisions remain in `ess/domains/datasource_reads.yaml`; adapter predicates and
runtime enforcement are not supplied by these private shapes.

| Entity / value | Notes |
|---|---|
| `OperationDeclaration.profile = logql-range` | value |
| Streams, lines | not entities; payload |

Proposed provider-independent collection/page facts and completeness decisions belong in `ess/domains/datasource_reads.yaml`; native selections remain adapter-owned as above. Provider grammar, full context equality, occurrence preservation/order, decoding, clocks, numeric/byte bounds and atomic current publication remain explicitly UNMAPPED binding obligations. No persistent provider entity, parser, public codec or cache implementation is introduced.

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Redaction | host filter, opt-in, flagged per line; not a contract guarantee |
| Window maximum | 24 h |
| Durable/archival continuation and live follow | separate unselected profiles; finite native queries and retained Loki paging remain selected |
