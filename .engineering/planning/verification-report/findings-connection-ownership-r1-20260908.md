---
format: aep.planning-md/1
id: verification-report:findings-connection-ownership-r1-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:connection-ownership-r1-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 4284aeda8984f0a1ed28139acaad7c0c0b2ffaa2f4610b7af254b1b6086bac6f
relations:
- verifies: review-result:connection-ownership-r1-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:connection-ownership-r1-20260908

This supplements [the immutable original](../review-result/connection-ownership-r1-20260908.md).
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
    "file": "contracts/auth/connection/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## RCB-01 — P2: separate host management orchestration from provider operation implementation\n\n`contracts/auth/connection/v1alpha1/semantics.md:64` and `:141` call connections.* adapter operations. In contrast, `docs/design.md:524` and `:551` assign lifecycle, concurrency and recovery to the shared coordinator; `:531` expressly excludes callback ownership from the runtime adapter. Connection metadata/publication is already host-owned at connection `:123` and custody `:576` in design. Merely forwarding an operation envelope cannot decide the persistence owner.\n\nRequired selection: public safe management may use the existing extended operation envelope and selected management payloads, but the executing host coordinates scope, acquisition/connection records, state/revisions, custody publication, audit and routing. Provider code owns reviewed auth request construction, exchange/refresh/revoke interpretation and identity/scope validation through private ports. Custody stores opaque immutable material and does not decide connection lifecycle. Give begin/complete/status/list/describe/revoke one owner each; remove the contradictory default. Curation must describe actual effects: auth.begin creates coordinator state, and client-credentials begin can perform an exchange (`acquisition:122`), so “safe” must not silently mean read-only. No new general CRUD service, mandatory HTTP route or issuer UI is needed.",
    "line": 64
  },
  {
    "file": ".engineering/planning/review-result/connection-ownership-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## RCB-02 — P2: management admission cannot require a pre-existing usable provider connection\n\n`acquisition:33` permits begin without repair_of and `connection:68` lists connections. These cannot resolve an existing ready connection as the generic service context suggests (`service/v1alpha2:62`, `:105`; compatibility `:64`). Repair/revoke are precisely needed when credentials are unusable; applying `connection:74` as a universal management precondition makes recovery impossible.\n\nRequired selection: define target rules for each selected management operation: instance/profile scope for create/list; exact admitted connection for describe/revoke/repair; acquisition-owner scope for status. Establish one authoritative target when outer connection and a repair target are both present, and reject contradictions. Separate permission to discover/list/inspect/create/repair/revoke from permission to invoke business operations. Current host policy and tenant/principal/optional realm/executor scope are checked independently at each action; possession of an opaque ref is not authority. Unready, disabled or revoked provider access must not itself block an independently permitted status/list/repair/revoke action. Still deny unsupported repair of a terminal revoked binding where the chosen lifecycle forbids resurrection. Metadata reads perform no credential capture, identity probe, refresh or provider verification (`evidence:92`)."
  },
  {
    "file": ".engineering/planning/review-result/connection-ownership-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## RCB-03 — P2: pin one acquisition coordinator and preserve it through federation completion\n\nThe owning-service callback default (`acquisition:180`) and design `:785` are consistent, but a complete trace does not yet say how an acquisition retains its owner when an alias/route changes, or how current admission constrains a delayed completion. F03 only authenticates selected describe/prepare/invoke requests (`service/delegation:127`); protected callback ingress remains separately bound by compatibility `:138` and service/v1alpha2 `:156`. It cannot authenticate an OAuth callback just because auth.begin traversed a gateway.\n\nRequired selection: begin records the single logical coordinator/instance, admitted originating scope, selected profile/registration and optional repair binding plus its revision. Status and trusted completion resolve that record at its owner; gateway aliases are presentation and cannot repoint an acquisition to a different coordinator or start a second exchange. Both hops admit forwarded safe management under F03; callback evidence follows the separately trusted completion port at the owner, never generic invoke or arbitrary caller-supplied destinations. Before publication, validate one-use/expiry/correlation and current permission/binding; revocation or a changed repair target wins. A callback proof alone cannot revive withdrawn management authority. If supported, a relay transports to that same owner under its own specified protected binding; it is not another coordinator. No relay is implied merely by operation forwarding. Test begin at leaf A, route repointed to B, then status/completion: retain A's ownership or refuse safely, never complete at B."
  },
  {
    "file": ".engineering/planning/review-result/connection-ownership-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## RCB-04 — P2: state the narrow action-URL exception and keep callback authority private\n\nDesign `:545` excludes secret material/reusable completion authority from ordinary model results; acquisition `:42`, `:44`, `:69` intentionally return a one-use action URL. Design `:785` forbids ordinary discovery of private callback capabilities, not all use of an operation envelope. Compatibility `:56` and `:138` already distinguish safe profile metadata from an admitted action URL.\n\nRequired selection: ordinary describe/list/status expose supported safe management/profile/status metadata only, never callback state, completion tickets, registration credentials, custody refs, authorization codes or provider credential material. A successful admitted begin may deliver its bounded one-use action to the explicitly trusted UI path; this is a narrow result exception, not public operation discovery or reusable completion authority. Treat the action URL as sensitive control data: do not copy it into logs, generic audit diagnostics or reusable result replay. An acquisition ref is only a scoped lookup coordinate, not completion authority. Completion evidence stays at the protected owner ingress/private provider interface. The selected action/callback placement must actually be reachable by its intended parties; an unreachable private leaf endpoint cannot be advertised as a working flow. Exact URL/ticket/HTTP/UI protocol details may remain with the acquisition binding owner."
  },
  {
    "file": ".engineering/planning/review-result/connection-ownership-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## RCB-05 — P2: distinguish local connection revocation from uncertain provider cleanup\n\n`connection:70` combines marking revoked, provider revocation and cursor/session invalidation into one “Effect.” The coordinator already serializes revoke with refresh/publication (`acquisition:92`, `:98`; `connection:84`). The documents do not yet explain the outward result when local revocation commits but custody/provider cleanup fails, or when an acknowledgement is lost.\n\nRequired selection: the host's authoritative revocation/invalidation decision cannot depend on provider readiness or be undone by provider cleanup failure. Only its provider auth implementation performs any configured external revocation, using leaf-local material. Preserve the known local revoked state and separately report the safe known/uncertain provider-cleanup observation; do not report the connection ready, pretend cleanup succeeded, or resurrect authority because an exchange reply was lost. Gateway forwarding does not revoke/spend/create a second owner record. Same-current-scope observation must not automatically replay an uncertain provider call; monotonic local state is not proof that a provider operation is naturally idempotent. Use the selected management/mutation outcome rules consistently rather than inferring a second protocol or silently claiming the general F01 provider-attempt flow already covers every local action."
  },
  {
    "file": ".engineering/planning/review-result/connection-ownership-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## RCB-06 — P2: global readiness cannot be the union of operation requirements\n\n`connection:58` globally marks insufficient_scope when scopes for enabled operations are missing, and `:74` refuses every non-ready connection. Evidence's public example does the same at `evidence:39`–`:49`. Yet `evidence:132` requires a read with read scope to succeed while write scope is absent. `evidence:74` also calls description and invocation “the same predicate,” although scope and permission checks are operation/target-specific (`:70`, `:71`; profile `:79`). Enabling an unrelated write would currently disable an otherwise usable read.\n\nRequired selection: one operation-independent viability reduction, plus per-invocation eligibility. Global viability uses only the selected connection/profile's declared common prerequisites and current binding/material facts. Selected operation profile/purpose, required scope alternative, target permission, host authorization and exact-generation freshness are evaluated separately; no union over enabled operations. If a global insufficient_scope state is retained, constrain it to explicitly declared profile minimum viability, not a missing optional write grant. Recompute operation scope evidence after refresh narrowing without automatically poisoning unrelated reads. A permission denial for target B must not globally disable access to admitted target A. “Same checks” can mean shared evidence rules, not identical global and operation predicates."
  },
  {
    "file": ".engineering/planning/review-result/connection-ownership-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## RCB-07 — P2: provide one state reduction and canonical state/error vocabulary\n\nThe connection table (`connection:54`–`:62`) contains disabled but no pending; its future lifecycle (`:132`) contains pending but no disabled. Acquisition failure/scenario uses scope_insufficient (`acquisition:47`, `:139`), while refresh, profile, E02 and ESS use insufficient_scope. A generic evidence failure currently chooses reauthorization_required even when the failed requirement is operation-specific (`evidence:77`). Repair mismatch also says “prior state” (`acquisition:71`) while connection `:106` prescribes reauthorization_required.\n\nRequired selection: a deterministic table with precedence and applicability for pending/unvalidated, disabled, locally revoked, custody-unavailable, invalid/revoked/uncertain credential, stale parent/binding, common profile insufficiency (if retained), and ready. State whether pending belongs to an unpublished acquisition or a published connection awaiting its first generation; an in-progress repair must not accidentally erase an existing usable binding. Terminal local revocation is different from provider credential invalidity that may be repaired. Parent degradation and invocation route_unavailable remain distinct from an operation-local route_refused; never direct-egress fallback (`discovery/mediated_route:55`, `:64`, `:83`). Use insufficient_scope consistently for that same failure, and distinguish payload state/private result enums from E02 ErrorCode. Define each scenario's connection state, operation refusal and allowed management next action in one place, then cross-check all public examples, conformance text and future-model notes."
  },
  {
    "file": "ess/domains/credentials.yaml",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## RCB-08 — P2: mark the actual ESS boundary and preserve established evidence/refresh guarantees\n\nConnection `:132`, acquisition `:167` and profile `:119` list Connection/Acquisition/AuthProfile identities and lifecycles under “ESS entities,” but current ESS declares none of those entities. Existing `ConnectionBinding` is a value; `CredentialGeneration` has immutable Captured state and explicitly leaves those owner relations UNMAPPED (`ess/domains/credentials.yaml:12`, `:46`–`:55`). EvidenceSnapshot is a value and DispatchAdmission is a separate transient lifecycle (`credential_evidence.yaml:31`, `:43`, `:71`). A public readiness enum is not any of those internal state machines.\n\nRequired selection: make the future-model notes consistent with the selected status vocabulary and explicitly distinguish proposed/unmodeled owners from actual typed values and existing refresh/admission entities. Do not add guessed CRUD/lifecycle commands just to make prose seem executable. If this task chooses to introduce typed readiness/management values, use the minimal ESS workflow and retain unknown owner/cardinality relations as UNMAPPED. Preserve F04/F05's immutable-generation identity, admitted validation, per-check freshness, publication/revocation cutoff and no second refresh exchange; neither a cached global ready state nor successful acquisition status bypasses those dispatch checks.",
    "line": 12
  }
]
```

