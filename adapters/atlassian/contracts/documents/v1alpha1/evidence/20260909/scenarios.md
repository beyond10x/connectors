# F14 document dispositions and manual scenarios

Owner: `story:contracts-document-admission` (original F14). Base before this
completion pass: `8e1836cad8ae1b2127ce9ae306c6d8131960db4c`.
The original [independent reports](../../../../../../../docs/evidence/datasource-semantics-20260908/reviews/)
remain immutable. This implementor record proposes dispositions and evaluates the
text manually; it is not an independent verdict or runtime conformance result.

## Initial finding dispositions

| Finding | Selected correction and cases |
|---|---|
| LD-A-04 | §§2/4 replace blanket opaque-ID local proof with admitted fixed-origin scoped collection reads or minimal key metadata. A01–A07, A25–A27. |
| LD-A-05 | §§2/5 name the actual provider observation, repeat body scope predicates and revalidate cached membership/current authority with original provenance. A04, A08–A13, A30. |
| LD-A-06 | §§3/6 retain a lossless full Jira fields object or explicit null omission, separately from storage prefixes. A14–A18. |
| LD-A-07 | §6 requires complete bounded source acquisition and exact representation counts, independent body/result causes and global list accounting. A16–A24, A27–A29. |
| LD-A-08 | §§3/4 require exact identity/current page/scope/storage/version correlation; nullable subtype and optional representation markers have explicit trusted-filter decisions. A02, A03, A08, A10–A13. |
| LD-B-05 | §4 selects bounded Confluence ID/space collection and admitted Jira key resolution plus reconciled ID/project search; host-known refusals still make zero calls. A01–A07. |
| LD-B-06 | Current dispatch/disclosure and cache revalidation do not claim a transaction or permit old membership/version as a grant. A04, A08–A13, A30. |
| LD-B-07 | Native payload/projection and honest ESS/OperationDeclaration claims are aligned in §§3/9; there is no stringified/pruned Jira object or invented global version. A14–A18, A31. |
| LD-B-08 | Full source envelopes, exact token/content lengths, all calls and aggregate output are bounded without per-item repair or failed-page advancement. A16–A29 and Q06–Q10. |

References above are to [native document semantics](../../semantics.md). The
shared family owns envelopes/common obligations; native grammar, traces and
representations stay in this adapter. The main fixes were already drafted at the
base; this pass settles optional source fields, simultaneous causes, bounded
immutable CQL branch state, quoting layers, source-envelope accounting and evidence.

## Evaluated document cases

Each row is a manual application of the current normative text. Safe errors below
name the selected profile mapping. A current host denial precedes private status
disclosure; no case authorizes stale-on-error, arbitrary links or a hidden retry.

| ID | Input or failure | Required observation |
|---|---|---|
| A01 | Empty allowlist, known host denial or caller-supplied scope/origin field | Zero provider requests; forbidden or invalid_input as applicable. A document coordinate is not authority. |
| A02 | Unseen page ID in an allowed space versus outside/missing/inaccessible | One fixed ID+space+current+page storage collection request. Valid exact allowed body succeeds; exhausted empty scoped collection yields the same not_found for all absence causes. |
| A03 | Wrong/duplicate ID, noncurrent status, forbidden space, multiple results or unexplained continuation on one-ID request | unavailable without body or foreign metadata disclosure. No unscoped /pages/{id} fallback. |
| A04 | Page moves between metadata and body lookup or after a provider observation | Body request repeats current fixed space predicate; release requires its own exact allowed response. A later move is not prevented or misrepresented as an atomic host/provider transaction. |
| A05 | Jira numeric ID versus old/lowercase/moved key | Numeric ID goes directly to fixed reconciled scoped search. Key first gets at most one fields=project metadata lookup, then the same search by returned canonical ID. No key-prefix grant. |
| A06 | Key metadata resolves outside allowed project, misses, or lacks canonical correlation | Outside/missing/inaccessible returns safe not_found; malformed identity/project is unavailable. No body request or foreign project disclosure. |
| A07 | Jira issue moves after key lookup | Scoped search includes canonical id, allowed project IDs, reconcileIssues=[id], fields=*all and maxResults=1. Accept only returned allowed project correlation; eventual plain search is not substituted. |
| A08 | Scoped PageBulk subtype absent/null, page, live, number, or malformed body | Absent/null can use mandatory subtype=page filter; page agrees. Live/non-string conflicts refuse. Missing/wrong ID/status/space/storage still refuses regardless of subtype. |
| A09 | Scope, connection, credential generation, parent route, projection, max_body_bytes or policy changes before cache reuse | Context mismatch defeats the entry; current admission remains mandatory. A token/version/TTL cannot override it. |
| A10 | Cache body version 7; fresh scoped metadata is same ID/version in an allowed space | May reuse full stored body. Item metadata comes from fresh scoped observation; body time/revision/count remain original. No expiry renewal. |
| A11 | Cache body moves into another allowed space without changed body version | Fresh allowed-space metadata may correlate the stored body, but the item exposes the fresh space ID; it cannot present cached old membership as current. |
| A12 | Null/missing/different version, forbidden membership, outage, expiry or lost cache | Never stale success. Use only the permitted fresh scoped body read within two calls, or safe refusal; loss is a miss and does not invent a new observation. Jira updated never enables this cache. |
| A13 | body.storage missing/value non-string, ADF-only body or conflicting nested representation marker | unavailable. A present marker must be storage; absent marker may rely on requested body-format plus returned storage key. Metadata-only revalidation consumes no incidental body. |
| A14 | Jira fields contains custom keys, nested objects/arrays/null and integer 9007199254740993 | Preserve every provider-visible native value without binary-float conversion. Missing/non-object fields is unavailable, not content:null. |
| A15 | Jira complete fields object fits both limits versus fails either | Return the complete native object or content:null with explicit applicable causes; never prune fields, manufacture {}, stringify or return a JSON prefix. |
| A16 | Source fields token is {"customfield_1":9007199254740993,"n":null} | Exact original token span is 43 UTF-8 bytes. body limit 42 causes whole-object omission; 43 permits the full object if result space fits. Trailing source-envelope bytes are not part of this token count. |
| A17 | Storage value A€😀, body limit 4 | Exact decoded count is 8 bytes; prefix A€ has 4 bytes and body_bytes. No split scalar or claim of well-formed truncated XHTML. |
| A18 | Native representation exceeds max_body_bytes and its reserved serialized-result allocation | Both body_bytes,result_bytes appear in that order, even if either alone would force shortening/omission. Truncated iff causes is nonempty. |
| A19 | Decoded storage size fits body limit but JSON escaping exceeds remaining result allocation | result_bytes; longest scalar prefix within the reserved allocation. Exact original decoded byte count remains unchanged. |
| A20 | 300 KiB bounded storage body at default 256 KiB, with sufficient result space | body.bytes=307200; longest <=262144-byte scalar prefix; body_bytes. Full source acquisition precedes the exact count. |
| A21 | Source response cutoff, malformed JSON/UTF-8, duplicate keys or rounded unsupported native number | unavailable with no fabricated full count or successful source prefix. Content-Length is not body.bytes. |
| A22 | Tiny selected body inside >source-ceiling metadata/links/whitespace/unrequested expansion | unavailable: entire encoded and decoded provider envelopes count, including filtered/unused fields. Metadata-only responses retain the smaller 512 KiB ceiling. |
| A23 | Individually valid provider pages together exceed 8 MiB, five calls or original shared deadline | Whole invocation fails; byte/call counters do not reset per item/page. timeout is distinct from malformed/oversized unavailable. |
| A24 | Required item/identity/cursor/envelope metadata alone exceeds 4 MiB | Refuse without cutting identity/protocol fields or advancing the cursor; body shortening cannot repair required-metadata overflow. |
| A25 | CQL page is empty or its candidates include currently unavailable/out-of-scope bodies | Empty candidates make zero body calls, never an unconstrained empty-ID query. Otherwise v2 uses the fixed candidate count as limit and comma-separated canonical ID arrays; current scoped enumeration filters absent bodies without per-item/unscoped lookup or metadata-only downgrade. |
| A26 | Current body response order differs from CQL candidates | Validate all requested IDs/current membership, finish enumeration, then reorder accepted bodies by candidate order. Item title/body/version share their v2 observation. |
| A27 | v2 body enumeration still has next after four body calls, or fails a page | Entire public source page is unavailable with no public cursor advancement; caller may start a separately admitted smaller search. |
| A28 | Many individually bounded bodies exceed total public space | Reserve all required metadata/cause/cursor fields first, then allocate storage prefixes in stable item order under 4 MiB. No fetched-but-unemitted item is skipped while advancing source position. |
| A29 | Empty filtered CQL page with next, versus exhausted valid CQL page | First remains complete:false with a cursor; second is complete:true. Body shortening is independent of list exhaustion. |
| A30 | Known revocation after lookup/body acquisition but before release | Current disclosure fence refuses cached or fresh content. No previous success or provider observation grants future access. |
| A31 | Read current ESS and OperationDeclaration mapping claims | Native representation/scope/CQL selection are private typed values; no existing representations field, public codec, parser, cache or provider entity is alleged. |
| A32 | Read configuration examples/defaults | IDs/keys/writes and Jira adapter page_limit=100 are illustrative. Document max_body_bytes default is 262144; CQL request default/max remain 25/100. |

The two literal byte lengths in A16/A17 were separately checked using
`printf '%s' <literal> | wc -c` (43 and 8; exit 0). This checks literal byte
length only, not lossless JSON decoding or projection behavior.

## Evaluated CQL grammar and continuation cases

| ID | Input or failure | Required observation |
|---|---|---|
| Q01 | label=ops OR title~error with a trailing multi-key ORDER BY | Parse the whole predicate, preserve its byte span inside one trusted conjunction, and append the unchanged ordering span outside. |
| Q02 | NOT/grouping, a quoted ORDER BY, nested native functions or escaped exact phrase | Respect structural tokens; preserve admitted original bytes and provider-native field/function semantics. Raw quote escapes are not JSON/URL escapes. |
| Q03 | Caller predicate contains forbidden space OR non-page type; configured key has quote/backslash | Fixed receiver type/space clauses remain outside the grouped caller predicate. Quote the full configured key exactly or refuse an unverified literal binding. Body filtering still uses stable allowed IDs. |
| Q04 | Extra parenthesis, second ORDER BY, comment, statement, grammar/node/depth/request-target overflow | invalid_input before metadata dispatch. Unknown structurally valid field/function may fail safely at provider metadata validation, without a body request. |
| Q05 | Provider link changes CQL/filter/limit/origin/path, contains duplicate keys, fragment, userinfo or conflicting Link/_links positions | unavailable; only one validated decoded cursor coordinate can be re-encoded into the fixed endpoint request. No returned-link authority. |
| Q06 | CQL next equals current/ancestor; v2 body next repeats its own position | unavailable, not progress. Search and body-enumeration lineages are distinct; private v2 cursors never become public CQL tokens. |
| Q07 | Replay cursor P after it produced child Q; replay now produces R | P and its ancestor set remain immutable; Q and R are siblings. Replayed provider results may change, but neither child becomes P's ancestor. |
| Q08 | Repeated replay attempts create many different next positions | All branches share one 64 KiB/1,024-state chain budget and original expiry. Same child may be reused; capacity failure returns unavailable without consuming P. Whole-chain eviction makes all its tokens stale. |
| Q09 | Depth 1,024 source page is exhausted versus still has next | Exhausted page may succeed. Further next refuses unavailable; state/byte capacity can refuse earlier. No false terminal page or renewed TTL. |
| Q10 | Body enumeration or current admission fails after provisional child reservation | Do not publish/advance a new public cursor; release provisional state and preserve the existing admitted cursor for a later separate bounded replay. |

There are 32 manual document cases and 10 manual CQL cases. They are authored
scenario counts, not test-runner counts. Independent final review and later
runtime/provider/transport/codec/cache conformance remain distinct gates.
