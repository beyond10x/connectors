---
format: aep.planning-md/1
id: review-result:federation-subjects-r1-20260908
kind: review-result
status: active
title: Independent F03 subjects review round 1
relations:
- reviews: story:contracts-federated-approval
revision: 1
---
# Independent F03 semantic review A — canonical approval and redemption ownership

Baseline: clean local main `b5ace3f29d6deb1dbc9e5ddd50a8a8b02152cb2f` on 2026-09-08. Scope: `story:contracts-federated-approval` (F03), one gateway and one executing leaf, including caller-selected managed connections. No runtime implementation, planning mutation, source edit or additional agent was performed. Read the applicable planning and ESS skills for artifact/model boundaries. Exact hashes and frozen source bytes for 22 inspected files accompany this report.

**Verdict: specification work required.** The baseline correctly keeps delegated mutation unadvertisable. F03 can close that gap only when a complete trace preserves one canonical subject and one redemption authority; forwarding unchanged approval bytes without that trace remains insufficient.

The requirements below are independent review IDs for the existing F03 finding, not new original F/E review findings. Recommendations are identified as such; the final author must choose and document the exact representation and verification binding.

## Baseline evidence

- Current federation projects `name__operation.id`, computes a gateway descriptor revision from gateway configuration plus source revisions, and invokes the selected leaf under an independently configured downstream credential (`crates/connectors-host/src/federation.rs:113`–`:186`; service v1alpha1 Federation section). It currently serves reads only. These are three different coordinates: gateway presentation ID/revision, executing leaf ID/revision, and authenticated downstream peer. They are not interchangeable authority subjects.
- Mutation §4 verifies caller/operation/connection/input/revision/expiry and its SDK table still requires unchanged approval forwarding (`contracts/operations/v1alpha1/semantics.md:95`, `:216`). Key namespaces separately include receiver, admitted tenant/realm/caller/executor and direct/federated origin; fingerprints include source-qualified operation, connection metadata revision, semantic contract/profile, descriptor/config revision and canonical input digest (`:146`–`:150`).
- E02 now makes extended fields/paths explicit and prohibits broad-token substitution. Its service §3.6 requires F03 to settle receiver/audience, method/route, canonical operation/connection/revisions, actual forwarded body, describe correlation, replay/clock/key bounds and one redemption owner. It is intentionally not yet a working delegation protocol (`contracts/service/v1alpha2/semantics.md:96`).
- Design §16 preserves source identity, treats aliases as presentation, requires both gateway and leaf admission, prohibits retries/rerouting, and starts with one hop. §23.3 requires one active one-time approval ledger and mutating route during cutover. Do not replace those requirements with a new central credential store or the old keyed-cell architecture.
- Old `ApprovalGate` checks exact nonempty issuer/subject/operation/connection/input digest and expiry and claims once in its store (`../connectors/crates/domain/src/approval.rs:294`, `:475`). **This is not evidence that the old full system trusts caller-made ApprovalRecord values.** Hosted enforcement loads a previously issued record from receiver storage before calling the gate (`../connectors/crates/server/src/hosted/enforcement.rs:412`, `:497`); issuance authenticates a human, verifies current operation/connection/description, then stores the generated reference (`../connectors/crates/server/src/hosted/approval.rs:17`). The gate alone is neither a signature protocol nor a federation binding. Its stored record omits descriptor revision even though issuance checks that revision, so the new stronger revision binding must be explicit rather than claimed inherited.
- The old design also explicitly limits recovery to a single hosted replica because its scan lacks fencing (`../connectors/docs/design/13-grant-evaluation-and-approval-redemption.md:117`). Preserve the need for one serialized authority, not that historical deployment or storage restriction.

## Required decisions and defects

### F03-A01 — P1: Canonical approval target is not defined across alias, leaf and connection coordinates

The current approval prose says operation/connection/revision without selecting which hop owns them. A gateway-visible `prod__issue.create` at gateway revision G14 is not the executing leaf's `issue.create` at L9. A managed connection similarly needs resolution to its leaf-owned binding, not a string with the same spelling in two scopes.

**Required decision:** define one complete canonical subject reconstructed by the executing receiver. Recommended shape: stable executing instance identity; leaf-owned operation reference; semantic family/version and profile; exact leaf descriptor and effect-relevant configuration revision; stable resolved leaf connection and its admitted metadata revision; canonical input algorithm identifier and digest; verified actor authority; and a tagged direct/federated origin identity. Bind the selected approval issuer/reference/expiry to that complete subject. The gateway is origin/presentation context, never a substitute for the leaf owner.

Use an injective structured encoding with explicit absence and exact string/byte rules. Do not splice IDs with a delimiter or compare only an unqualified operation name. The existing `adapter-v1-canonical-json` input digest is only the input coordinate, not the entire approval subject. Credentials, credential generations and refresh bytes are excluded; effect-relevant binding changes must alter the semantic revision.

**Required observation:** the canonical subject before approval and after leaf reconstruction is exactly equal; change leaf instance, semantic profile, connection, input or relevant revision and the approval cannot authorize dispatch, even if alias/body text remains superficially similar.

### F03-A02 — P1: Client/issuer cannot obtain the resolved managed-connection subject from existing metadata

The current public Operation metadata cannot resolve a caller-selected managed connection's leaf ref/revision. A scheme supporting only the implicit configured connection would not satisfy F03 for the proposed auth.connection family. Asking clients to reconstruct authority by concatenating gateway aliases or scraping unrelated connection-list results spreads correctness across unfinished management APIs.

**Recommended bounded solution:** introduce a safe host-owned **approval preparation read**, with a declared profile and input/result schemas. It accepts the gateway-visible operation and descriptor revision, selected connection (explicit when managed), business input and any executor assertion. Current gateway/leaf metadata and policy resolution returns the complete canonical leaf subject plus separate gateway routing context. Apply the same resolution rule to configured and managed connections.

Preparation must grant, reserve and spend nothing. It performs no provider credential resolution/use, identity probe, preflight refresh or business request. It can perform bounded authenticated leaf control-plane resolution and its normal admitted audit. It must refuse unresolvable/out-of-scope selections safely. The result is a value, not a PreparedApproval entity, reservation or proof that later execution will be admitted. Both hops revalidate at invoke; a stale preparation is no authority.

Issuance/consent UX and general connection CRUD can stay outside F03. The narrow control read is necessary to make the selected subject concretely reviewable for managed connections; fixed per-operation origin metadata alone is insufficient. If another approach is selected, it must provide equivalent authenticated exact-subject visibility without assuming implicit connections or caller-assembled private mappings.

**Required observation:** Alice selects managed connection A, sees the exact safe leaf binding/revisions and approves it; Bob's connection B or a subsequently replaced/reassigned A cannot be substituted between preparation and invocation. The output never discloses secret refs, private provider URLs or proxy bindings.

### F03-A03 — P1: Routing freshness, signed semantic identity and remapping races need separate rules

Gateway descriptor revision changes when any source/config changes; the leaf revision identifies its own contract. Blindly replacing the client's gateway revision with the leaf revision makes an unchanged approval unverifiable. Blindly signing every routing value can also conflate harmless gateway snapshot refresh with semantic approval identity without saying so.

**Required decision:** name separate coordinates for (a) gateway-visible operation/descriptor snapshot used to select an immutable route; (b) leaf semantic target/revisions sealed in approval; and (c) any route-binding generation that changes effect-relevant interpretation. Specify whether a purely unrelated gateway snapshot change invalidates the approval or only forces describe/preparation refresh and deliberate resubmission. Either chosen rule must be explicit; no silent auto-replay or target replacement is allowed. Relevant leaf/connection/config changes always invalidate the former subject.

The gateway selects one reviewed mapping and must use that mapping for the entire invocation. The leaf verifies its own current target and the trusted forwarding context; the signed approval is forwarded unchanged, not rewritten by the gateway to fit a new leaf. A gateway policy narrowing/route revocation observed before its dispatch decision refuses; no claim of atomicity with changes after actual forward is implied.

**Required observation:** prepare at G14/L9, then update G or L independently. The trace states the exact refusal/refresh behavior. Repointing the same alias to another leaf or connection never reuses approval or a live idempotency fingerprint as unchanged authority.

### F03-A04 — P1: Original actor/origin cannot be derived from the broad downstream credential

The executing leaf must distinguish authenticated gateway peer, originating admitted principal, optional realm/tenant and executor. A gateway token alone identifies only the configured service principal. An issuer-signed approval also does not replace current host grants or provider restrictions.

**Required decision:** specify the receiver-owned trust/configuration that permits exactly this gateway to attest exactly the narrowed actor/origin context for this leaf/audience/route. Bind that verified context to the selected request, and re-evaluate leaf admission and provider restrictions. A body executor remains an assertion until verified. Direct origin and federated origin are tagged and distinct; another gateway origin must not reuse the approval, even if it forwards the same leaf operation/body. Key rotation preserves stable configured peer/origin identity rather than changing namespaces with token bytes.

**Concrete ESS gap:** `connectors.idempotency.AuthorityScope.executor` is currently a required `String` (`ess/domains/idempotency.yaml:11`–`:18`), while service authentication/admission allows no executor. F03 must choose explicit absence semantics. Do not synthesize an empty string, collapse it into principal, or manufacture an entity merely to fill the type. Preserve absent realm versus `default` and absent tenant versus wildcard.

The old issued-reference store proves authority by trusted lookup, whereas the new proposal sketches issuer-signed evidence. Choose the new proof source and verifier trust/key rules explicitly; matching a caller-supplied issuer string is not sufficient. Issuer authority, original caller, executor and gateway peer remain distinct facts.

**Required observation:** gateway admits Alice narrowly; leaf retains Alice's exact authority and checks its own policy. A forged actor, a different origin/receiver, an executor mismatch, realm absence substitution or an out-of-scope connection is refused before new business dispatch. Policy outage cannot fall back to static-bearer admission.

### F03-A05 — P1: One approval must have one durable redemption authority, independent of route replicas

The gateway and leaf both run host machinery, but only one authority may spend the approval. Spending once at each hop either rejects the legitimate request or creates two independent chances to execute. A gateway must not spend approval before forwarding and then ask the leaf to trust an unbound assertion that it was spent.

**Required decision:** select one logical redemption authority keyed by the exact configured issuer/reference domain and bound to the executing attempt. Recommended ownership: the executing leaf is the sole verifier/redemption coordinator for business execution; the gateway may precheck but does not spend. A backend may be external or replicated, but every replica for that logical leaf must observe the same serialized durable claim authority. Do not key spend uniqueness by request ID, alias, current gateway revision, connection pool or transient process identity. Do not add gateway origin to a spend key in a way that lets the same approval be consumed independently through another ingress.

Receiver/target binding prevents a proof for leaf L from authorizing L2, regardless of independent physical stores. Cutover/failover cannot clone the same stable receiver identity with an empty redemption ledger. A lost/ambiguous spend acknowledgement does not grant dispatch and never restores a spent approval. Retention must prevent resurrecting valid approval authority after compaction; custody cleanup is unrelated.

This must preserve F01 ordering: prepare the durable business attempt, spend required authority, then separately win Prepared→Dispatching. Spending is not evidence of send and is not the dispatch fence. Gateway audit is distinct from the leaf attempt and does not create a second business ledger owner.

**Required observation:** two concurrent gateway replicas present the same proof to the leaf; at most one spend and one dispatch gate succeed. Repeat after gateway/leaf restart, ambiguous spend acknowledgement and route cutover. A failed or fenced attempt may leave approval spent, but cannot falsely claim it reusable.

### F03-A06 — P1: Transport anti-replay and keyed outcome replay must not invalidate one another

Delegation needs a bounded replay/expiry rule, while F02 explicitly permits an independently admitted exact key replay without re-verifying live original approval or spending it again. A leaf verifier that rejects spent/expired approval before checking an admitted existing key would regress the settled replay semantics.

**Required decision:** distinguish (1) authenticated delivery/proof replay protection, (2) approval one-time spending for a new business attempt, and (3) the receiver's key reservation and result replay. A fresh intentional client observation can use a new authenticated delegation delivery while carrying the unchanged original approval field; after current result admission, an exact live key observes the original rather than requiring that old approval to remain usable. Invalid current transport authority still refuses before result disclosure. Reusing an old delivery nonce is not a new request, and a transport refusal says nothing about whether the original business attempt ran.

Pin the actual forwarded request body—including key/approval/executor semantics—and bind method/path/receiver/selected route. State how describe or preparation with no invocation body is correlated and replay-limited. Bind the purpose so a proof for describe/preparation cannot authorize business invoke. Do not automatically mint another delivery and resend after a lost mutation response.

**Required observation:** lost original reply followed by a deliberate same-key observation with fresh delegated delivery and expired/spent original approval returns the admitted durable outcome without new dispatch/spend. Changed key fingerprint conflicts; an unauthorized observer sees no original existence/result; a repeated signed delivery is refused without implying the original was not attempted.

### F03-A07 — P1: Gateway failure must preserve the leaf's attempt identity and effect knowledge

The gateway's own timeout/cancellation/audit state cannot settle the leaf's business attempt. It may lose a downstream answer after the leaf committed success, or fail before sending anything; these have different evidence but neither gives the gateway permission to resend elsewhere.

**Required decision:** preserve original leaf attempt/request identity and its mutation observation through the gateway, with separate outer request correlation and separate source_audit. Explain request-ID mapping if the gateway creates a new leaf request ID; it must not redefine the approval/idempotency subject. When the gateway lacks a valid definitive leaf observation after possible forward, report unknown without inventing a leaf attempt ref. Never manufacture not_attempted from a cancelled local future or missing leaf audit.

Only the leaf's atomic pre-dispatch fence can prove that an anchored leaf attempt did not dispatch. A pre-forward gateway refusal can truthfully describe that refused observation/candidate, but must not assert that an existing same-key original never ran. Both hops retain bounded deadlines; the gateway cannot silently reset a new leaf execution budget after waiting.

**Required observation:** known leaf success plus gateway final-audit failure preserves applied; dropped downstream response becomes unknown with no fabricated identity; duplicate-waiter cancellation does not cancel the original. A stale leaf response may refresh route metadata but never triggers automatic reinvocation.

### F03-A08 — P2: ESS should type the chosen values without inventing an ownership graph

Current models deliberately leave operation qualification, ApprovalRedemption and authority/connection ownership unresolved (`ess/domains/mutations.yaml:42`–`:55`; service_wire final comments). F03 is the point to replace only the uncertainties actually settled by its contract.

**Recommended modeling boundary:** type the canonical approval subject, resolved target, origin/actor authority with explicit executor absence, separate routing snapshot, prepared-subject result and verified delivery/correlation values. Reuse existing EffectKnowledge and AttemptId. Reconcile the existing idempotency namespace/fingerprint representation rather than maintain two differently qualified copies. Public preparation is a value, not a durable reservation entity.

If F03 introduces a durable redemption receipt, give it the selected issuer/reference identity and exact attempt reference only after ownership/lifetime are stated. A receipt that must survive route configuration or replay-cache deletion must not be modeled as owned by either. Issuer, tenant, principal, GrantRecord and Connection entity relations stay UNMAPPED where their identities/lifecycles remain owned by other stories. Do not fabricate one-to-one grant/audit relationships or runtime OperationDeclaration ownership from a gateway alias.

ESS structural validation cannot prove signature validity, canonical byte equality, authenticated actor derivation, global spend uniqueness, distributed fencing, clock/replay bounds, route snapshot atomicity or actual send counts. Record those as executable obligations with exact positive/adversarial vectors and a separate manual semantic trace. Do not turn a compiled scenario or a string-valued field into proof of authority.

## Suggested complete one-hop trace

This is a recommended trace shape, not a selected wire codec:

1. Alice authenticates to gateway G. G's visible operation and selected managed connection resolve, through current admitted metadata, to leaf L / operation O / connection C and exact semantic revisions. Preparation returns those safe canonical coordinates plus G's distinct routing snapshot. It grants/spends/reserves nothing and performs no provider I/O.
2. The authorized approval issuer binds Alice's verified actor/origin, L/O/C, semantic revisions and canonical input to one approval reference/expiry. The human-visible subject and verifier subject are the same structured value; no private connection locator is exposed.
3. Alice explicitly invokes G under the current visible revision with selected C, input, key and unchanged approval. G revalidates its route and narrowing policy. It does not redeem or create the business attempt.
4. G sends one bounded, authenticated delegated invocation to the selected L, binding peer/origin/actor, receiver, purpose/method/path, routing and semantic subjects, actual leaf body and deadline/correlation. No alternate route or second send follows a lost reply.
5. L verifies current delivery authority, reconstructs the target from its own operation/connection/configuration, rechecks current caller/result admission and follows the settled F02 lookup/miss ordering. Exact admitted replays need no new approval spend. A new candidate verifies the exact issuer-approved subject and expiry.
6. L durably audits/prepares/reserves as required, performs the sole approval claim, then wins its separate dispatch gate and dispatches once. Any failed/ambiguous prior step prevents opening that gate; gateway and leaf replicas share one logical claim authority.
7. L returns known effect or explicit unknown with its original attempt identity when known. G preserves that observation and provenance, adds its own audit/correlation, and never upgrades absent evidence into not_attempted or applied.
8. A later deliberate same-key observation uses fresh valid current transport authority and current result admission. It observes the durable original under F02, including a refusal/unknown, without reusing dispatch authority. New fingerprint/origin/semantic revision follows the explicit conflict/refusal rule.

## Minimum final review matrix

The final verification should cover: configured **and managed** connection preparation; direct versus federated origin; same leaf through another gateway; alias repointing; unrelated gateway snapshot change versus relevant leaf/connection change; actor/executor/realm mismatch; issuer/evidence substitution; proof purpose/body/receiver mismatch; concurrent claim and ambiguous claim acknowledgement; exact keyed replay with expired original approval; same-key conflict under changed semantic revision; lost downstream answer; gateway final-audit failure; and failover with retained versus missing redemption authority. These observations are what would make F03's acceptance claim independently checkable. No implementation is required to write the specification and authored/manual vectors honestly.
