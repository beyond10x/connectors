# Atlassian document profiles/v1alpha1

- **Status:** proposed semantics and private ESS values; no adapter, public codec or runtime cache implementation.
- **Shared dependency:** this adapter binding specializes the shared datasource contract of the same family and version. Native rules and future conformance fixtures are owned here.
- **Base:** bounded record reads in [service v1alpha1](../../../../../contracts/service/v1alpha1/semantics.md), with explicit selected-payload compatibility in [service compatibility](../../../../../contracts/service/compatibility.md). A new payload is not an extension accepted by today's closed Page reader.
- **Evidence:** [exact provider sources and their limits](../../../../../docs/evidence/datasource-semantics-20260908/provider-evidence.md). Historical declarations are evidence of intended behavior, not proof of current provider guarantees.

## 1. Identity and preserved behavior

| Profile | Selection | Provider-native representation |
|---|---|---|
| `confluence-page` | One current, ordinary page by canonical page ID in configured spaces | `confluence.storage`: decoded storage string, without sanitization or conversion |
| `jira-issue` | One issue by ID or provider-resolved key in configured projects | `jira.fields.v2`: the complete provider-returned `fields` JSON object, including custom fields |
| `confluence-cql` with body projection | Native CQL candidate search, followed by current scoped page/body retrieval | Each delivered item has the same `confluence.storage` body rules |

A document is unpaged. Search remains a list with provider continuation. A Jira issue is not a Confluence page and has no invented global content version. Jira REST v2 field values remain native, including v2 wiki strings; ADF, attachments and range reads are deferred.

The predecessor's `confluence-page-get` requested `body.storage,version`; its page search also requested bodies and space metadata. This design preserves both body capabilities but selects current v2 collection reads for scoped body retrieval. The predecessor's Jira catalog requested full fields, while one hosted datasource implementation projected fewer fields; this profile preserves the declared full-field capability, not that historical projection.

## 2. Authority, identity and observation boundary

The receiver owns the site/cloud ID, fixed provider origin/base path, connection, auth profile, current scope and policy/configuration revisions. The caller supplies a document coordinate, never a destination, tenant, project/space grant, expansion or provider URL. Invocation and any metadata lookup require current host admission through the selected authenticated transport. A mediated binding also requires current parent/route admission; it cannot substitute a different source.

Configured scopes use canonical positive signed-64-bit provider IDs represented as decimal strings, with no leading zero or float conversion. Confluence space keys and Jira project keys are display/query coordinates associated with those IDs by admitted configuration validation; key text is not the authority. A configuration has at most 100 allowed spaces or projects. An empty allowlist admits no read. Site, scope or identity changes activate a new binding and invalidate dependent cache/cursor reuse.

**Selected guarantee:** the host admits a bounded request using receiver-owned scope predicates, and releases content only after the response establishes the required exact object and scope correlation. Each body request carries those provider-side predicates. No pair of metadata/body calls is claimed atomic. Provider filtering and returned membership describe the provider observation, not a lock against a move after that observation; fresh membership at the instant a remote caller receives the response is not promised. A stricter no-race retrieval guarantee requires a different provider binding with evidence for that guarantee.

A host-known policy refusal still makes zero provider calls. An unseen ID has no such local proof: it follows the independently admitted lookup/scoped-read path below. A caller assertion, key prefix, cached membership, matching content version or short TTL cannot grant content access.

Immediately before each provider dispatch and before final result/cache disclosure, re-evaluate current admission and compare the source, connection identity, credential generation where applicable, scope, auth/policy/configuration and profile/projection bindings. Known revocation or a mismatch refuses that attempt; no retry silently changes the selected identity. These host checks do not make independent provider operations transactional.

## 3. Document payload

A closed input contains exactly `id`, `representation` and optional `max_body_bytes`:

```json
{"id":"123456","representation":"confluence.storage","max_body_bytes":262144}
```

Confluence accepts only canonical positive signed-64-bit decimal page IDs. Jira accepts that numeric form or a nonempty UTF-8 issue-key coordinate of at most 256 bytes without control characters; the latter is percent-encoded as one path segment for provider resolution, never interpolated into JQL. Malformed decimal IDs are not rounded. An unknown field/representation or a non-integer, boolean, zero, negative or over-ceiling body limit is `invalid_input` before dispatch. Default `max_body_bytes` is 262,144; permitted values are 1–1,048,576 bytes.

The selected result has `item`, `body`, `complete` and `provenance`; no document cursor:

```json
{
  "item":{"id":"123456","title":"Example","status":"current","space_id":"42",
          "version":{"number":7,"when":"2026-09-01T10:00:00.000Z"}},
  "body":{"representation":"confluence.storage","bytes":12000,
          "content":"<p>...</p>","truncated":false,"truncation":[]},
  "complete":true,
  "provenance":{"instance":"selected-instance","resource":"confluence:page:123456",
                "observed_at_unix_ms":0,"source_revision":"7"}
}
```

The byte count above illustrates the shape; it is not a byte-accurate test vector.

| Field | Rule |
|---|---|
| Confluence `item` | Canonical `id`, `title`, `status: current`, canonical `space_id`, and nullable `version`; ordinary `subtype: page` is required from the source |
| Confluence `version` | Null when absent; otherwise the provider's positive integer `number` and nullable provider creation time as `when`; missing optional time stays null |
| Jira `item` | Canonical numeric `id`, provider-returned canonical `key`, `project {id,key}`, and `version: null`; no Confluence title/status fields are invented |
| `body.content` | Confluence: string, possibly shortened; Jira: complete native JSON object, or null only for explicitly reported whole-body omission |
| `body.bytes` | Exact full representation length measured from the fully received, valid, bounded body observation under §6; never a guess after an upstream cutoff |
| `body.truncation` | Ordered distinct array drawn from `body_bytes`, `result_bytes`; empty iff `truncated: false` |
| `complete` | True for a successfully established single authorized object; independent of body shortening/omission; provider partial/malformed detail responses are errors |
| `provenance` | Exact source/instance and canonical resource; original body observation time; source revision is Confluence's version number as decimal text, or null |

Required identity, scope or selected body fields missing from a purported success cause `unavailable`. Unknown provider metadata is not copied into the selected item. Provider-native Jira fields remain nested under `body.content`, with their original names and JSON values; a missing or non-object source `fields` is an invalid upstream result, not an omitted body.

## 4. Selected provider traces

### 4.1 Confluence by ID

Use the fixed site's `GET /wiki/api/v2/pages`, with the one exact `id`, receiver-owned `space-id` set, explicit `status=current`, `subtype=page`, `limit=1` and, for a body read, `body-format=storage`. Neither the caller nor a response link supplies an unscoped `/pages/{id}` fallback. The v2 default includes archived pages, so omitting the status filter is not equivalent.

A non-cached read needs one scoped body request. A possible cache hit first makes the same scoped request without `body-format`; this metadata observation may establish current membership and version for §5. A changed/absent version or unusable cache entry requires a fresh scoped body request, within the same two-call budget. The body request repeats every scope predicate rather than inheriting an earlier membership decision.

The selected v2 source is `PageBulk`, not the detail Page shape. Accept exactly one returned current ordinary page with the requested canonical ID, a space ID in the configured set and the selected storage body. More than one object, duplicate/mismatched identity, inconsistent status/subtype or unexplained continuation for this one-ID read is `unavailable`. An empty scoped result is `not_found`: it does not distinguish missing, inaccessible or outside-scope pages. A returned conflicting scope is a provider-protocol failure, never content to release.

### 4.2 Jira by ID or key

A numeric ID can proceed directly to the scoped body query. A key first uses the fixed site's `GET /rest/api/2/issue/{issueIdOrKey}` with `fields=project`, no expansions/properties and no body-bearing field request. Metadata lookup needs its own admission and the smaller source-byte ceiling. Only canonical issue `id`, returned `key` and `fields.project {id,key}` are consumed; incidental provider metadata is private and discarded.

The provider documents case-insensitive and moved-key resolution without redirect. Therefore a different returned key is permitted, but never grants the project named by the old key. Missing/malformed canonical identity or project is `unavailable`; missing, inaccessible or currently outside-configured-project metadata produces the same safe `not_found` result. Neither body nor foreign project metadata is released.

Use `GET /rest/api/2/search/jql` for the body observation, with a receiver-constructed bounded predicate `id = <canonical-id> AND project IN (<allowed-project-ids>)`, `reconcileIssues` containing exactly that ID, `fields=*all`, and `maxResults=1`. No caller JQL is accepted by this detail profile. Reconciliation and the scope predicate are repeated together; a plain eventual-index search is not a replacement. Validate exactly the selected issue ID and a current returned project in the allowed ID set. Reconcile identity, not key text. A project change between lookup and query is accepted only if the body observation itself satisfies current admitted scope.

An empty exhausted result is `not_found`. Unexpected pagination, duplicate or different issue identity, absent project correlation, or a partial/malformed response is `unavailable`. The provider-visible full `fields` object is retained without renaming custom fields; permission-hidden fields are not invented. Jira `updated` is not a global version, so this profile has null version/source_revision and no body-cache reuse.

### 4.3 Lookup and transport failures

Every dispatch uses the fixed bound origin and profile-owned path/method/parameters; redirects and arbitrary returned links are not followed. Metadata acquisition is not a generic identity-probing operation exposed to callers. A response never supplies new host, credential, expansion or scope authority.

For Confluence pagination, accept at most one unambiguous next position from the selected endpoint's `_links.next` or HTTP Link `rel=next`; when both are present they must identify the same cursor. Resolve a relative link only against the configured request URL, require the same HTTPS origin and exact endpoint path, and reject userinfo, fragments, duplicate query keys or unsupported parameters. The single nonempty decoded cursor is at most 4,096 UTF-8 bytes. Any other supplied query parameter must equal the receiver's already fixed value; omitted parameters are restored from private context, never from defaults. Dispatch a newly constructed request to the fixed endpoint with that extracted cursor and the original filters/expansions/limit. A next position equal to the current position or an ancestor in that cursor's immutable lineage is `unavailable`, not progress. Replaying a public cursor uses its original lineage; later or branched positions do not retroactively become its ancestors. No-next is established only after a complete valid response; a malformed next link is never exhaustion.

Host denial is `forbidden`; invalid input is `invalid_input`; admitted scoped absence is `not_found`; expired execution is `timeout`; missing binding guarantees, invalid/oversized upstream content and unusable source observations are `unavailable`. Current auth failures follow the shared auth/error contract without leaking provider bodies. This profile grants no refresh or extra provider retry budget; the read-refresh owner must define any such composition separately.

## 5. Cache admission and moves

Only Confluence results with an actual provider version may be body-cache candidates. The partition includes source/site, instance, connection and external identity, credential generation where applicable, allowed-scope identity, policy/configuration and auth/profile/projection revisions, canonical page ID and representation. The stored value retains body version, exact original bytes/count and original observation time. Store only a fully received native body observation, not a previously shortened public response.

Maximum body-observation age is 300 seconds or the configured smaller TTL. No stale-on-error reuse is allowed. A cache hit still requires the fresh scoped metadata observation in §4.1, the same canonical ID/current allowed space and equal nonnull content version, plus current host/result admission. That check's time never replaces the original body observation time. A move into a different allowed space may be served only when the fresh metadata establishes it and the binding remains authorized; old membership alone is never used.

If membership/version cannot be established, fail or perform the admitted fresh scoped body read within the original budget; do not return a stale success. Source/policy/configuration/credential changes or known invalidation defeat a TTL-valid entry. Lost cache state is a miss; expiry and eviction do not invent a new observation. Metadata/body separation remains the observation contract of §2, not atomic provider membership locking.

## 6. Body and whole-response bounds

Every upstream response is completely received and strictly parsed inside its byte/time ceiling before an exact `body.bytes` or successful result is claimed. Invalid UTF-8, malformed JSON, duplicate object keys or a selected native value that the lossless binding cannot preserve causes `unavailable`. In particular, JSON numbers must not silently round through binary floating point.

For Confluence, `body.bytes` counts UTF-8 bytes of the decoded storage string, before public shortening. Select the longest Unicode-scalar-aligned prefix fitting both `max_body_bytes` and the full serialized service-result budget; serialization includes JSON escaping, metadata, provenance and envelope framing. This is a text prefix, not a claim that truncated XHTML is well-formed or safe to render. Preserve the complete stored provider version and original count.

For Jira, `body.bytes` counts the UTF-8 source JSON token span of the complete `fields` object, including its internal whitespace/escapes. The selected lossless binding preserves that native object without numeric conversion. If the complete object does not fit the body bound or remaining serialized-result budget, emit `content: null`, `truncated: true` and the applicable cause(s); never prune custom fields, manufacture an empty object, or label a JSON-text prefix as an object. Null here means omitted content and does not claim a null provider `fields` value.

A source cutoff gives no exact full-body count and no successful truncated object: refuse `unavailable`. A source Content-Length describes transport bytes, not decoded storage text or a nested fields object, and cannot supply the count. Output shortening does not relax upstream limits. If required metadata/envelope alone cannot fit, refuse rather than cut protocol fields.

| Budget | Selected ceiling |
|---|---|
| Input and serialized service result | 64 KiB and 4 MiB, including framing |
| Body limit per item | Default 256 KiB; maximum 1 MiB |
| One metadata-only provider response | Independently 512 KiB encoded entity and decoded/decompressed HTTP body; 16 KiB response headers |
| One body-bearing provider response | Independently 4 MiB encoded entity and decoded/decompressed HTTP body; 16 KiB response headers |
| Aggregate provider response bytes | Independently 8 MiB encoded and decoded entity bytes per invocation across all lookup/body pages |
| Provider requests | Detail: at most two; body-bearing CQL page: at most five total, including its source-search call |
| Time | One 20 s execution deadline and one shared 15 s provider-work budget across every call, including connects; each connect at most 5 s and all work within the remaining budgets |
| Retained body cache | Optional; at most 8 MiB accounted storage per entry, 64 MiB / 256 entries per service instance, and 16 MiB / 64 entries per admitted principal/tenant partition; smaller configured ceilings allowed |
| CQL continuation state | At most 64 KiB per chain, 1,024 live chains / 64 MiB per service instance, and 64 chains / 4 MiB per admitted principal/tenant partition; public token at most 4,096 UTF-8 bytes |

No per-item call, redirect, pagination or repair resets these budgets. Account retained allocations, indices and private context before admission; reservations and publication share the port's capacity fence across concurrent requests. Expiry and least-recently-used eviction may retire these ephemeral entries but cannot renew original age or erase another port's safety records. If optional body-cache insertion cannot fit, return the freshly admitted uncached result. CQL must reserve its continuation context before publishing a page that needs a cursor; capacity failure is `unavailable`, with no page/cursor advancement. An uncached detail binding may advertise without a body cache; a profile requiring continuation cannot advertise without these bounded state semantics. These are receiver-selected limits, not proof of a particular allocator or cache implementation.

## 7. Body-bearing Confluence CQL search

Preserve native CQL and its provider continuation; this is neither a fixed query nor a metadata-only replacement for the old body-capable search. The selected input declares `cql`, `limit` (1–100, default 25), `max_body_bytes` and a separate closed `{cursor}` resume variant. CQL is at most 8 KiB UTF-8. The initial query, ordering, scope, bounds and projection are immutable across its cursor.

A binding must parse the predicate and optional trailing ORDER BY using the selected [CQL admission grammar and rewriting rules](cql.md). It prefixes receiver-owned `type=page` and safely quoted space-key predicates, then the parenthesized original predicate; original ordering is reattached outside the conjunction. Unknown syntax or unsupported constructs refuse `invalid_input`; substring checks and concatenating an unparsed full query are not scope validation. The grammar is specified here; its parser/quoting implementation still needs binding evidence before advertisement.

First call `GET /wiki/rest/api/content/search` for metadata, with `expand=space`, the fixed rewritten CQL and public limit; omit cqlcontext and every body-related expansion. Require a complete valid collection with no more than limit entries, canonical distinct candidate IDs, page type, current status and well-formed space ID/key metadata. Reject malformed/duplicate/non-page entries instead of skipping them. Candidate scope is still untrusted until the scoped body observation; no candidate body is disclosed or treated as trusted. Current configured space IDs remain authoritative if a key is stale or reassigned. Provider CQL/index consistency, provider-side date functions and ordering are preserved as candidate-selection semantics, not promoted to a snapshot or a fresh-content predicate evaluation.

For at most 100 distinct candidate page IDs, use v2 `GET /wiki/api/v2/pages` with those IDs, the configured space IDs, `status=current`, `subtype=page`, `body-format=storage`, and bounded page size. Consume any v2 continuation only through the same fixed endpoint and immutable filters: extract an opaque cursor after validating the declared continuation form, never fetch an arbitrary link. There are at most four bulk-body calls within the five-call/8-MiB/original-deadline ceiling. Reject duplicate/unrequested IDs, inconsistent returned membership and malformed source pages.

Finish the bounded bulk enumeration before publishing the public source page. Reorder admitted bodies into the original CQL candidate order. An absent current scoped body is filtered out; it does not authorize unscoped detail lookup. If bulk enumeration cannot be exhausted within the limits, the whole public page fails `unavailable` with no public cursor advancement. There is no hidden lookup per candidate and no silent body-to-metadata downgrade. A caller may start a new search with a smaller page limit; that is not continuation of a failed page.

Apply per-item body bounds, then the 4 MiB complete-result bound. For aggregate pressure, shorten storage strings in stable item order: each gets the longest prefix fitting its own body limit and the remaining budget after reserving every item's required metadata and the continuation/provenance envelope. Mark `result_bytes` on affected bodies. If that reserved metadata cannot fit, refuse the page without advancing continuation.

The selected list result is closed `{items, complete, next_cursor, provenance}`. Each item is a complete document result from §3, including its own body observation provenance; body/title/version come from the same accepted v2 observation, not from stale CQL metadata. Root provenance names the configured Confluence page collection and the source-search observation time; it does not claim that all bodies were read simultaneously.

The public cursor retains validated provider continuation, consumed-position digests and the complete source/query/scope/order/bounds/auth/projection context through the bounded host cursor port. A chain permits at most 1,024 source pages and expires at most 300 seconds after its original search observation; a later cursor does not renew that expiry. Reaching the page ceiling while the provider still has continuation refuses the page as `unavailable`, without a false terminal success. Resume is `{cursor}` only; current admission and every binding equality are required again, before disclosing token validity. Expired, lost, evicted or mismatched state is `stale_cursor`; unavailable authority/state infrastructure is `unavailable`. A cursor is never a grant.

Each successful source page yields a new cursor only if the CQL source has continuation; an empty filtered page can therefore have `complete: false` and a cursor. An exhausted source page has `complete: true`, independent of per-body truncation. Repeating an admitted CQL cursor repeats the fixed provider request and may observe changed metadata/body results; unlike retained Loki paging, this profile does not promise byte-identical replay. No cross-page snapshot, current-query-membership guarantee, or absence of duplicates caused by live provider changes is asserted.

## 8. Required semantic and binding checks

- Unknown Confluence ID in an allowed space versus outside scope; a local host denial makes zero calls, while an admitted opaque ID uses the selected filtered route.
- A page moves between metadata and body reads, after a cached body, and after the provider observation; observations and disclosure claims match §2 without a fabricated transaction.
- An old/lowercase Jira key resolves to a different key or project; canonical ID plus reconciled project predicate controls the body read.
- Current authorization, parent route, credential generation, configuration and scope change before dispatch, resume, cache hit and final disclosure.
- Native Jira custom fields, nested objects, nulls and large exact numbers survive within bounds; an oversized structured body is explicitly omitted, not stringified or pruned.
- A 300 KiB storage body at a 256 KiB body limit gives an exact original count and scalar-aligned prefix; escaping and many body-bearing items also fit the full result.
- Oversized source, duplicate keys, wrong object identity, absent scope/body, malformed continuation, bulk-page limit and incomplete enumeration refuse without a partial public page.
- CQL OR/NOT/quoted literals and ORDER BY cannot escape the scope conjunction; stale space-key mappings cannot release bodies outside configured space IDs.
- Empty filtered CQL page with continuation remains resumable; loss/expiry/current denial does not become completed empty output.
- Cache metadata revalidation preserves the original observation time; absent version, provider outage and Jira updated fields do not manufacture reusable versioned content.

These are authored obligations, not executed adapter conformance. Textual traces and ESS shape checks must name that limit; actual parser/provider/transport/cache implementations need their own later tests.

## 9. Binding and ESS ownership

[Service compatibility](../../../../../contracts/service/compatibility.md) owns the wire-version boundary. Each document/search profile needs its own closed public input/output schema and faithful error mapping before advertisement. The existing generic result slot, Page type and SDK cursor signer do not implement this profile.

Private generic selection, observation and decision values belong in `ess/domains/datasource_reads.yaml`, where representation identifiers are opaque. The closed native representation and space/project vocabulary belongs in the [Atlassian-owned model](../../../spec/ess/domains/documents.yaml). Provider content is a declared native payload, not a newly invented persistent entity. `OperationDeclaration` currently has profile and schema/mapping fields; it has no existing `representations` list. This document's representation selection is a proposed profile-schema obligation, not a claim that such a field already exists.

Exact parsing, scoped provider guarantees, lossless JSON, byte arithmetic, current authority, cursor/cache bounds and observation correlation remain explicit UNMAPPED binding obligations wherever ESS cannot execute them. [Design §31](../../../../../docs/design.md#31-host-persistence-ownership-and-atomicity) owns host ports and unresolved durable ownership; no new Connection/cache ownership edge is guessed here.
