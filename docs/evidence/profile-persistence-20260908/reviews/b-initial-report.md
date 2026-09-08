# Independent reviewer B — discovery profiles / persistence ownership, initial

Verdict: **needs revision**. Ten P2 findings, each with one owning story. Baseline 175053fd2c951dc9564ff587a636775ab8027d2e. The 49 frozen inputs include 47 current repository files and two old design files pinned to 81459ac42ddd518d3942f4b079841e9e0ed6efc8; exact hashes are in source-hashes.json. No source/planning/runtime changes, runtime test suite, external integration or other reviewer output was used. The detailed baseline port/state/atomicity inventory is inventory.md; actual ESS entities are machine-listed in ess-entity-inventory.json.

## PP-B-01 — operation/profile/source binding is still deferred (P2)

Sole owner: **story:contracts-discovery-profiles**.

Resource discovery31 now forbids caller profile selection but explicitly defers exact operation binding. Kubernetes adapter56 chooses services.observe; Grafana's operation map chooses datasources.observe, while the family still uses conceptual resources.observe. A concrete declaration needs one unambiguous source-qualified operation, fixed contract/profile, configured source connection/scope and current description revision. The shape must reject a supplied profile/declaration/source override, including attempts to route through a generic profile selector, rather than ignore it. Selected input selectors can only narrow an admitted configured selection where explicitly declared; F08's all-namespaces mode cannot be activated by request emptiness. Update the operation maps, example and selected closed input together; retain old-reader refusal and no implicit aggregate namespace. Tests: wrong profile, source/connection substitution, stale descriptor, legitimate fixed-profile pages.

## PP-B-02 — exact Argo recognition and non-route disposition are absent (P2)

Sole owner: **story:contracts-discovery-profiles**.

Current Kubernetes config90 merely names argocd and §7 says one observation per recognized Service; resource discovery's open decision names old markers plus argocd without an exact predicate. Pinned old design10:179–186 selects an exact-identity arm over Service name **OR** app.kubernetes.io/name label so Helm-prefixed API Services remain recognized; argocd-server-metrics, argocd-repo-server and argocd-redis do not match their ordinary exact name/label values. Do not replace that OR with AND, suffix/substring matching, a broad app label or a name-only rule. Specify case-sensitive exact token argocd-server and deterministic precedence/ambiguous recognition behavior. This remains inferred observation evidence, not trustworthy provider identity or authority. Preserve old design188–204: no materialized Kubernetes-mediated Argo/Grafana child from discovery, no inherited downstream account, no new target grant/callable Argo adapter. A declared private endpoint/port interpretation must be pinned or explicitly deferred, never dialed for recognition. Cover exact-name/no-label, Helm-name/exact-label, ordinary metrics/repo/redis and partial lookalikes.

## PP-B-03 — configured source identity is not physical cluster identity (P2)

Sole owner: **story:contracts-discovery-profiles**.

Resource discovery97 leaves E32 open and already distinguishes configured provider authority from physical cluster identity; Kubernetes actor/config examples do not finish that boundary. Select the qualified configured instance/connection plus admitted parsed API origin and trust/configuration revision as the actual source boundary; public source identity must be safe and distinct for two configured clusters. A kubeconfig display/context name, same URL text, certificate filename, namespace or authenticated user is not proof of an unchanged physical cluster. Origin/trust/target replacement must invalidate old evidence/scope/routes/cursors according to existing fixed-target rules, not silently repurpose a connection. State the absence of a physical cluster-UID validator and its consequences explicitly; do not invent a hidden admin/identity probe or claim detection of an indistinguishable cluster replacement. Demonstrate different configured sources, same display name, endpoint reconfiguration and the explicit unverified-physical-identity limit.

## PP-B-04 — no single complete ownership inventory exists (P2)

Sole owner: **story:contracts-persistence-ownership**.

Design959–961 states the principle but not the inspectable mapping. Family obligations call each store a new host module independently. inventory.md identifies 18 current logical port responsibilities and separately deferred design obligations. Consolidate them in the requested design section with one singular named owner per persistent decision, state/key scope, durability expectation, atomic operation, failure observation and family link. Shared infrastructure may back several ports; a universal keyed-byte store or provider-owned database does not follow. Include delivery nonces, independent audit, session redemption/lease/terminal accounting and optional evidence/cursor caches, which the story's short checklist can otherwise miss.

## PP-B-05 — required multi-record atomic decisions need a named coordinator boundary (P2)

Sole owner: **story:contracts-persistence-ownership**.

Acquisition103–110 requires source exclusion/consumption, active-reference/generation publication, refresh state and admission invalidation under one authority; evidence98 requires final dispatch ordered against that publication/revocation. Operations97/144 requires keyed reservation plus Prepared attempt atomically, and169 requires terminal settlement/retention ordering. Discovery/resources129 and mediated route77 require view/private-index or route-revision publication with current dependency fences. Naming separate stores cannot be mistaken for independent sequential writes that collectively provide these atomic decisions. Assign exactly one coordinating port owner per operation and state the local metadata binding must fulfill the required coupled comparison/update, or refuse the profile. Do not add an unsupported cross-service transaction or make custody's three-argument publish_active CAS stand in for refresh coordination.

## PP-B-06 — failure handoffs must keep independent acknowledgements distinct (P2)

Sole owner: **story:contracts-persistence-ownership**.

Custody58–61, operations97–103/128, service compatibility76–78 and delegation133/151–167 select different irreversible boundaries. The inventory must preserve: durable secret write is not metadata publication; possible token exchange is not registered recoverable response; approval/event spend is not dispatch; nonce consumption is not authenticated entry until post-ack checks; admission audit is not outcome audit; gateway success/audit is not leaf business success. Assign the recovery observer and conservative action for each unknown acknowledgement. No owner reread, retry, restart or timeout may reacquire a one-shot send/spend. The leaf alone owns business attempt/key/approval; gateways retain independent transport/admission/audit. Protected acquisitions keep their single coordinator; sessions lose continuity rather than attach from a record.

## PP-B-07 — retention/cleanup/restore can erase different kinds of authority (P2)

Sole owner: **story:contracts-persistence-ownership**.

There is no consolidated retention matrix for custody62/74, operations169–171, delegation151–155, discovery/resources143–150 and session restart/cutoff. Preserve their distinct clocks and terminal rules: 24h known mutation replay from durable settlement only; no automatic Pending/Quarantined or spent approval expiry; nonce retirement only at trusted lower>=exp; private refresh consumption survives blob GC; bounded discovery history/epoch retirement is not withdrawal; loss of a cache/pin/cursor grants nothing. Capacity refuses new authority instead of evicting live exclusion/tombstone records. Backup/failover under the same stable leaf identity must not restore divergent/empty uniqueness state. Inventory each owner and handoff without inventing reconciliation, retention numbers for unselected domains or remote transactions.

## PP-B-08 — modeled entities and derived status are misstated (P2)

Sole owner: **story:contracts-persistence-ownership**.

Custody109 claims a CredentialSet written→active→superseded→deleted entity/relation that is absent from ESS. Connection153 still asks for update status even though §4.1/§9 explicitly defines readiness as a reduction of current facts, not an independently authoritative lifecycle. Exact declared entities are in ess-entity-inventory.json: eleven, including a deliberately transient DispatchAdmission and modeled Session without runtime continuity. Correct the tables and inventory to distinguish declared entities, values, conceptual persistent records and unmodeled owners. Persistent Connection/Acquisition/AuthProfile/custody versions/audit/discovery/mediated-route/Composition are not implemented or declared; idempotency.RouteBinding is unrelated gateway fingerprint data. Updating underlying facts or projecting a status observation is not permission to set ready independently.

## PP-B-09 — private publication fence versus semantic revision needs explicit ownership (P2)

Sole owner: **story:contracts-persistence-ownership**.

Acquisition109 advances a binding revision on successful same-identity refresh; operations150 says a same-identity credential refresh is not a semantic revision and excludes rotating material from the keyed fingerprint. Connection131 permits unchanged configuration revision while the private material generation changes. The inventory must distinguish the coordinator's monotonic CAS/publication fence from the effect-relevant connection/configuration revision used in descriptors, approval subjects and F02 fingerprints. Name who advances each. Otherwise a faithful refresh implementation can spuriously conflict a legitimate existing key or a semantic target change can wrongly retain approval/fingerprint identity. Preserve private-generation invalidation and current admission without making token rotation a new namespace or semantic operation.

## PP-B-10 — custody diagnostics contradict the established private-reference boundary (P2)

Sole owner: **story:contracts-persistence-ownership**.

Custody53 says invariant failures are logged without the reference, but65 permits scope and version ids in read errors/logs/metrics. Auth connection §4 forbids secret references in errors/logs, evidence/capability keep private snapshots/generations out of diagnostics, and management43 excludes credential references from ordinary outputs/logs/audit. The ownership inventory must not export private custody handles as a generic diagnostic key. Align custody65 with safe public correlation, and keep any needed private operational correlation inside its explicitly protected owner boundary; do not introduce a second public identifier path through metrics/errors. Material bytes and actionable callback/registration values remain excluded too.

## Verification required for this semantic checkpoint

Use a textual owner/atomicity/failure/retention coverage audit plus the discovery cases above. If adding settled ESS values, report shape checks and explicitly unmodeled predicates separately. No runtime/backend implementation, new CRUD/issuer/event protocol, physical-cluster validator or migration/rollout is requested by these findings. The existing family owners continue to govern their semantics; this story consolidates rather than weakens them.
