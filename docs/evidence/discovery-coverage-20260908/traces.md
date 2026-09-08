# Declared discovery and composition traces

These are textual conformance expectations audited against the normative owners, **not executed runtime traces**. The ESS schema checks in type-results.json check values only. No provider, metadata transaction, route validator, timer or composition executable was run for these expectations.

Owners: [resource discovery](../../../contracts/discovery/resources/v1alpha1/semantics.md), [route continuity](../../../contracts/discovery/mediated_route/v1alpha1/semantics.md#41-observation-continuity-and-route-revalidation), [composition](../../../contracts/discovery/composition.md), [exact authorization](../../../contracts/auth/evidence/v1alpha1/semantics.md#44-exact-authorization-targets-and-fan-out-budget-f08).

| ID | Input/history and decisive fact | Required result |
|---|---|---|
| DC-T01 | Previously complete a+b; b now denied, a contains a current positive | No b provider read; positive a observed, nonterminal b history stale; both coverage projections and root incomplete |
| DC-T02 | Same selection as T01; a completely exhausted and its old object absent | Withdraw absent a only; b remains unknown history, overall incomplete |
| DC-T03 | All namespaces allowed but b provider continuation unresolved | Authorization complete does not make collection complete or prove b absence |
| DC-T04 | Complete comparable empty b collection | Previously known absent b incarnation withdrawn |
| DC-T05 | Complete a-only scan after narrowing a+b configuration | Different selection/scope; no disappearance conclusion about old b |
| DC-T06 | Same label but changed source/profile/projection/disclosure revision | No cross-scope negative evidence or cursor reuse |
| DC-T07 | Candidate allowlist removes an object | Candidate immediately ineligible under current policy; exclusion does not establish provider deletion |
| DC-T08 | 500 examined objects, provider exhaustion positively proved at the boundary | May be complete; cap equality alone does not force incompleteness |
| DC-T09 | 500 examined objects, more objects or unknown continuation | Capped partial view; no absence for that partition |
| DC-T10 | Small/empty provider page without reviewed exhaustion predicate | No complete collection claim |
| DC-T11 | Malformed later provider page after trustworthy positives | Discard untrusted page; bounded partial positives plus classified history if publication remains admitted |
| DC-T12 | Required SSAR unknown or timed out before resource reads | unavailable and zero resource requests, preserving F08 preflight refusal |
| DC-T13 | Every exact target definitely denied | forbidden and zero resource requests; no successful empty page |
| DC-T14 | 65 distinct authorization targets or partitions | Refuse before provider work under the selected limits; target overflow invalid_input |
| DC-T15 | Explicit admitted empty configured namespace list for Service discovery | One exact list/core/v1/services/empty-namespace/empty-name/empty-subresource SSAR target; one cluster-wide partition, all scan ceilings preserved |
| DC-T16 | Missing/invalid configuration, denied namespace or caller empty selector | Cannot activate all-namespaces mode or fall back to it |
| DC-T17 | Cached namespace a allow offered for all-namespaces list, or reverse | No exact-target evidence reuse; require the selected exact check under current binding |
| DC-T18 | Switch between finite namespaces and all-namespaces mode | New selection/scope, fresh admission/evidence; no cross-mode absence conclusion |
| DC-T19 | Provider deadline exhausted at 15 s, trustworthy positives available, outer 20 s deadline live | Stop provider sends/cancel outstanding work; bounded capped publication may finish within original outer deadline |
| DC-T20 | Outer deadline expires before atomic publication | Refuse; this attempt cannot publish a new view or reset either deadline |
| DC-T21 | Possible commit followed by lost acknowledgement/expired response deadline | No claimed success or invented generation; observe the same attempt at the metadata owner under fresh admission, no automatic rescan |
| DC-T22 | Two scans captured one predecessor; second publishes first | First completion loses CAS and is discarded; no relabeling old content with a newer generation |
| DC-T23 | Source/configuration/credential/policy fence changes during scan | Stale attempt cannot publish or restore route authority |
| DC-T24 | Crash loses verifiable view/private-index continuity | Invalidate old handles, establish non-reusable epoch, do not reconstruct private targets from public rows |
| DC-T25 | Page size 1 from acknowledged complete three-row view | Same generation/coverage across immutable public pages; root complete only on final page |
| DC-T26 | Final row page of partial collection | next_cursor null and complete false; no cursor promising new provider work |
| DC-T27 | Current access denied while presenting an old cursor | Current refusal precedes stale_cursor; cursor retains no grant |
| DC-T28 | New publication in same scope or changed coverage/authorization | Old cursor invalidated; no silent retarget, rescan or deadline reset on continuation |
| DC-T29 | Historical row expires before the next page | Retire the immutable view/cursors; require a fresh admitted collection, never silently filter or extend historical rows |
| DC-T30 | Repeated partial scans exceed 500 rows or history reaches 600 s | Bounded retention/eviction with truncation; no renewal from failed scan, no deletion claim or child-pinned unbounded history |
| DC-T31 | Confirmed withdrawal, then denied/capped/failed partition | Retained old incarnation remains withdrawn, never stale |
| DC-T32 | Same provider identity reappears after T31, including in a partial scan | New incarnation/id and new child if materialized; old terminal row may remain until eviction |
| DC-T33 | Lost incarnation continuity after eviction/restart, same UID seen again | Mint new id; cannot reuse old id solely from UID/type |
| DC-T34 | Title-only rename with proved equal private identity/type/fixed target | Preserve observation id and fixed child target; new view still requires current route evidence |
| DC-T35 | Same UID/type, changed private port/destination/tenant/auth-placement | New observation/child binding; old child cannot silently repoint; old row withdrawn only with complete comparable absence evidence, otherwise stale/ineligible |
| DC-T36 | New observation generation, same sealed target, explicit validation action with current exact permission and independent grants | CAS may advance private route evidence for same connection; invalidate old child verification/pending admissions |
| DC-T37 | Stale retained observation and a successful old proxy call | Cannot validate/materialize from history or infer private target equality |
| DC-T38 | Services list denied, exact services/proxy allowed | List denial is not proxy denial, but missing fresh observation/equality still makes route unavailable |
| DC-T39 | Exact services/proxy denied or parent/current child revoked | route_refused or owning revocation refusal; no direct fallback or ordinary-call invented validation probes |
| DC-T40 | Same-target route validation loses expected revision/parent-generation CAS or acknowledgement | No forward on ambiguous/stale route evidence; no silent retarget or revived child |
| DC-T41 | Authored executable links concrete parent and child and injects SDK ports into generic host | Composition executable owns construction; host and sibling adapters gain no concrete cross-dependency |
| DC-T42 | Same machine/pod/deployment but parent and child in separate OS processes | Private mediated port unavailable; configuration cannot claim same-process profile |
| DC-T43 | Candidate names an uninstalled implementation or incompatible/missing port/profile | Explicit materialization refusal; discovery cannot download/load/start an adapter |
| DC-T44 | Independent standalone direct parent/child adapters | Remain usable through their own supported direct profiles; composition does not become a mandatory concrete host dependency |
| DC-T45 | Nested mediated parent, port/process loss, or arbitrary target override | Refuse unsupported route; restart requires fresh verified live binding, no generic wire proxy |
| DC-T46 | Remote business operation targets an already selected supported service/federation endpoint | Existing admitted operation boundary may be used; private provider ports and credentials are not exported as a transport |

The two source findings closed by this packet are F13 and E03. E07/E14/E32 recognition, operation naming and physical cluster identity remain owned by discovery-profiles; persistence entities/relations and cross-owner storage bindings remain unimplemented and explicitly unmodeled.
