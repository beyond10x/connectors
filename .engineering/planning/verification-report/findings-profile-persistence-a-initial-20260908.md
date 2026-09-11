---
format: aep.planning-md/1
id: verification-report:findings-profile-persistence-a-initial-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:profile-persistence-a-initial-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: da50cccc637e4da21b95a49da56fd31d0ae545985eb59ca9b43d38310ffdc187
relations:
- verifies: review-result:profile-persistence-a-initial-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:profile-persistence-a-initial-20260908

This supplements [the immutable original](../review-result/profile-persistence-a-initial-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

The original report's verdict and scope remain authoritative; no new verdict is assigned.

## Transcription method

8 findings remain in the scope of this report's final stated conclusion.
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
    "file": "contracts/discovery/resources/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### PP-A-01 — P2 — Finish descriptor-fixed discovery declarations\n\n**Sole correction owner:** `story:contracts-discovery-profiles` (E07).\n\n`contracts/discovery/resources/v1alpha1/semantics.md:31` leaves exact operation names and profile selection deferred, while `docs/adapters/kubernetes.md:56` selects `services.observe` and `docs/adapters/grafana.md:61` selects `datasources.observe`. The input already has only limit/cursor. There is not yet one authoritative rule relating the conceptual family operation to these adapter declarations and to the resolved source.\n\nSelect the concrete declaration/profile mapping in one owner. A descriptor fixes contract/version/profile and the permitted source-selection mode; each invocation resolves exactly one admitted source under that declaration. Multiple managed source connections need ordinary admitted connection selection, not a caller-selected provider/profile or an implicit global source. Wrong or extra profile/source/namespace/target selectors must fail schema/admission rather than retarget the operation. Explain whether any literal `resources.observe` ID is advertised. Keep describe static and preserve F13 scope, page and revision rules. A source or declaration selection change must not silently reuse a cursor or generation from another scope.",
    "line": 31
  },
  {
    "file": "docs/adapters/kubernetes.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### PP-A-02 — P2 — Define Argo recognition separately from callable candidates\n\n**Sole correction owner:** `story:contracts-discovery-profiles` (E14).\n\nThe owner currently lists `argocd` as observation-only (`resources/...:196`), but Kubernetes says every recognized Service has `candidate.confidence = inferred` (`docs/adapters/kubernetes.md:97`) and gives no exact recognition predicate. A client cannot tell whether Argo gets no route candidate, an unusable advertised candidate, or no observation. Shared resources allow unknown observations with `candidate:null` (`resources/...:154`), which also needs an explicit Kubernetes disposition.\n\nSelect API recognition using the **whole exact Service name `argocd-server` OR whole exact value of `app.kubernetes.io/name` equal to `argocd-server`**. Do not turn this into name AND label, arbitrary suffix/substring matching, or a namespace/component guess. Include default-name-only and Helm-renamed-with-stable-label positives; ordinary server-metrics/repo-server/redis and Helm-suffix-only negatives. Define deterministic precedence if different recognition inputs identify competing providers. An observation marker must not manufacture an Argo adapter, callable operation, materialized connection or mediated route.\n\nOld evidence: design10 lines 175–186; `crates/integration-kubernetes/src/local.rs:1171–1211`; `local_tests.rs:112–167`. The old implementation additionally lowercases inputs and accepts aliases `app`, `k8s-app`, `name`. The story selects a narrower stable-label rule. Explicitly disposition these aliases and normalization; do not claim the entire old recognizer is preserved unchanged. Recognition is untrusted inference, not proof that the Service speaks the API or that routing is authorized. Specify whether unknown/noncandidate Services still produce rows, and make candidate-null examples consistent across family and adapter docs.",
    "line": 97
  },
  {
    "file": "docs/design.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### PP-A-03 — P2 — Specify configured source identity without claiming physical cluster proof\n\n**Sole correction owner:** `story:contracts-discovery-profiles` (E32).\n\nThe current collection key deliberately defers physical cluster identity (`resources/...:97`) while design lists future Kubernetes cluster identity (`docs/design.md:939`). Stable source-qualified references are required (`design.md:352`) but configured instance, connection, API authority and physical cluster identity are not yet fully distinguished.\n\nSelect the configured source qualification and API-origin/authority binding, including how its interpretation participates in source/configuration revisions. Two configured instances must remain distinct even if their display names or API origins coincide. Context names, URLs, credential/account identity and Service UID do not independently prove physical cluster continuity; the same endpoint can later serve a replacement cluster and one cluster can have aliases. Either select a separately evidenced physical identity mechanism or explicitly defer it and bound claims accordingly. Configuration/authority changes must fence old scope/cursors/observations/routes; same-identity credential rotation follows the settled generation rules rather than pretending to identify a new physical cluster. Keep private API origins out of public output and perform no discovery/identity probe during describe.",
    "line": 939
  },
  {
    "file": "docs/design.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### PP-A-04 — P2 — Replace the broad host-state paragraph with an inspectable port inventory\n\n**Sole correction owner:** `story:contracts-persistence-ownership` (E28).\n\n`docs/design.md:959–961` names example state and warns against a universal store, but does not provide one named logical port owner, state scope and operation set for each current obligation. Sources alternately call their owner a host, coordinator, metadata authority, ledger or store. The detailed inventory below shows the minimum coverage needed; some rows can share one logical authority without pretending to be one generic storage interface.\n\nAdd the narrow design persistence section required by the existing story. For each row identify exactly one authoritative owner, owned facts versus referenced facts, acknowledgement/atomicity boundary, restart guarantee, bounded retention/capacity behavior, and conservative unavailable/ambiguous handoff. Identify private adapter runtime and derived metadata explicitly, rather than treating every ESS entity or status value as durable storage. Crosslink owning contracts instead of copying a second divergent protocol.",
    "line": 959
  },
  {
    "file": ".engineering/planning/review-result/profile-persistence-a-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "### PP-A-05 — P2 — Name the common metadata ordering boundary without inventing cross-store transactions\n\n**Sole correction owner:** `story:contracts-persistence-ownership` (E28).\n\nSeveral settled contracts require the same facts to participate in one atomic decision: refresh authorize/publish/revoke (`auth/acquisition/...:99–116`), final credential admission (`auth/evidence/...:98`), connection revocation/publication (`auth/connection/...:85–87`), collection publication (`discovery/resources/...:129–133`) and route revalidation (`discovery/mediated_route/...:77`). Merely assigning these to separate independent CAS ports would not satisfy the specified races.\n\nName the logical authority/guarded operations that serialize the relevant metadata and explain which port owns the transaction. A backend may implement several narrow ports, but incompatible independent authorities cannot claim the cross-object cutoff; such a composition refuses the affected profile. Custody remains an immutable sensitive-version owner with acknowledged write before metadata publication, not part of an external provider transaction. An orphan is never active authority. Acquisition completion must preserve its sole-owner one-use/current-scope/expected-target checks and publication ordering across replicas; do not substitute a fresh process-local callback registry after restart. Final checks should retain the existing admitted policy decision boundary, not promise atomicity with every future external policy update."
  },
  {
    "file": ".engineering/planning/review-result/profile-persistence-a-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "### PP-A-06 — P2 — Inventory irreversible receipts and audit independently of replay storage\n\n**Sole correction owner:** `story:contracts-persistence-ownership` (E28).\n\nMutation and federation already select materially different durability promises (`operations/...:97–103,144–171`; `service/delegation.md:149–167`), while audit ownership and its attempt relationship remain an explicit binding obligation (`service/compatibility.md:76`). A single undifferentiated “operation store” row would hide the important guarantees.\n\nKeep atomic Prepared-attempt plus keyed-reservation creation and the unique dispatch gate with the executing host's attempt authority. Keep approval redemption and transport nonce consumption distinct: exact keys, executing-leaf sole ownership across all ingress/replicas, approval bound to one Prepared attempt, no gateway second spend/attempt, no automatic resend after unknown acknowledgement. State the ordered audit anchor → prepare/reserve → spend → dispatch gate → provider → outcome/audit handoffs without promising one transaction with the provider or audit store. Known live applied/refused knowledge survives final persistence failure; recovery without it remains uncertain.\n\nPending/Quarantined keys, consumed refresh authority and spent-approval uniqueness must survive restart/compaction. Their retention is not a generic TTL. Preserve 86,400-second known-result replay expiry and exact-generation retirement, trusted-clock nonce retirement and no automatic approval-tombstone expiry. Capacity refuses new authority before dispatch rather than evicting unresolved/spent facts. Copy/failover/restore must not activate one stable owner identity against empty or divergent uniqueness history."
  },
  {
    "file": ".engineering/planning/review-result/profile-persistence-a-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "### PP-A-07 — P2 — Explicitly separate durable records from nonresumable sessions and deferred event state\n\n**Sole correction owner:** `story:contracts-persistence-ownership` (E28).\n\nDesign §20 includes subscription cursors and assignments; §§14–15 require assignment/provisioning and event persist-before-ack (`design.md:716–751`). Sessions specify single redemption, serialized terminal/lease decisions and no restart reattach (`sessions/...:83,91,97–120`). These obligations disappear if the new inventory covers only connections and mutations, or become misleading if a saved session row is described as restored authority.\n\nInclude named logical ownership for the declared event acceptance/deduplication/checkpoint, assignment/configuration and session authority/accounting boundaries, with explicit deferred profile/backend details where they are not yet selected. Do not implement or invent new event protocols. Distinguish source event intake acknowledgement from independent consumer checkpoint and business approval. For sessions preserve one-use establishment authority and first terminal accounting, but deny reconstruction of live leases, sockets, peer state or resend permission from a persisted record. Restart loses continuity; surviving gates expire under their already-issued deadlines. Session replay/reattach and durable subscription custody remain separately scoped work."
  },
  {
    "file": "ess/domains/credential_evidence.yaml",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### PP-A-08 — P2 — Keep ESS identity/value evidence and retention claims truthful\n\n**Sole correction owner:** `story:contracts-persistence-ownership` (E28), with the discovery profile selection referenced from E07/E14/E32.\n\nThe current ESS has actual CredentialGeneration/RefreshAttempt/AttemptRecord/KeyReservation and delegation receipt identities, but Connection, Acquisition, AuthProfile, custody and persistent discovery relations are deliberately unmapped. DispatchAdmission is explicitly transient (`ess/domains/credential_evidence.yaml:56–64`); discovery is values only (`ess/domains/discovery.yaml:92`); session comments reject guessed persistent ownership (`ess/domains/sessions.yaml:87–91`). Custody's §9 table still presents a conceptual CredentialSet lifecycle and Connection relationship (`auth/custody/...:109–110`).\n\nThe consolidated inventory must label these distinctions explicitly. Port responsibility does not establish an ESS `owns` edge, deletion cascade or executable database guarantee. In particular, KeyReservation references AttemptRecord so replay deletion cannot delete audit/attempt facts; F03's RouteBinding fingerprint value is not the persistent mediated route owner. New profile-selection values can be typed without inventing persistent owners solely to compile. If existing conceptual tables are linked as evidence, mark their undeclared status.\n\nCarry existing limits into each applicable owner without shortening safety guarantees: custody's 8-version cap/24-hour retention cannot evict an active or still-required version; discovery has 500 rows/256 scopes/600-second history and immutable-view retirement; audit/history policies still require explicit binding choices. At capacity, refuse the affected new write/admission where protected live state prevents reclamation. Lost or expired caches may require revalidation or a new discovery epoch; they cannot become positive authority or absence proof. Validation should distinguish typed shapes, textual decision vectors and future real storage fault tests.",
    "line": 56
  }
]
```

