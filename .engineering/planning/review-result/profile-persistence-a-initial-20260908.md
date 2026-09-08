---
format: aep.planning-md/1
id: review-result:profile-persistence-a-initial-20260908
kind: review-result
status: active
title: Discovery profiles and persistence reviewer A initial
relations:
- reviews: story:contracts-discovery-profiles
- reviews: story:contracts-persistence-ownership
revision: 1
---
# Independent initial review A — discovery profiles and persistence ownership

Verdict: **needs revision** for the two existing draft stories. **8 P2 findings; 0 P0/P1.** These identify missing selections/consolidation in the baseline specification, not defects in an advertised runtime. No implementation is requested by this review.

Reviewed baseline: `175053fd2c951dc9564ff587a636775ab8027d2e`. Exact baseline inputs are frozen under `sources/`, with 42 inputs in `source-hashes.json`. Two supplemental old implementation/test files are frozen in `old-source/` and `supplemental-source-hashes.json`. All old sources use commit `81459ac42ddd518d3942f4b079841e9e0ed6efc8`. Citations below name baseline repository paths and line numbers; old citations name that separately pinned repository. Working-tree edits and other reviewer outputs were excluded. No runtime tests were run.

## Findings

### PP-A-01 — P2 — Finish descriptor-fixed discovery declarations

**Sole correction owner:** `story:contracts-discovery-profiles` (E07).

`contracts/discovery/resources/v1alpha1/semantics.md:31` leaves exact operation names and profile selection deferred, while `docs/adapters/kubernetes.md:56` selects `services.observe` and `docs/adapters/grafana.md:61` selects `datasources.observe`. The input already has only limit/cursor. There is not yet one authoritative rule relating the conceptual family operation to these adapter declarations and to the resolved source.

Select the concrete declaration/profile mapping in one owner. A descriptor fixes contract/version/profile and the permitted source-selection mode; each invocation resolves exactly one admitted source under that declaration. Multiple managed source connections need ordinary admitted connection selection, not a caller-selected provider/profile or an implicit global source. Wrong or extra profile/source/namespace/target selectors must fail schema/admission rather than retarget the operation. Explain whether any literal `resources.observe` ID is advertised. Keep describe static and preserve F13 scope, page and revision rules. A source or declaration selection change must not silently reuse a cursor or generation from another scope.

### PP-A-02 — P2 — Define Argo recognition separately from callable candidates

**Sole correction owner:** `story:contracts-discovery-profiles` (E14).

The owner currently lists `argocd` as observation-only (`resources/...:196`), but Kubernetes says every recognized Service has `candidate.confidence = inferred` (`docs/adapters/kubernetes.md:97`) and gives no exact recognition predicate. A client cannot tell whether Argo gets no route candidate, an unusable advertised candidate, or no observation. Shared resources allow unknown observations with `candidate:null` (`resources/...:154`), which also needs an explicit Kubernetes disposition.

Select API recognition using the **whole exact Service name `argocd-server` OR whole exact value of `app.kubernetes.io/name` equal to `argocd-server`**. Do not turn this into name AND label, arbitrary suffix/substring matching, or a namespace/component guess. Include default-name-only and Helm-renamed-with-stable-label positives; ordinary server-metrics/repo-server/redis and Helm-suffix-only negatives. Define deterministic precedence if different recognition inputs identify competing providers. An observation marker must not manufacture an Argo adapter, callable operation, materialized connection or mediated route.

Old evidence: design10 lines 175–186; `crates/integration-kubernetes/src/local.rs:1171–1211`; `local_tests.rs:112–167`. The old implementation additionally lowercases inputs and accepts aliases `app`, `k8s-app`, `name`. The story selects a narrower stable-label rule. Explicitly disposition these aliases and normalization; do not claim the entire old recognizer is preserved unchanged. Recognition is untrusted inference, not proof that the Service speaks the API or that routing is authorized. Specify whether unknown/noncandidate Services still produce rows, and make candidate-null examples consistent across family and adapter docs.

### PP-A-03 — P2 — Specify configured source identity without claiming physical cluster proof

**Sole correction owner:** `story:contracts-discovery-profiles` (E32).

The current collection key deliberately defers physical cluster identity (`resources/...:97`) while design lists future Kubernetes cluster identity (`docs/design.md:939`). Stable source-qualified references are required (`design.md:352`) but configured instance, connection, API authority and physical cluster identity are not yet fully distinguished.

Select the configured source qualification and API-origin/authority binding, including how its interpretation participates in source/configuration revisions. Two configured instances must remain distinct even if their display names or API origins coincide. Context names, URLs, credential/account identity and Service UID do not independently prove physical cluster continuity; the same endpoint can later serve a replacement cluster and one cluster can have aliases. Either select a separately evidenced physical identity mechanism or explicitly defer it and bound claims accordingly. Configuration/authority changes must fence old scope/cursors/observations/routes; same-identity credential rotation follows the settled generation rules rather than pretending to identify a new physical cluster. Keep private API origins out of public output and perform no discovery/identity probe during describe.

### PP-A-04 — P2 — Replace the broad host-state paragraph with an inspectable port inventory

**Sole correction owner:** `story:contracts-persistence-ownership` (E28).

`docs/design.md:959–961` names example state and warns against a universal store, but does not provide one named logical port owner, state scope and operation set for each current obligation. Sources alternately call their owner a host, coordinator, metadata authority, ledger or store. The detailed inventory below shows the minimum coverage needed; some rows can share one logical authority without pretending to be one generic storage interface.

Add the narrow design persistence section required by the existing story. For each row identify exactly one authoritative owner, owned facts versus referenced facts, acknowledgement/atomicity boundary, restart guarantee, bounded retention/capacity behavior, and conservative unavailable/ambiguous handoff. Identify private adapter runtime and derived metadata explicitly, rather than treating every ESS entity or status value as durable storage. Crosslink owning contracts instead of copying a second divergent protocol.

### PP-A-05 — P2 — Name the common metadata ordering boundary without inventing cross-store transactions

**Sole correction owner:** `story:contracts-persistence-ownership` (E28).

Several settled contracts require the same facts to participate in one atomic decision: refresh authorize/publish/revoke (`auth/acquisition/...:99–116`), final credential admission (`auth/evidence/...:98`), connection revocation/publication (`auth/connection/...:85–87`), collection publication (`discovery/resources/...:129–133`) and route revalidation (`discovery/mediated_route/...:77`). Merely assigning these to separate independent CAS ports would not satisfy the specified races.

Name the logical authority/guarded operations that serialize the relevant metadata and explain which port owns the transaction. A backend may implement several narrow ports, but incompatible independent authorities cannot claim the cross-object cutoff; such a composition refuses the affected profile. Custody remains an immutable sensitive-version owner with acknowledged write before metadata publication, not part of an external provider transaction. An orphan is never active authority. Acquisition completion must preserve its sole-owner one-use/current-scope/expected-target checks and publication ordering across replicas; do not substitute a fresh process-local callback registry after restart. Final checks should retain the existing admitted policy decision boundary, not promise atomicity with every future external policy update.

### PP-A-06 — P2 — Inventory irreversible receipts and audit independently of replay storage

**Sole correction owner:** `story:contracts-persistence-ownership` (E28).

Mutation and federation already select materially different durability promises (`operations/...:97–103,144–171`; `service/delegation.md:149–167`), while audit ownership and its attempt relationship remain an explicit binding obligation (`service/compatibility.md:76`). A single undifferentiated “operation store” row would hide the important guarantees.

Keep atomic Prepared-attempt plus keyed-reservation creation and the unique dispatch gate with the executing host's attempt authority. Keep approval redemption and transport nonce consumption distinct: exact keys, executing-leaf sole ownership across all ingress/replicas, approval bound to one Prepared attempt, no gateway second spend/attempt, no automatic resend after unknown acknowledgement. State the ordered audit anchor → prepare/reserve → spend → dispatch gate → provider → outcome/audit handoffs without promising one transaction with the provider or audit store. Known live applied/refused knowledge survives final persistence failure; recovery without it remains uncertain.

Pending/Quarantined keys, consumed refresh authority and spent-approval uniqueness must survive restart/compaction. Their retention is not a generic TTL. Preserve 86,400-second known-result replay expiry and exact-generation retirement, trusted-clock nonce retirement and no automatic approval-tombstone expiry. Capacity refuses new authority before dispatch rather than evicting unresolved/spent facts. Copy/failover/restore must not activate one stable owner identity against empty or divergent uniqueness history.

### PP-A-07 — P2 — Explicitly separate durable records from nonresumable sessions and deferred event state

**Sole correction owner:** `story:contracts-persistence-ownership` (E28).

Design §20 includes subscription cursors and assignments; §§14–15 require assignment/provisioning and event persist-before-ack (`design.md:716–751`). Sessions specify single redemption, serialized terminal/lease decisions and no restart reattach (`sessions/...:83,91,97–120`). These obligations disappear if the new inventory covers only connections and mutations, or become misleading if a saved session row is described as restored authority.

Include named logical ownership for the declared event acceptance/deduplication/checkpoint, assignment/configuration and session authority/accounting boundaries, with explicit deferred profile/backend details where they are not yet selected. Do not implement or invent new event protocols. Distinguish source event intake acknowledgement from independent consumer checkpoint and business approval. For sessions preserve one-use establishment authority and first terminal accounting, but deny reconstruction of live leases, sockets, peer state or resend permission from a persisted record. Restart loses continuity; surviving gates expire under their already-issued deadlines. Session replay/reattach and durable subscription custody remain separately scoped work.

### PP-A-08 — P2 — Keep ESS identity/value evidence and retention claims truthful

**Sole correction owner:** `story:contracts-persistence-ownership` (E28), with the discovery profile selection referenced from E07/E14/E32.

The current ESS has actual CredentialGeneration/RefreshAttempt/AttemptRecord/KeyReservation and delegation receipt identities, but Connection, Acquisition, AuthProfile, custody and persistent discovery relations are deliberately unmapped. DispatchAdmission is explicitly transient (`ess/domains/credential_evidence.yaml:56–64`); discovery is values only (`ess/domains/discovery.yaml:92`); session comments reject guessed persistent ownership (`ess/domains/sessions.yaml:87–91`). Custody's §9 table still presents a conceptual CredentialSet lifecycle and Connection relationship (`auth/custody/...:109–110`).

The consolidated inventory must label these distinctions explicitly. Port responsibility does not establish an ESS `owns` edge, deletion cascade or executable database guarantee. In particular, KeyReservation references AttemptRecord so replay deletion cannot delete audit/attempt facts; F03's RouteBinding fingerprint value is not the persistent mediated route owner. New profile-selection values can be typed without inventing persistent owners solely to compile. If existing conceptual tables are linked as evidence, mark their undeclared status.

Carry existing limits into each applicable owner without shortening safety guarantees: custody's 8-version cap/24-hour retention cannot evict an active or still-required version; discovery has 500 rows/256 scopes/600-second history and immutable-view retirement; audit/history policies still require explicit binding choices. At capacity, refuse the affected new write/admission where protected live state prevents reclamation. Lost or expired caches may require revalidation or a new discovery epoch; they cannot become positive authority or absence proof. Validation should distinguish typed shapes, textual decision vectors and future real storage fault tests.

## Minimum persistence obligation inventory

The owner labels below identify logical responsibility to name in E28, not mandatory Rust trait names or backend choices. A row marked “binding/deferred” must remain an explicit requirement rather than a new advertisement claim.

| State / facts | Sole logical owner and minimum operation | Acknowledgement, recovery and boundary | Existing source |
|---|---|---|---|
| Configured instance identity, admitted configuration/assignment revisions | Host configuration/assignment authority: validate candidate and atomically activate expected revision; preserve prior on rejection | Stable identity across restart; explicit dependency/session effects; no automatic resource reprovisioning after unknown effect | design:352,507,716,959 |
| Connection identity, fixed route/mode/owner, enablement, local revocation, active binding | Host connection metadata authority: unique identity admission, expected-revision publication/replacement and monotonic revoke | Shared final-dispatch/refresh cutoff; metadata unavailable is unavailable; revoked connection never restored by restart/config reload | connection:80–95; management:9,25,33 |
| Acquisition owner/scope/profile/registration selection, optional repair target, expiry and one use | Owning host acquisition coordinator: allocate, claim/consume permitted completion once, settle safe lifecycle and publication | One logical authority across replicas; current scope/target before exchange and publication; no state cloning/reroute or exchange repeat after ambiguity; protected continuation stays private | acquisition:85–90,138–149; management:39–47 |
| Complete sensitive credential versions | Scoped custody port: immutable write/read/delete with declared durable acknowledgement | Write before metadata publication; orphan never authority; active or valid-use versions protected; outage/missing/denied distinct; no provider/refresh coordination in this store | custody:29–66,73–76 |
| Immutable credential capture association and active generation | Host credential metadata authority: allocate/recover coherent immutable capture, bind expected identity and publish only validated candidate | Material identity is not a path/version/UUID alone; duplicate rotating-material aliases cannot create independent exchange authority; snapshot recovery must be verifiable | evidence:84–104; acquisition:97 |
| Refresh reservation, fence/owner, consumed exchange authority, candidate registration/publication | Host refresh coordinator integrated with connection metadata: reserve, authorize once, preauthorization fence, attach durable response, single recovery transfer, guarded publish | Only live definitely acknowledged authorization sends; ambiguity consumes/quarantines; custody response orphan cannot stand in for committed response; revoke and publish serialize | acquisition:99–118 |
| Evidence snapshots, exact permission cache and transient admissions | Host evidence/admission owner: publish matched evidence, check exact applicable key/age, invalidate and order final gate with binding changes | Derived status is not a persisted lifecycle; cache retention never extends authority; permission checks nontransferable across generation; admission entity does not imply restartable grant | connection:93; evidence:94–118; auth_access/credential_evidence ESS |
| Attempt and keyed reservation/index | Executing mutation host's ledger: atomic prepare+reserve, Prepared→Dispatching versus Aborted, terminal settlement, exact-generation expiry | One live acknowledged dispatch winner; no recovery resend; Pending/Quarantined remain indexed; known-result expiry cannot delete independent attempt/audit history | operations:97–103,144–171 |
| Approval/event-claim single redemption | Executing leaf approval authority: exact issuer/reference consume bound to one Prepared attempt; event claim remains separately selected binding | Definite spend before gate; unavailable/ambiguous spend no gate/refund; tombstones no automatic expiry; gateway is not another spender | delegation:141,152–167,192 |
| Delivery nonce receipts | Receiving leaf delegated ingress authority: atomic exact issuer/receiver/jti consume | Definite consume then fresh final entry checks; unknown acknowledgement no entry; trusted expiry retirement only; exact uniqueness shared across replicas | delegation:133,145–155 |
| Invocation audit admission and final observation | Host audit port: append/acknowledge admission, append linked completion; separate gateway and leaf records | Unacknowledged anchor forbids dispatch; final failure preserves known effect and acknowledged anchor; attempt ledger and telemetry/event history are not substitutes | compatibility:72–78; delegation:161–181 |
| Discovery scope/epoch, immutable classified view and private observation binding index | Host discovery metadata authority: predecessor/config/credential/policy CAS publication of entire view+index, same-attempt observation, atomic view retirement | One acknowledged generation, no stale completion rebasing; unknown publish never rescan-to-certainty; recover private continuity or invalidate handles/new epoch; bounded history does not prove deletion | resources:97–150 |
| Child materialization and private mediated route evidence revision | Host connection/route metadata authority: admitted fixed-target creation and guarded same-target revalidation CAS | Checks current view/incarnation, parent generation and independent authority; old route admissions invalidated; unknown acknowledgement no forward; target/parent/mode/owner changes require new connection | mediated_route:73–79; connection:85–87 |
| Session establishment authority, terminal facts, lease decisions and accounting | Host session authority/serialized session owner; transport verifier one-use redemption, authenticated bounded leases | No restart reattach/lease reconstruction; first terminal persists in accounting, gate cutoff and local teardown distinct; live transport/media queues remain adapter/runtime state | sessions:83,91,97–120 |
| Durable provider event acceptance/deduplication and consumer checkpoints (binding/deferred) | Host event acceptance port and separately scoped subscription checkpoint owner | Persist/known duplicate before provider ack; consumer ack independent; provider source scope/retention/ordering/replay policy required; no exactly-once business execution inferred | design:726–761,990,1004 |
| Product resource provisioning/tenant assignment/history (binding/deferred) | Product/control-plane host persistence owner | Durable request/known or unknown allocation, assignment and activation prerequisites; reuse mutation outcome rules rather than repeating unknown billable allocation | design:615,716–718 |
| Datasource cursors, caches and projected federation snapshots | Owning host's selected cursor/cache or configuration projection binding; no universal requirement that every cursor be a stored row | Exact scope/query/revision/authority and expiry; cached data not authority; configured gateway snapshot replaced atomically, old complete snapshot retained on refresh failure, invoke never automatically replayed | design:446,773; service/v1alpha1:124–138 |

## Verification needed for the revision

Use fixed-profile wrong-input/connection/old-cursor traces; Argo exact name OR stable label positives and sibling/alias/normalization negatives; distinct configured source identities and explicitly unproved physical continuity. For persistence, trace each uncertain acknowledgement boundary, competing metadata publication/revocation, safe retention/capacity and restart with missing continuity. This can be reviewed as normative/textual evidence now. Generated schemas can check selected values and references, but cannot prove real uniqueness, durable fsync, clocks, CAS linearizability, private source equality, provider completeness or actual dispatch counts. No runtime implementation or unrelated provider research is necessary to close this specification review.
