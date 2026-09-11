---
format: aep.planning-md/1
id: verification-report:findings-discovery-b-initial-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:discovery-b-initial-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 617511ac6a39130f0a12d2b2f212f8ecb070007f4578b32ce37c9c652216afce
relations:
- verifies: review-result:discovery-b-initial-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:discovery-b-initial-20260908

This supplements [the immutable original](../review-result/discovery-b-initial-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

No verdict is offered on discovery-profiles E07/E14/E32. They remain explicitly outside this cluster until this prerequisite is complete. No external publication or implementation support is inferred.

## Transcription method

9 findings remain in the scope of this report's final stated conclusion.
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
    "file": ".engineering/planning/review-result/discovery-b-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-B-01 — withdrawal has no exact coverage/equality scope (P2; F13)\n\nEvidence: resource discovery semantics lines66–68,79,81–82 still speak of one set/generation per source connection and a complete replacing generation. F08 evidence §4.4 authorizes exact namespace/resource targets and forbids withdrawing from incomplete scans. A complete read of namespace a must not withdraw b, and a result filtered by a caller's disclosure or a changed allowlist must not prove provider disappearance.\n\nRequired correction: select the immutable represented scope of an authoritative collection: qualified source/binding, discovery declaration/profile revision, configured selectors and exact normalized target set, visibility/recognition/projection inputs and applicable authority/configuration revision. Keep private equality inputs private. Withdrawal may compare only equivalent represented scope and a definitely complete successful observation of that scope. Narrower/different scope, source replacement, filtered disclosure and configuration exclusion need their own stale/invalidated treatment, not fabricated absent-provider facts. This does not settle E07 operation naming, E14 recognition or E32 cluster identity; retain those as separate prerequisites."
  },
  {
    "file": ".engineering/planning/review-result/discovery-b-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-B-02 — cap, incomplete scan and retained-history behavior is unselected (P2; F13)\n\nEvidence: resource discovery lines68,79,88 promise both no withdrawal from incomplete results and atomic set replacement with a 500-object cap. No rule says whether a capped/failed refresh preserves an older complete set, publishes partial additions, or evicts stale observations. Repeated bounded partial scans can also accumulate unbounded retained history unless the collection/storage bounds cover it. F08's denied subset may return allowed data, while unknown permission refuses before any resource reads.\n\nRequired correction: choose one conservative partial-publication/retention policy. Separate latest refresh outcome from last complete authoritative evidence; label retained observations as historical/unknown/stale rather than freshly observed or withdrawn. Empty allowed collection, all-denied forbidden, denied subset, cap exhaustion, malformed/failed provider page, timeout and metadata publication failure need distinct outcomes. No truncated scan or omission caused by pagination, filtering or budget can produce a disappearance fact. A strict retained-record/storage bound must fail or evict only under a declared policy that does not turn eviction into confirmed withdrawal. Permission coverage remains separate from provider collection coverage."
  },
  {
    "file": ".engineering/planning/review-result/discovery-b-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-B-03 — atomic replacement lacks stale-publisher and binding fences (P2; F13)\n\nEvidence: resource discovery line79 says readers never see a mix, but no publication ordering is selected. Line111 gives adapter-transient state backed by a host metadata store without defining who commits it. Existing connection publication/revocation ordering in auth.connection §4 cannot safely be bypassed by a discovery refresh. An older scan can finish after a newer one or after source configuration/authority changes and otherwise overwrite newer evidence or withdraw a resource it never saw.\n\nRequired correction: identify one logical host collection/publication owner and an atomic publication decision over the expected source/scope/configuration and predecessor/current collection revision. Pin scan inputs; reject stale or conflicting publishers rather than assigning authority by completion time. Generation monotonicity must reflect committed publication, not wall clock or provider resourceVersion. Unknown commit grants no second publication or fresh route admission from an assumed result; read authoritative state or fail closed. Restart must recover verifiable identity/revision continuity or invalidate old snapshots/cursors/routes, never reuse generation numbers as proof. Durable entities/backend mechanics remain an explicit later binding gate."
  },
  {
    "file": ".engineering/planning/review-result/discovery-b-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-B-04 — provider scans, public paging and complete are conflated (P2; F13)\n\nEvidence: resource discovery lines31–56 exposes paged observations and a single complete/generation, line68 rejects old generations, and line98 tests that rule. Current Kubernetes list implementation lines93–145 binds a provider continuation to request/config context and uses absence of continue to set Page.complete; SDK lines156–210 supplies a 300-second opaque cursor with restart-invalidated keys. Neither current behavior is an atomic discovery-snapshot implementation.\n\nRequired correction: distinguish provider scan continuation from paging an immutable published discovery view. Establish when provider completeness is actually known (all admitted targets, all provider pages, no cap/truncation/failure and any required coherent-list predicate). Empty items alone cannot prove exhaustion. Public pages must reference one immutable collection/coverage view and deterministic ordering; a cursor binds scope, projection/configuration/current authorization and that view, with finite expiry. Choose whether a newer publication invalidates an old cursor or retains its exact immutable view; never mix pages from generations. Current denial and changed coverage must still refuse/stale the continuation under F08. Define page exhaustion separately from authoritative collection completeness; a final page of an incomplete snapshot must remain incomplete. New fields require a supporting strict reader; existing Page remains unchanged."
  },
  {
    "file": ".engineering/planning/review-result/discovery-b-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-B-05 — route generation pinning has no safe same-target revalidation path (P2; F13)\n\nEvidence: mediated route lines40,55,60 binds an observation id plus generation at materialization; lines63–65 also require current parent generation/route observation; resources line66 increments the whole source set. A harmless next complete refresh can therefore invalidate every child forever, or tempt implementations to silently replace the route pin. Scenario84 only describes withdrawal and re-materialization. Auth profile §4.2 prohibits changing fixed target/parent/mode/owner under the same connection ref.\n\nRequired correction: separate committed collection revision, a resource's stable identity and route-relevant binding revision, and private parent credential generation. Define explicit current same-target revalidation/admission after a new collection or parent generation; successful revalidation cannot change the fixed identity/route or inherit old permission. Define what retained stale/unknown observations permit: at minimum they are no materialization or dispatch authority. An exact current services/proxy denial remains route_refused regardless of retained discovery history; source list denial is not itself an interchangeable proxy denial. Missing fresh route evidence yields the appropriate unavailable/degraded result; no direct fallback, new target selection or hidden unlimited probe follows. Revalidation provider reads/checks must have a declared owner and share the child's remaining F08 deadlines/budget where applicable."
  },
  {
    "file": ".engineering/planning/review-result/discovery-b-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-B-06 — stable observation identity and invalidation reasons contradict adapter wording (P2; F13)\n\nEvidence: resource discovery line78 keeps id stable for unchanged private UID, but Grafana adapter line117 degrades on a renamed source, and resource discovery's old-evidence line26 similarly lists rename as a reason. Resource scenario95 combines rename and type change, so it does not resolve rename alone. Candidate allowlist/mapping removal, provider object disappearance, changed type and same-name replacement are also currently bundled under withdrawal/degradation.\n\nRequired correction: choose stable identity separately from mutable display metadata and route-relevant target facts. State whether a title-only rename keeps the same observation and route binding; if another field changes routing identity, name that predicate without inventing vendor behavior. Same Kubernetes namespace/name with a new UID, changed provider type/fixed target or parent/source must not silently revive/repoint an old child. Distinguish confirmed absent, stale/unknown observation, locally excluded candidate, and invalid route binding. Define whether reappearance of the exact old identity can revalidate the observation and which terminal connection/retirement facts cannot be resurrected. These are coverage/identity continuity rules; exact recognition and cluster identity remain E07/E14/E32 work."
  },
  {
    "file": ".engineering/planning/review-result/discovery-b-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-B-07 — concrete wiring is assigned to the generic server (P2; E03)\n\nEvidence: mediated route §8 line101 assigns composition loading to connectors-host/src/server.rs. Design lines245–253 expressly forbid host-to-concrete-adapter dependencies. Actual server.rs lines14,31–47 accepts an injected Arc<dyn Adapter>, and connectors-host/Cargo.toml has no concrete adapter dependencies. Kubernetes main.rs constructs its concrete library and injects ScopedHttp before calling the server. Grafana adapter lines9–11,115 require colocated parent/child construction without naming who links them.\n\nRequired correction: name a deployment/composition executable or thin entrypoint above the generic host and selected adapter libraries as the concrete linking/wiring owner. It may instantiate both adapters and inject parent route ports/child capabilities into generic host orchestration. Parent code owns provider-specific proxy construction and binding interpretation; host owns generic policy/materialization/metadata/fences; SDK owns provider-neutral ports. No sibling adapter dependency or provider switch in generic host/server. Explain this dependency direction in a small diagram and update the misleading obligation row. No new executable, registry, loader or wire proxy is required in this semantic story."
  },
  {
    "file": ".engineering/planning/review-result/discovery-b-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-B-08 — colocation and installed support do not yet imply a mediated capability (P2; E03)\n\nEvidence: design lines797–799 recommends independent executables; mediated route lines43,67,92 permits a private same-composition port and excludes provider-traffic federation; Grafana adapter line115 says the host starts or binds the child. Current SDK lines55–57 only has injected AuthenticatedHttp.get; it has no MediatedRoute port or generic multicomponent server registry. Merely running independent services in the same machine/network cannot provide the private in-process port.\n\nRequired correction: explicitly define the selected mediated placement as one statically composed process/boundary with the required concrete parent and child support installed and their ports wired. Separate executable deployment remains valid for standalone/direct adapters, but cross-process mediation requires an independently specified transport and is unavailable here. Materialization/configuration checks selected target implementation, route profile, one-hop placement, scope, and parent/child independent admission before publishing a usable child. Unsupported placement/port/provider mapping fails explicitly; an observation never downloads/executes a plugin, authorizes a broad parent operation, creates a generic proxy or grants direct egress. Starting a configured implementation is composition-owned and separately admitted, not a side effect of listing. Preserve independent library builds and fake-port conformance as later verification obligations."
  },
  {
    "file": ".engineering/planning/review-result/discovery-b-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-B-09 — proposed persistent observations/routes are presented as modeled relations (P2; shared documentation precision)\n\nEvidence: resource discovery §9 lines117–119 claims ResourceObservation lifecycle/relations and says multiplicity is recorded as a Connection.parent reverse cardinality; mediated route §9 line108 claims RouteBinding lifecycle/relations. Frozen ESS declarations/system contain no ResourceObservation, RouteBinding or persistent Connection owner. The same resource table at line111 also mixes adapter-transient and host-persistent ownership. Both current stories expressly state proposed entities are not modeled.\n\nRequired correction: align these tables with the selected semantic ownership and actual ESS state. Label proposed persistent entities/relations/cardinality/retention as unmodeled obligations under their existing persistence owner; do not imply an implemented reverse relation. If this cluster introduces useful typed coverage/composition values, model only settled shapes and mark publication, authority, provider completeness, temporal/retention and cross-entity predicates as UNMAPPED. No invented persistence implementation or broad entity decomposition is required to close F13/E03."
  }
]
```

