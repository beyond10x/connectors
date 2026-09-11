---
format: aep.planning-md/1
id: verification-report:findings-mutation-profiles-a-initial-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:mutation-profiles-a-initial-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 857f17b4a5121236cd2fd034e8094931542bbb3c30b86c9f875f826cce883181
relations:
- verifies: review-result:mutation-profiles-a-initial-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:mutation-profiles-a-initial-20260908

This supplements [the immutable original](../review-result/mutation-profiles-a-initial-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

The original report's verdict and scope remain authoritative; no new verdict is assigned.

## Transcription method

7 findings remain in the scope of this report's final stated conclusion.
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
    "file": "contracts/operations/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### MP-A-01 — P2 — SIP's executable effects fail the shared mutation discriminator\n\n**Sole correction owner:** `story:contracts-mutation-classification` (F10/E06).\n\nOperations §3 makes external_write the discriminator and §8 requires its presence for profile:mutation (`contracts/operations/v1alpha1/semantics.md:51,217`). SIP declares mutation with only session_establishment/send_external, and two examples additionally put human_visible in executable effects (`docs/adapters/media-session.md:17,37,63`). That descriptor would fail its own selected validator obligation.\n\nChoose one normative classifier and reconcile all examples and session-establishment references. A minimal compatible choice is external_write plus the applicable session_establishment/send_external/network effects, with human_visible confined to semantic_effects. If the owner instead permits another executable effect to independently select mutation, explicitly define the entire qualifying set and its converse validation rules so an effectful declaration cannot evade the mutation gate by omitting external_write. A network read or descriptive human visibility alone must not acquire mutation authority. Effects and semantic_effects remain metadata, never substitute grants, approval, the attempt ledger or dispatch permission. Unsupported/unknown/incomplete combinations fail before advertisement/execution.\n\nPinned predecessor `providers/b10x.toml:741–760` actually used effects=[write,network], semantic_effects=[send_external,human_visible] and interaction_shape=session_establishment. The v2 document deliberately remaps that vocabulary; do not describe its inconsistent present examples as an exact preserved declaration. Existing adapter-kind schemas and ESS Declaration strings do not execute the proposed effect discriminator today.",
    "line": 51
  },
  {
    "file": "docs/adapters/media-session.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### MP-A-02 — P2 — Define SIP effect knowledge independently of ready-session publication\n\n**Sole correction owner:** `story:contracts-mutation-classification` (F10/E06).\n\n`sessions/v1alpha1/semantics.md:65–87` requires every stream/application binding to be ready before returning a session ref, and a terminal before readiness produces error/no ref. `docs/adapters/media-session.md:37` correctly says dialing may ring a real endpoint, but neither owner maps partially completed establishment onto the F01/F03 effect observation. This leaves a dangerous inference that no ready session means no external effect.\n\nDefine the operation-specific definitive success/effect evidence and distinguish it from successful session handoff and eventual call duration/completion. Trace at least pre-dispatch refusal, definite remote refusal proving no effect, possible SIP dispatch with lost response, confirmed call effect followed by application-binding/readiness failure, and success followed by later session termination. A lost/missing session ref, failed application channel, cancellation or teardown cannot prove that the call never rang or undo an earlier known effect. Preserve known applied knowledge where proved, and otherwise report outcome_unknown rather than a false not_attempted/refused. An outer error can carry applied under compatibility §5; it must not fabricate a ready transport handle. A later lost session does not retroactively rewrite the operation's settled ledger outcome or authorize redial. Keep the existing non-idempotent/no-automatic-resend and session cutoff rules.\n\nSources: `operations/...:99–130`; `service/compatibility.md:92–106`; `sessions/...:83–91`; `docs/design.md:375`; old `providers/b10x.toml:741–774` explicitly separates externally visible establishment from the established response.",
    "line": 37
  },
  {
    "file": "docs/adapters/docker.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### MP-A-03 — P2 — Docker lifecycle operations need individual, bounded idempotency claims\n\n**Sole correction owner:** `story:contracts-restart-idempotency` (F11/E08).\n\n`docs/adapters/docker.md:25` calls start/stop/restart natural, while its operation table gives restart none at line 47. All endpoints and provider assertions remain marked unverified at lines 11 and 36. The single running-container no-op example does not justify every listed operation or every repeated-result claim.\n\nSelect one classification per operation and make summary, table and scenarios agree. For start and stop, if natural is selected, cite a pinned provider version and define repeated desired-state behavior for the same exact container and parameters, including already-running/already-stopped replies and any intervening state changes. Natural does not imply identical HTTP replies, a historical host replay record, never another external transition, or permission to retry an uncertain invocation. Define whether success means the desired lifecycle state/request acceptance rather than healthy application completion. Restart must not inherit start's no-op justification; state its repeat behavior separately. Handle missing container, refusal, response loss and timeout conservatively under the shared effect table. A caller-requested repeat remains a newly admitted attempt where no keyed host reservation governs it.\n\nCurrent vendor facts require the parent's Connectors-first/official-source verification; this initial review does not promote the baseline's unpinned endpoint expectations into vendor evidence.",
    "line": 25
  },
  {
    "file": "docs/adapters/kubernetes.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### MP-A-04 — P2 — Preserve Kubernetes UID/resourceVersion semantics before selecting idempotency\n\n**Sole correction owner:** `story:contracts-restart-idempotency` (F11/E08).\n\n`docs/adapters/kubernetes.md:55` labels rollout restart natural merely because it writes an annotation timestamp. That neither proves natural idempotency nor proves the opposite claim that every repeated dispatch necessarily triggers another rollout.\n\nThe pinned predecessor's `RestartInput` requires namespace/name/**Deployment UID**/resource_version (`old crates/integration-kubernetes/src/workloads.rs:304–310,1197–1234`). The UID is provider resource identity, not Invocation.request_id or a business idempotency key. `local_workloads.rs:297–350` sends those metadata preconditions with a newly generated restartedAt annotation in a strategic merge PATCH, verifies response namespace/name/UID/nonempty version, and reports patch_accepted. Its error mapper at lines 108–124 distinguishes explicit API refusals/stale state from unknown post-dispatch outcomes. That is exact source evidence, not proof that every provider response or version combination has a particular outcome.\n\nSelect stable request intent, all required preconditions, timestamp construction/fixity for one prepared attempt, success meaning and one idempotency classification. Preserve the distinction between a replay of the same host attempt, a new candidate with the same original UID/version, and deliberate new intent with a fresh version or different UID. After a lost response, original preconditions may prevent a second write if the first write advanced the version; a resulting conflict does not prove whether the original succeeded or some other writer changed the object. Do not drop/refresh preconditions or regenerate/retry automatically to make it succeed. If provider no-op/version behavior matters, qualify that case explicitly instead of asserting unconditional conflict/rollout. A changed precondition changes canonical input/F02 fingerprint and needs deliberate admission/approval; same live key with changed input conflicts. Success is accepted patch, not completed/healthy rollout. No follow-up polling/probe is implicit in the mutation budget.\n\nA keyed host classification is compatible with preserving the provider's optimistic concurrency guard, but does not by itself make the provider operation naturally idempotent. Any other selection needs equally explicit repeated-request semantics. Do not copy the old broad 409/422 mapping as universal proof of non-effect without verifying the selected provider response meaning.",
    "line": 55
  },
  {
    "file": "docs/adapters/docker.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### MP-A-05 — P2 — Docker mutable names are insufficient as stable lifecycle targets\n\n**Sole correction owner:** `story:contracts-restart-idempotency` (F11/E08).\n\nDocker permits id **or name** and name/label allowlists (`docs/adapters/docker.md:49,68`) without selecting how the target is stabilized for an approved mutation or a natural repeat claim. A name can later resolve to another container; the same textual input then refers to a different effect target. F02/F03 require effect-relevant bindings to remain exact (`operations/...:150`; `service/delegation.md:59–64`).\n\nSelect a stable daemon-qualified full container identity for lifecycle mutation intent, or a specified admitted name-to-identity resolution/binding that is included in approved canonical intent and rechecked before dispatch. Prefix/name/label selection must not silently repoint a pending attempt or a deliberate repeated desired-state request. If discovery/inspection is needed to obtain that identity, it is a separately admitted bounded read; F03 approval.prepare is a safe host metadata read and cannot invent provider lookup. Preserve timeout/signal and other effect-relevant lifecycle parameters in intent/configuration meaning. A simpler first profile can require an exact previously observed full ID for writes while retaining convenient names for independently scoped reads. Unknown/changed identity refuses rather than dispatching against a guessed successor.",
    "line": 49
  },
  {
    "file": ".engineering/planning/review-result/mutation-profiles-a-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "### MP-A-06 — P2 — `none` overstates effect occurrence and blurs replay identity\n\n**Sole correction owner:** `story:contracts-restart-idempotency` (F11/E08).\n\nThe shared `idempotency.kind` table defines none as “every dispatch is a new effect” (`operations/...:54`). Even a non-deduplicated request can be refused, fail before a business effect, or find an already-satisfied state. The same owner correctly distinguishes not_attempted/refused/unknown/applied and defines a fresh none/natural invocation as a new **attempt** (`operations/...:136`).\n\nDefine none as no selected repeat-effect/deduplication guarantee, not proof that every request changed the world. Define natural in terms of the exact operation's repeat semantics and assumptions; define keyed as the receiving host's namespace/fingerprint/reservation/replay contract. Correlation request_id, resource UID, provider resourceVersion, generated timestamp, provider lifecycle state and host idempotency key are separate coordinates. None and natural still use the mutation ledger/approval/dispatch fence; neither creates a host replay record or permits an automatic resend. Returned no-op success needs an explicit operation success meaning rather than manufacturing a physical change. Existing key expiry, quarantine, changed-input conflict, current replay authority and spent-approval rules remain unchanged."
  },
  {
    "file": "docs/adapters/atlassian.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### MP-A-07 — P2 — Select one complete visibility/refusal matrix without rewriting legacy behavior\n\n**Sole correction owner:** `story:contracts-mutation-visibility` (E12).\n\nCurrent sources provide pieces, not the requested complete matrix: implemented-only advertisement (`service/v1alpha1/...:20`); enabled-operation description (`...:27–32`); extended implemented+enabled+policy-admitted projection (`service/v1alpha2/...:72`); disabled mutation gives Forbidden (`operations/...:93`); Atlassian writes disabled unless listed (`docs/adapters/atlassian.md:111–117`). Docker explicitly says it advertises only enabled operations (`docker.md:60`). Hidden configuration and caller policy therefore must be distinguished from implementation absence and readiness without treating descriptor absence as one universal lookup outcome.\n\nSelect discovery and invocation results for unimplemented/unbound ID, implemented-disabled ID, enabled but unauthorized caller, enabled/authorized but not-ready connection, and enabled/authorized/ready operation. State which input conditions are assumed—valid framing/authentication, fresh supported descriptor revision, current disclosure/operation policy—so simultaneous stale/disabled/denied states have a deterministic safe precedence. Preserve not_granted for current host-policy denial, forbidden for an admitted configuration/resource refusal, and no mutation/result/key existence disclosure when current observation authority fails. Describe-time visibility never grants invocation authority. Readiness failure must not hide otherwise useful implemented metadata or block the separately admitted management repair surface.\n\nIf selected extended discovery hides disabled operations while an admitted guessed disabled ID gives forbidden, explain that future invocation resolves the receiver's implemented registry/configuration before dispatch; it does not dispatch by merely looking up the public projection or expose private metadata to unauthorized callers. No disabled descriptor field is required solely to explain that policy. Keep configuration/authority changes bound to descriptor freshness and deliberate resubmission.\n\nCurrent code is narrower: `crates/connectors-core/src/lib.rs:81–86` looks up only the descriptor vector and gives NotFound for absence. Host `server.rs:66–70,109–138` authenticates, checks the exact descriptor revision, then uses that lookup before invoking. It has no implemented extended mutation registry, curation fields or current per-person grant decision. The explicit future legacy projection separately requires guessed hidden mutations to return not_found (`service/compatibility.md:43–45`); its own revision is not the full descriptor revision. Document these scopes rather than changing legacy facts to make the extended matrix appear already implemented. Crosslink Atlassian/Docker enablement examples to the selected owner matrix.",
    "line": 111
  }
]
```

