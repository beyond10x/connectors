---
format: aep.planning-md/1
id: verification-report:findings-federation-subjects-r1-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:federation-subjects-r1-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: b36c4f0928e489faf8438019f0c206f5c9f1f0e444bf08dcfdef34b4395c35ef
relations:
- verifies: review-result:federation-subjects-r1-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:federation-subjects-r1-20260908

This supplements [the immutable original](../review-result/federation-subjects-r1-20260908.md).
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
    "file": ".engineering/planning/review-result/federation-subjects-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### F03-A01 — P1: Canonical approval target is not defined across alias, leaf and connection coordinates\n\nThe current approval prose says operation/connection/revision without selecting which hop owns them. A gateway-visible `prod__issue.create` at gateway revision G14 is not the executing leaf's `issue.create` at L9. A managed connection similarly needs resolution to its leaf-owned binding, not a string with the same spelling in two scopes.\n\n**Required decision:** define one complete canonical subject reconstructed by the executing receiver. Recommended shape: stable executing instance identity; leaf-owned operation reference; semantic family/version and profile; exact leaf descriptor and effect-relevant configuration revision; stable resolved leaf connection and its admitted metadata revision; canonical input algorithm identifier and digest; verified actor authority; and a tagged direct/federated origin identity. Bind the selected approval issuer/reference/expiry to that complete subject. The gateway is origin/presentation context, never a substitute for the leaf owner.\n\nUse an injective structured encoding with explicit absence and exact string/byte rules. Do not splice IDs with a delimiter or compare only an unqualified operation name. The existing `adapter-v1-canonical-json` input digest is only the input coordinate, not the entire approval subject. Credentials, credential generations and refresh bytes are excluded; effect-relevant binding changes must alter the semantic revision.\n\n**Required observation:** the canonical subject before approval and after leaf reconstruction is exactly equal; change leaf instance, semantic profile, connection, input or relevant revision and the approval cannot authorize dispatch, even if alias/body text remains superficially similar."
  },
  {
    "file": ".engineering/planning/review-result/federation-subjects-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### F03-A02 — P1: Client/issuer cannot obtain the resolved managed-connection subject from existing metadata\n\nThe current public Operation metadata cannot resolve a caller-selected managed connection's leaf ref/revision. A scheme supporting only the implicit configured connection would not satisfy F03 for the proposed auth.connection family. Asking clients to reconstruct authority by concatenating gateway aliases or scraping unrelated connection-list results spreads correctness across unfinished management APIs.\n\n**Recommended bounded solution:** introduce a safe host-owned **approval preparation read**, with a declared profile and input/result schemas. It accepts the gateway-visible operation and descriptor revision, selected connection (explicit when managed), business input and any executor assertion. Current gateway/leaf metadata and policy resolution returns the complete canonical leaf subject plus separate gateway routing context. Apply the same resolution rule to configured and managed connections.\n\nPreparation must grant, reserve and spend nothing. It performs no provider credential resolution/use, identity probe, preflight refresh or business request. It can perform bounded authenticated leaf control-plane resolution and its normal admitted audit. It must refuse unresolvable/out-of-scope selections safely. The result is a value, not a PreparedApproval entity, reservation or proof that later execution will be admitted. Both hops revalidate at invoke; a stale preparation is no authority.\n\nIssuance/consent UX and general connection CRUD can stay outside F03. The narrow control read is necessary to make the selected subject concretely reviewable for managed connections; fixed per-operation origin metadata alone is insufficient. If another approach is selected, it must provide equivalent authenticated exact-subject visibility without assuming implicit connections or caller-assembled private mappings.\n\n**Required observation:** Alice selects managed connection A, sees the exact safe leaf binding/revisions and approves it; Bob's connection B or a subsequently replaced/reassigned A cannot be substituted between preparation and invocation. The output never discloses secret refs, private provider URLs or proxy bindings."
  },
  {
    "file": ".engineering/planning/review-result/federation-subjects-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### F03-A03 — P1: Routing freshness, signed semantic identity and remapping races need separate rules\n\nGateway descriptor revision changes when any source/config changes; the leaf revision identifies its own contract. Blindly replacing the client's gateway revision with the leaf revision makes an unchanged approval unverifiable. Blindly signing every routing value can also conflate harmless gateway snapshot refresh with semantic approval identity without saying so.\n\n**Required decision:** name separate coordinates for (a) gateway-visible operation/descriptor snapshot used to select an immutable route; (b) leaf semantic target/revisions sealed in approval; and (c) any route-binding generation that changes effect-relevant interpretation. Specify whether a purely unrelated gateway snapshot change invalidates the approval or only forces describe/preparation refresh and deliberate resubmission. Either chosen rule must be explicit; no silent auto-replay or target replacement is allowed. Relevant leaf/connection/config changes always invalidate the former subject.\n\nThe gateway selects one reviewed mapping and must use that mapping for the entire invocation. The leaf verifies its own current target and the trusted forwarding context; the signed approval is forwarded unchanged, not rewritten by the gateway to fit a new leaf. A gateway policy narrowing/route revocation observed before its dispatch decision refuses; no claim of atomicity with changes after actual forward is implied.\n\n**Required observation:** prepare at G14/L9, then update G or L independently. The trace states the exact refusal/refresh behavior. Repointing the same alias to another leaf or connection never reuses approval or a live idempotency fingerprint as unchanged authority."
  },
  {
    "file": "ess/domains/idempotency.yaml",
    "category": "legacy-review",
    "severity": "blocker",
    "message": "### F03-A04 — P1: Original actor/origin cannot be derived from the broad downstream credential\n\nThe executing leaf must distinguish authenticated gateway peer, originating admitted principal, optional realm/tenant and executor. A gateway token alone identifies only the configured service principal. An issuer-signed approval also does not replace current host grants or provider restrictions.\n\n**Required decision:** specify the receiver-owned trust/configuration that permits exactly this gateway to attest exactly the narrowed actor/origin context for this leaf/audience/route. Bind that verified context to the selected request, and re-evaluate leaf admission and provider restrictions. A body executor remains an assertion until verified. Direct origin and federated origin are tagged and distinct; another gateway origin must not reuse the approval, even if it forwards the same leaf operation/body. Key rotation preserves stable configured peer/origin identity rather than changing namespaces with token bytes.\n\n**Concrete ESS gap:** `connectors.idempotency.AuthorityScope.executor` is currently a required `String` (`ess/domains/idempotency.yaml:11`–`:18`), while service authentication/admission allows no executor. F03 must choose explicit absence semantics. Do not synthesize an empty string, collapse it into principal, or manufacture an entity merely to fill the type. Preserve absent realm versus `default` and absent tenant versus wildcard.\n\nThe old issued-reference store proves authority by trusted lookup, whereas the new proposal sketches issuer-signed evidence. Choose the new proof source and verifier trust/key rules explicitly; matching a caller-supplied issuer string is not sufficient. Issuer authority, original caller, executor and gateway peer remain distinct facts.\n\n**Required observation:** gateway admits Alice narrowly; leaf retains Alice's exact authority and checks its own policy. A forged actor, a different origin/receiver, an executor mismatch, realm absence substitution or an out-of-scope connection is refused before new business dispatch. Policy outage cannot fall back to static-bearer admission.",
    "line": 11
  },
  {
    "file": ".engineering/planning/review-result/federation-subjects-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### F03-A05 — P1: One approval must have one durable redemption authority, independent of route replicas\n\nThe gateway and leaf both run host machinery, but only one authority may spend the approval. Spending once at each hop either rejects the legitimate request or creates two independent chances to execute. A gateway must not spend approval before forwarding and then ask the leaf to trust an unbound assertion that it was spent.\n\n**Required decision:** select one logical redemption authority keyed by the exact configured issuer/reference domain and bound to the executing attempt. Recommended ownership: the executing leaf is the sole verifier/redemption coordinator for business execution; the gateway may precheck but does not spend. A backend may be external or replicated, but every replica for that logical leaf must observe the same serialized durable claim authority. Do not key spend uniqueness by request ID, alias, current gateway revision, connection pool or transient process identity. Do not add gateway origin to a spend key in a way that lets the same approval be consumed independently through another ingress.\n\nReceiver/target binding prevents a proof for leaf L from authorizing L2, regardless of independent physical stores. Cutover/failover cannot clone the same stable receiver identity with an empty redemption ledger. A lost/ambiguous spend acknowledgement does not grant dispatch and never restores a spent approval. Retention must prevent resurrecting valid approval authority after compaction; custody cleanup is unrelated.\n\nThis must preserve F01 ordering: prepare the durable business attempt, spend required authority, then separately win Prepared→Dispatching. Spending is not evidence of send and is not the dispatch fence. Gateway audit is distinct from the leaf attempt and does not create a second business ledger owner.\n\n**Required observation:** two concurrent gateway replicas present the same proof to the leaf; at most one spend and one dispatch gate succeed. Repeat after gateway/leaf restart, ambiguous spend acknowledgement and route cutover. A failed or fenced attempt may leave approval spent, but cannot falsely claim it reusable."
  },
  {
    "file": ".engineering/planning/review-result/federation-subjects-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### F03-A06 — P1: Transport anti-replay and keyed outcome replay must not invalidate one another\n\nDelegation needs a bounded replay/expiry rule, while F02 explicitly permits an independently admitted exact key replay without re-verifying live original approval or spending it again. A leaf verifier that rejects spent/expired approval before checking an admitted existing key would regress the settled replay semantics.\n\n**Required decision:** distinguish (1) authenticated delivery/proof replay protection, (2) approval one-time spending for a new business attempt, and (3) the receiver's key reservation and result replay. A fresh intentional client observation can use a new authenticated delegation delivery while carrying the unchanged original approval field; after current result admission, an exact live key observes the original rather than requiring that old approval to remain usable. Invalid current transport authority still refuses before result disclosure. Reusing an old delivery nonce is not a new request, and a transport refusal says nothing about whether the original business attempt ran.\n\nPin the actual forwarded request body—including key/approval/executor semantics—and bind method/path/receiver/selected route. State how describe or preparation with no invocation body is correlated and replay-limited. Bind the purpose so a proof for describe/preparation cannot authorize business invoke. Do not automatically mint another delivery and resend after a lost mutation response.\n\n**Required observation:** lost original reply followed by a deliberate same-key observation with fresh delegated delivery and expired/spent original approval returns the admitted durable outcome without new dispatch/spend. Changed key fingerprint conflicts; an unauthorized observer sees no original existence/result; a repeated signed delivery is refused without implying the original was not attempted."
  },
  {
    "file": ".engineering/planning/review-result/federation-subjects-r1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "### F03-A07 — P1: Gateway failure must preserve the leaf's attempt identity and effect knowledge\n\nThe gateway's own timeout/cancellation/audit state cannot settle the leaf's business attempt. It may lose a downstream answer after the leaf committed success, or fail before sending anything; these have different evidence but neither gives the gateway permission to resend elsewhere.\n\n**Required decision:** preserve original leaf attempt/request identity and its mutation observation through the gateway, with separate outer request correlation and separate source_audit. Explain request-ID mapping if the gateway creates a new leaf request ID; it must not redefine the approval/idempotency subject. When the gateway lacks a valid definitive leaf observation after possible forward, report unknown without inventing a leaf attempt ref. Never manufacture not_attempted from a cancelled local future or missing leaf audit.\n\nOnly the leaf's atomic pre-dispatch fence can prove that an anchored leaf attempt did not dispatch. A pre-forward gateway refusal can truthfully describe that refused observation/candidate, but must not assert that an existing same-key original never ran. Both hops retain bounded deadlines; the gateway cannot silently reset a new leaf execution budget after waiting.\n\n**Required observation:** known leaf success plus gateway final-audit failure preserves applied; dropped downstream response becomes unknown with no fabricated identity; duplicate-waiter cancellation does not cancel the original. A stale leaf response may refresh route metadata but never triggers automatic reinvocation."
  },
  {
    "file": "ess/domains/mutations.yaml",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### F03-A08 — P2: ESS should type the chosen values without inventing an ownership graph\n\nCurrent models deliberately leave operation qualification, ApprovalRedemption and authority/connection ownership unresolved (`ess/domains/mutations.yaml:42`–`:55`; service_wire final comments). F03 is the point to replace only the uncertainties actually settled by its contract.\n\n**Recommended modeling boundary:** type the canonical approval subject, resolved target, origin/actor authority with explicit executor absence, separate routing snapshot, prepared-subject result and verified delivery/correlation values. Reuse existing EffectKnowledge and AttemptId. Reconcile the existing idempotency namespace/fingerprint representation rather than maintain two differently qualified copies. Public preparation is a value, not a durable reservation entity.\n\nIf F03 introduces a durable redemption receipt, give it the selected issuer/reference identity and exact attempt reference only after ownership/lifetime are stated. A receipt that must survive route configuration or replay-cache deletion must not be modeled as owned by either. Issuer, tenant, principal, GrantRecord and Connection entity relations stay UNMAPPED where their identities/lifecycles remain owned by other stories. Do not fabricate one-to-one grant/audit relationships or runtime OperationDeclaration ownership from a gateway alias.\n\nESS structural validation cannot prove signature validity, canonical byte equality, authenticated actor derivation, global spend uniqueness, distributed fencing, clock/replay bounds, route snapshot atomicity or actual send counts. Record those as executable obligations with exact positive/adversarial vectors and a separate manual semantic trace. Do not turn a compiled scenario or a string-valued field into proof of authority.",
    "line": 42
  }
]
```

