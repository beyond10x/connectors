---
format: aep.planning-md/1
id: review-result:connection-ownership-r1-20260908
kind: review-result
status: active
title: Independent connection ownership review round 1
relations:
- reviews: story:contracts-connection-readiness
- reviews: story:contracts-management-boundary
revision: 1
---
# Independent connection semantics review B — initial requirements

Verdict: **revision required within E04 and F07/E19/E21**. Eight concrete requirements below; these concern the selected semantics, not runtime implementation. Baseline: `12c11f4b43cd6cf5ff67d3243017887b28788344`. All file:line references refer to the immutable `initial-snapshot/`; `source-hashes-initial.json` preserves 21 source files. No source/planning changes, runtime tests, integration calls, additional agents or other reviewer reports were used.

## RCB-01 — P2: separate host management orchestration from provider operation implementation

`contracts/auth/connection/v1alpha1/semantics.md:64` and `:141` call connections.* adapter operations. In contrast, `docs/design.md:524` and `:551` assign lifecycle, concurrency and recovery to the shared coordinator; `:531` expressly excludes callback ownership from the runtime adapter. Connection metadata/publication is already host-owned at connection `:123` and custody `:576` in design. Merely forwarding an operation envelope cannot decide the persistence owner.

Required selection: public safe management may use the existing extended operation envelope and selected management payloads, but the executing host coordinates scope, acquisition/connection records, state/revisions, custody publication, audit and routing. Provider code owns reviewed auth request construction, exchange/refresh/revoke interpretation and identity/scope validation through private ports. Custody stores opaque immutable material and does not decide connection lifecycle. Give begin/complete/status/list/describe/revoke one owner each; remove the contradictory default. Curation must describe actual effects: auth.begin creates coordinator state, and client-credentials begin can perform an exchange (`acquisition:122`), so “safe” must not silently mean read-only. No new general CRUD service, mandatory HTTP route or issuer UI is needed.

## RCB-02 — P2: management admission cannot require a pre-existing usable provider connection

`acquisition:33` permits begin without repair_of and `connection:68` lists connections. These cannot resolve an existing ready connection as the generic service context suggests (`service/v1alpha2:62`, `:105`; compatibility `:64`). Repair/revoke are precisely needed when credentials are unusable; applying `connection:74` as a universal management precondition makes recovery impossible.

Required selection: define target rules for each selected management operation: instance/profile scope for create/list; exact admitted connection for describe/revoke/repair; acquisition-owner scope for status. Establish one authoritative target when outer connection and a repair target are both present, and reject contradictions. Separate permission to discover/list/inspect/create/repair/revoke from permission to invoke business operations. Current host policy and tenant/principal/optional realm/executor scope are checked independently at each action; possession of an opaque ref is not authority. Unready, disabled or revoked provider access must not itself block an independently permitted status/list/repair/revoke action. Still deny unsupported repair of a terminal revoked binding where the chosen lifecycle forbids resurrection. Metadata reads perform no credential capture, identity probe, refresh or provider verification (`evidence:92`).

## RCB-03 — P2: pin one acquisition coordinator and preserve it through federation completion

The owning-service callback default (`acquisition:180`) and design `:785` are consistent, but a complete trace does not yet say how an acquisition retains its owner when an alias/route changes, or how current admission constrains a delayed completion. F03 only authenticates selected describe/prepare/invoke requests (`service/delegation:127`); protected callback ingress remains separately bound by compatibility `:138` and service/v1alpha2 `:156`. It cannot authenticate an OAuth callback just because auth.begin traversed a gateway.

Required selection: begin records the single logical coordinator/instance, admitted originating scope, selected profile/registration and optional repair binding plus its revision. Status and trusted completion resolve that record at its owner; gateway aliases are presentation and cannot repoint an acquisition to a different coordinator or start a second exchange. Both hops admit forwarded safe management under F03; callback evidence follows the separately trusted completion port at the owner, never generic invoke or arbitrary caller-supplied destinations. Before publication, validate one-use/expiry/correlation and current permission/binding; revocation or a changed repair target wins. A callback proof alone cannot revive withdrawn management authority. If supported, a relay transports to that same owner under its own specified protected binding; it is not another coordinator. No relay is implied merely by operation forwarding. Test begin at leaf A, route repointed to B, then status/completion: retain A's ownership or refuse safely, never complete at B.

## RCB-04 — P2: state the narrow action-URL exception and keep callback authority private

Design `:545` excludes secret material/reusable completion authority from ordinary model results; acquisition `:42`, `:44`, `:69` intentionally return a one-use action URL. Design `:785` forbids ordinary discovery of private callback capabilities, not all use of an operation envelope. Compatibility `:56` and `:138` already distinguish safe profile metadata from an admitted action URL.

Required selection: ordinary describe/list/status expose supported safe management/profile/status metadata only, never callback state, completion tickets, registration credentials, custody refs, authorization codes or provider credential material. A successful admitted begin may deliver its bounded one-use action to the explicitly trusted UI path; this is a narrow result exception, not public operation discovery or reusable completion authority. Treat the action URL as sensitive control data: do not copy it into logs, generic audit diagnostics or reusable result replay. An acquisition ref is only a scoped lookup coordinate, not completion authority. Completion evidence stays at the protected owner ingress/private provider interface. The selected action/callback placement must actually be reachable by its intended parties; an unreachable private leaf endpoint cannot be advertised as a working flow. Exact URL/ticket/HTTP/UI protocol details may remain with the acquisition binding owner.

## RCB-05 — P2: distinguish local connection revocation from uncertain provider cleanup

`connection:70` combines marking revoked, provider revocation and cursor/session invalidation into one “Effect.” The coordinator already serializes revoke with refresh/publication (`acquisition:92`, `:98`; `connection:84`). The documents do not yet explain the outward result when local revocation commits but custody/provider cleanup fails, or when an acknowledgement is lost.

Required selection: the host's authoritative revocation/invalidation decision cannot depend on provider readiness or be undone by provider cleanup failure. Only its provider auth implementation performs any configured external revocation, using leaf-local material. Preserve the known local revoked state and separately report the safe known/uncertain provider-cleanup observation; do not report the connection ready, pretend cleanup succeeded, or resurrect authority because an exchange reply was lost. Gateway forwarding does not revoke/spend/create a second owner record. Same-current-scope observation must not automatically replay an uncertain provider call; monotonic local state is not proof that a provider operation is naturally idempotent. Use the selected management/mutation outcome rules consistently rather than inferring a second protocol or silently claiming the general F01 provider-attempt flow already covers every local action.

## RCB-06 — P2: global readiness cannot be the union of operation requirements

`connection:58` globally marks insufficient_scope when scopes for enabled operations are missing, and `:74` refuses every non-ready connection. Evidence's public example does the same at `evidence:39`–`:49`. Yet `evidence:132` requires a read with read scope to succeed while write scope is absent. `evidence:74` also calls description and invocation “the same predicate,” although scope and permission checks are operation/target-specific (`:70`, `:71`; profile `:79`). Enabling an unrelated write would currently disable an otherwise usable read.

Required selection: one operation-independent viability reduction, plus per-invocation eligibility. Global viability uses only the selected connection/profile's declared common prerequisites and current binding/material facts. Selected operation profile/purpose, required scope alternative, target permission, host authorization and exact-generation freshness are evaluated separately; no union over enabled operations. If a global insufficient_scope state is retained, constrain it to explicitly declared profile minimum viability, not a missing optional write grant. Recompute operation scope evidence after refresh narrowing without automatically poisoning unrelated reads. A permission denial for target B must not globally disable access to admitted target A. “Same checks” can mean shared evidence rules, not identical global and operation predicates.

## RCB-07 — P2: provide one state reduction and canonical state/error vocabulary

The connection table (`connection:54`–`:62`) contains disabled but no pending; its future lifecycle (`:132`) contains pending but no disabled. Acquisition failure/scenario uses scope_insufficient (`acquisition:47`, `:139`), while refresh, profile, E02 and ESS use insufficient_scope. A generic evidence failure currently chooses reauthorization_required even when the failed requirement is operation-specific (`evidence:77`). Repair mismatch also says “prior state” (`acquisition:71`) while connection `:106` prescribes reauthorization_required.

Required selection: a deterministic table with precedence and applicability for pending/unvalidated, disabled, locally revoked, custody-unavailable, invalid/revoked/uncertain credential, stale parent/binding, common profile insufficiency (if retained), and ready. State whether pending belongs to an unpublished acquisition or a published connection awaiting its first generation; an in-progress repair must not accidentally erase an existing usable binding. Terminal local revocation is different from provider credential invalidity that may be repaired. Parent degradation and invocation route_unavailable remain distinct from an operation-local route_refused; never direct-egress fallback (`discovery/mediated_route:55`, `:64`, `:83`). Use insufficient_scope consistently for that same failure, and distinguish payload state/private result enums from E02 ErrorCode. Define each scenario's connection state, operation refusal and allowed management next action in one place, then cross-check all public examples, conformance text and future-model notes.

## RCB-08 — P2: mark the actual ESS boundary and preserve established evidence/refresh guarantees

Connection `:132`, acquisition `:167` and profile `:119` list Connection/Acquisition/AuthProfile identities and lifecycles under “ESS entities,” but current ESS declares none of those entities. Existing `ConnectionBinding` is a value; `CredentialGeneration` has immutable Captured state and explicitly leaves those owner relations UNMAPPED (`ess/domains/credentials.yaml:12`, `:46`–`:55`). EvidenceSnapshot is a value and DispatchAdmission is a separate transient lifecycle (`credential_evidence.yaml:31`, `:43`, `:71`). A public readiness enum is not any of those internal state machines.

Required selection: make the future-model notes consistent with the selected status vocabulary and explicitly distinguish proposed/unmodeled owners from actual typed values and existing refresh/admission entities. Do not add guessed CRUD/lifecycle commands just to make prose seem executable. If this task chooses to introduce typed readiness/management values, use the minimal ESS workflow and retain unknown owner/cardinality relations as UNMAPPED. Preserve F04/F05's immutable-generation identity, admitted validation, per-check freshness, publication/revocation cutoff and no second refresh exchange; neither a cached global ready state nor successful acquisition status bypasses those dispatch checks.

## Minimal review trace set

1. Admitted creator with no existing connection: discover safe begin, create one owned acquisition, complete at its coordinator, publish only after current validation and durable custody, return safe connection ref; execute no waiting business operation.
2. Caller may use a connection but lacks management permission: management is hidden/refused without cross-scope existence or identity disclosure; provider grant alone does not change that.
3. Same scope, expired credentials/custody outage: permitted safe inspection and repair/revoke do not require business readiness; provider work occurs only through its specifically admitted coordinator step.
4. Federated begin at A, gateway route changes to B: status/completion remain owned by A or refuse; callback never becomes a generic forwarded operation or second acquisition.
5. Local revoke commits, provider revocation reply is lost: local binding remains revoked, no stale completion/refresh can republish, cleanup stays uncertain and is not automatically replayed.
6. Read scope present/write absent, then grant narrowing/target-specific denial: viable read proceeds where its requirements still hold; only ineligible operations refuse; descriptions do not run probes.
7. Exercise each state and overlapping facts (revoked plus custody outage, disabled plus pending repair, degraded parent plus insufficient write scope) against the same precedence table and management permissions.
8. Inspect ordinary discovery/list/status/audit and begin output separately: only the admitted trusted-UI action carries a one-use URL; no callback evidence, registration secret, private credential address or reusable ticket leaks.

These are textual acceptance traces. They do not claim callback, federation, provider, persistence or clock execution. The revision can settle the boundary without implementing runtime or expanding the complete acquisition-profile/issuer UI protocols.
