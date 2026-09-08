# F12 log dispositions and manual scenarios

Owner: `story:contracts-log-continuation` (original F12). Base before this
completion pass: `8e1836cad8ae1b2127ce9ae306c6d8131960db4c`.

The initial reports remain unchanged in the
[datasource intake](../../../../../../../docs/evidence/datasource-semantics-20260908/reviews/).
This implementor record supplies proposed dispositions and a manual textual audit;
it is neither an independent review verdict nor executed runtime conformance.
The base already contained the main remedies; this pass makes simultaneous/native
cause triggers explicit and retains previously missing native source evidence.

## Initial finding dispositions

| Finding | Selected correction and coverage |
|---|---|
| LD-A-01 | §4.3 captures occurrences without deduplication; §4.4 advances retained offsets, never provider timestamps. L01–L04, L08 cover multiplicity, bounded progress and conservative saturation. |
| LD-A-02 | §3 closes cursor-only resume; §4.4 binds original selection/source/auth/projection and TTL. L05, L06, L14–L16 cover replay, changes and loss. |
| LD-A-03 | §5 separates exhaustion, omissions and clipped content, independently accumulates causes and bounds all source/storage/result work. L03, L04, L07–L12, L17–L19 cover the reduction. |
| LD-B-01 | The same occurrence-preserving observation handles identical saturated ties without hidden requery. L01–L05 and L15. |
| LD-B-02 | §4.3 defines checked canonical signed-64-bit nanoseconds and [start,end); native pod/Docker profiles keep relative/second selectors and provider line order. L06, L13; native K01–K08 and D01–D10. |
| LD-B-03 | Entire JSON is validated, unsupported results/partial extensions refuse, and every selected cap has a cause or an explicit error. L03, L04, L07–L12, L17–L19 and native bounded-stream cases. |
| LD-B-04 | §4.2 inspects actual native selector/pipeline roots and required exact equalities before unchanged-byte dispatch; current complete context is checked on every page. L14–L16, L20–L23. |

All citations above refer to [Loki semantics](../../semantics.md). Native
Kubernetes/Docker scenarios reside with their owners; those bindings do not
inherit a LogQL selector or retained-page algorithm.

## Evaluated textual cases

Each row states the input/failure and the observation selected by the current
text. “Evaluated” means reading and applying those rules manually; there is no
provider, parser, cache or reducer implementation under test.

| ID | Input or failure | Required observation after correction |
|---|---|---|
| L01 | 1,001 source occurrences at T, including identical text/labels, collection_limit=1,000, public limit=200; returned groups all fit | Preserve all 1,000 returned occurrences across five pages. First four have provider_limit,page_limit; fifth has provider_limit, complete:false, no cursor. Unseen totals are null; no sixth provider call. |
| L02 | 999 validated occurrences, cap=1,000, limit=200, no other bounds | 200/200/200/200/199 occurrences, strictly advancing offsets; only the fifth page is complete. Dropped totals are zero. |
| L03 | Exactly 1,000 valid occurrences and clean success | Conservative provider_limit and complete:false on the terminal page, even if exactly 1,000 existed upstream. |
| L04 | Zero valid occurrences versus wrong resultType or unknown partial extension with zero lines | Verified empty streams result may be complete; wrong-kind/partial-extension result is Unavailable, never complete empty. |
| L05 | Replay the same admitted retained cursor before expiry | Same slice and next offset, same observation time, zero provider calls. A separately started initial read may differ. |
| L06 | Omitted end followed by resume; attempt mixed cursor+end | Initial receiver time resolves end once. Resume uses that bound; mixed input is InvalidInput rather than a fresh query. |
| L07 | 501 fully observed one-entry groups, below the occurrence cap | Keep 500 groups; final complete:false, stream_limit, exact dropped totals 1 occurrence/1 group. |
| L08 | Two provider groups have equal output labels; entries also become equal after redaction | Keep separate group ordinals and every occurrence; tie order is ascending original group/entry coordinates. |
| L09 | Public count reaches limit and adding next retained occurrence also exceeds 4 MiB | Both page_limit and response_bytes appear, in that order; remainder has a cursor and is not counted as dropped. |
| L10 | Bytes prevent adding next occurrence before public limit is reached | response_bytes only among page causes. Progress is nonempty; inability to fit one occurrence plus envelope refuses without a cursor. |
| L11 | An emitted 20 KiB line and simultaneous source saturation/group/page limits | line_bytes appears with every other applicable cause; content clipping never deduplicates or increments dropped occurrences. |
| L12 | One clipped line, otherwise complete exhausted single page | complete:true, line_bytes, zero dropped occurrences/groups; redacted is independently determined. |
| L13 | start=0, end=1; source timestamps 0 and 1; negative/leading-zero/overflow input | Timestamp 0 is included and 1 excluded. Out-of-window provider data refuses. Invalid canonical/range input makes zero calls; checked window maximum is 24 h. |
| L14 | Caller/tenant denied, otherwise authentic or stale cursor | Current denial precedes private cursor-status/content disclosure. No provider I/O or cache release. |
| L15 | TTL expiry, eviction, restart/key loss or source/parent-route/credential/scope/config/redaction change | After current admission, StaleCursor for lost/mismatched state; no reconstruction or direct fallback. Authority/cache infrastructure outage is Unavailable. |
| L16 | Known revocation between collection and publication, or before a later page disclosure | Current publication/disclosure fence refuses; previous success and token possession grant nothing. |
| L17 | Duplicate JSON key, malformed tuple, >collection_limit entries, reversed within-group order, out-of-window timestamp | Entire observation refuses Unavailable; no prefix publication or skipped malformed line. |
| L18 | Headers/source/labels/retained-allocation ceiling exceeded; pool cannot reserve even after allowed eviction | Unavailable before first page. Full source envelope and retained backing/index/context allocations count; no stored-piece dropping. |
| L19 | Provider deadline expires during acquisition; effective provider limit is unverified | Deadline exhaustion is Timeout. Missing effective-limit/incomplete-execution guarantee is Unavailable; neither proves EOF/exhaustion. |
| L20 | Required tenant equality only in a regex, quoted line, pipeline filter or output label | Forbidden before dispatch for a valid log query lacking the original selector equality. |
| L21 | Valid native selector and pipeline with exact required equality plus additional native matchers | Accepted syntax/scope is forwarded byte-for-byte after URL encoding; no fixed-query replacement. |
| L22 | Metric, literal/vector root, unparseable input, AST depth/node/query-byte overflow | InvalidInput with no dispatch; interface membership alone is insufficient. Unverified parser cannot advertise this profile. |
| L23 | Caller attempts provider tenant/header/origin change; mediated route lacks tenant guarantee | Closed input rejects caller overrides. Missing admitted tenant route refuses without direct fallback. Current receiver header configuration governs the direct request. |

There are 23 manual Loki cases. The Kubernetes and Docker owner records add
8 and 10 manual cases respectively. These counts are authored scenario counts,
not test-runner counts. Original input snapshots/reports remain immutable and
independent final review is still required before story closure.
