# operations/v1alpha1 — `mutation` profile

- **Status:** proposed; mutation runtime and wire binding are not implemented. The host attempt lifecycle is modeled in [ESS](../../../ess/domains/mutations.yaml), with [authored scenarios](scenarios) and an explicit [verification boundary](verification.md).
- **Base contract:** `operations/v1alpha1` as implemented in [service v1alpha1](../../service/v1alpha1/semantics.md) (describe/invoke wire boundary, error envelope, limits, stale-description refusal). This document adds one profile and does not restate the base.
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `operations/v1alpha1` |
| Profile | `mutation` |
| Existing profiles | read profiles declared per adapter (`kubernetes-list`, `gitlab-*`, `postgresql-native-text`), all promising "no external business mutation" (`contracts/service/v1alpha1/semantics.md`, Wire boundary) |
| Relation | new semantic profile: an operation declares `profile: mutation`; public fields require the selected extended service binding |

A mutation is an operation whose dispatch can change external state. The profile exists because the base wire contract explicitly promises no business mutation, no retry, and no idempotency; those promises cannot cover Jira issue creation, a Kubernetes rollout restart, or a SIP dial.

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| Operation metadata `direction`, `risk`, `idempotency`, `effects`, `semantic_effects` | `../connectors/providers/b10x.toml:741-752` (`sip-dial`), `../connectors/providers/jira.toml:369-376` (`jira-issue-create`, `effects = ["write","network"]`) | preserve as typed descriptor fields (`effects`, `idempotency`, `risk`); drop `direction` (redundant with effects) |
| Approval verification against issuer, subject, operation, Connection, canonical input digest, expiry; refusal names no axis | `../connectors/crates/domain/src/approval.rs:1-10` | preserve |
| One-time redemption as a bounded append whose payload is the attempted-audit row; anchor before claim, claim before dispatch | `approval.rs:12-35` | preserve the ordering guarantee; the storage mechanism (keyed byte cells) is not preserved (`docs/design.md:961`) |
| Recovery scan classifying anchored attempts as `indeterminate` or `aborted` | `approval.rs:37-44` | preserve |
| Local event-triggered one-time `event:` claims at the dispatch seam | `../connectors/crates/connectors-runtime/src/claims.rs:1-15` | change: express as a second admission profile (`event-claim`) of the same approval binding, not a separate mechanism (`docs/design.md:1093`) |
| Hosted grant evaluation as a separate policy from approval | `docs/design.md:78` | preserve the separation; grant evaluation is host policy outside this contract |
| Automatic retries, rate-limit-triggered resend | `docs/design.md:377` | remove: no automatic retry of a mutation under any outcome |

## 3. Types

Operation curation in the proposed [extended service binding](../../service/compatibility.md#4-extended-descriptor-and-invocation-surface). These fields are required there, including for reads. The following is a descriptor excerpt; description and advertised limits are omitted. Existing legacy reads serialize unchanged only through the verified projection in §7:

```json
{
  "id": "issue.create",
  "contract": "operations/v1alpha1",
  "profile": "mutation",
  "effects": ["external_write", "network"],
  "semantic_effects": ["human_visible"],
  "risk": "medium",
  "idempotency": { "kind": "keyed", "key": "caller_supplied", "retention_seconds": 86400 },
  "approval": "required",
  "input_schema": {},
  "output_schema": {}
}
```

| Field | Values | Meaning |
|---|---|---|
| `effects` | `external_write`, `network`, `process`, `local_system`, `send_external`, `session_establishment` | executable effect categories; `external_write` is the mandatory discriminator for this profile, including externally observable call initiation |
| `semantic_effects` | `human_visible`, `billable`, `irreversible` | admission and UX hints; never authority |
| `risk` | `low`, `medium`, `high` | admission hint |
| `idempotency.kind` | `none`, `keyed`, `natural` | `none`: no selected repeat-effect or deduplication guarantee; `keyed`: receiver-owned scoped reservation/replay under §5.1; `natural`: the exact provider operation has the documented repeat semantics and assumptions in §5.2, without host replay storage |
| `approval` | `not_required`, `required`, `event_claim` | which admission profile must be satisfied before dispatch |

**Effect declaration rule (F10/E06).** `external_write` covers possible external **business** changes, including messages, call initiation and session establishment; it does not require a persisted database write. Every such operation must declare `profile: mutation` and include `external_write`. Conversely, `external_write` is incompatible with a read profile. Describing an effectful implementation as a read or omitting the discriminator is an invalid declaration, not a way to avoid mutation admission.

`network` describes transport use and alone does not make an operation a mutation. A bounded provider query can remain a read. `send_external` means intentional outbound business communication, not every network packet, and `session_establishment` denotes business session establishment when carried by an operation declaration; either requires the mutation profile and `external_write`. Protocol-internal authentication, session control and private transport setup remain governed by their own admitted contracts; they do not become new public business operations because they use sockets or allocate tasks. `process`/`local_system` must be assessed against the actual selected profile: a business change still requires the discriminator, while these labels alone grant no mutation capability.

Both effect arrays are required by the extended descriptor, contain distinct members, and use their respective closed vocabularies. A selected operation binding must support every declared executable effect and declare every applicable category. The receiver refuses unknown, duplicate, unsupported, incomplete or incompatible declarations before advertisement or dispatch. `human_visible`, `billable` and `irreversible` belong only in `semantic_effects`; they cannot satisfy the discriminator or grant caller authority. Hints may tighten policy or presentation, never waive current grants, approval, the attempt ledger or its dispatch fence. The selected SIP set is `[external_write, network, send_external, session_establishment]` with `semantic_effects: [human_visible]`.

These are future declaration-validator and receiver obligations. The current adapter-kind schema and `OperationDeclaration` entity do not implement them. The private ESS effect values in §9 check enum/field shapes only; a shape-valid read declaration containing `external_write` must still fail the semantic rule.

Illustrative invocation on `POST /v1alpha2/invoke`; the implemented legacy reader refuses these additions. The [compatibility owner](../../service/compatibility.md) defines the selected codec and routes.

```json
{
  "version": "v1alpha2",
  "request_id": "…",
  "operation": "issue.create",
  "revision": "<descriptor revision>",
  "input": {},
  "idempotency_key": "optional, required when idempotency.kind = keyed",
  "approval": { "reference": "opaque", "evidence": "opaque, issuer-signed" }
}
```

The proposed response carries a `mutation` observation alongside the ordinary result/error and required audit metadata. Its five members are required when the observation is disclosed: `classification`, nullable `attempt`, nullable `original_request_id`, `replayed`, and nullable `cause`. [The exact response rules and outcome table](../../service/compatibility.md#5-extended-responses-audit-and-mutation-observation) are authoritative. For example, an admitted replay of an applied result keeps classification `applied`, identifies the original attempt/request, and sets `replayed: true`; the outer request id identifies the current observation. Replay is not an effect classification. A denied observation omits mutation metadata and discloses no original outcome.

New error codes beyond `ErrorCode` (`crates/connectors-core/src/lib.rs:13-27`):

| Code | When |
|---|---|
| `approval_required` | operation declares `approval: required` and none was presented |
| `approval_refused` | presented approval failed verification; the message never names the failing axis |
| `approval_replayed` | the approval reference was already spent |
| `idempotency_conflict` | same namespace/key, different request fingerprint while its reservation remains live; disclose neither the stored request nor the differing axis |
| `outcome_unknown` | the relevant attempt may have dispatched and the observer lacks definitive outcome evidence |

An observation has two independent semantic dimensions: **classification** (`not_attempted`, `refused`, `applied`, `unknown`) describes knowledge of the business effect; **cause** describes why an error occurred (`Timeout`, `Unavailable`, an admission refusal, etc.). `not_attempted` is a classification, not another competing `Error.code`. For example, an unavailable attempt store before any dispatch permission gives cause `Unavailable` and classification `not_attempted`. It does not lose its cause to a second error code. These dimensions have a typed ESS home, `connectors.mutations.Observation`; it remains a private semantic value. The proposed public encoding is the distinct `connectors.service_wire.MutationObservation`, governed by the compatibility owner.

The mutation response code for an uncertain outcome is `outcome_unknown`; a transport timeout can remain its safe diagnostic cause, but must not replace it with a plain `Timeout` that implies non-dispatch. An HTTP status or successful transport exchange alone proves neither success nor refusal of the business effect. Read-profile error meanings are unchanged. `not_attempted` proves only that this business attempt did not dispatch: it does not restore a spent approval or promise that resubmission will be admitted.

## 4. Rules

Execution sequence (specializes `docs/design.md:362-371`):

1. Validate bounded transport framing, authentication, version, envelope syntax and syntactic identifiers (base contract). Operation-specific input shape and existence are not disclosed at this stage.
2. Resolve receiver-owned operation/connection candidates internally only as needed for current policy. This is not a publicly observable lookup: it must not disclose existence, schema or private resolution errors before admission.
3. Check current caller/target observation authority, description freshness, private bound lookup/enablement, then operation-specific input shape and its containment within the admitted operation/result scope, in the [extended precedence matrix](../../service/v1alpha2/semantics.md#321-visibility-lookup-and-refusal-precedence-e12). With admitted lookup and a current revision, a disabled mutation is `Forbidden`; an absent/unbound one is `NotFound`. Current denial precedes stale/existence disclosure, and freshness precedes the private disabled lookup. Input validation follows permitted fresh bound/enabled lookup and precedes key inspection in step 4. All such admission refusals omit mutation metadata and precede any replay result, conflict or original existence disclosure. Legacy projection separately returns not_found for a guessed hidden mutation.
4. If `idempotency.kind` is `keyed`, derive the trusted namespace and full fingerprint in §5.1 and inspect its reservation. A matching record is a replay/wait/quarantine observation, never a new dispatch. A conflicting record refuses. Replay and waiting require current access to the original result scope; they neither validate a new execution approval nor redeem the original one again.
5. Only a candidate for a new attempt verifies its required approval against the complete [F03 canonical subject](../../service/delegation.md#2-canonical-subject-and-revisions), including trusted caller/executor/origin, leaf operation/connection and semantic revisions, canonical input, and proof issuer/expiry, or verifies an admitted unspent event claim. A previously spent/expired approval cannot authorize a new attempt, including reuse after replay retention expires. Before returning a candidate's `approval_refused` after a cache miss, perform the authoritative winner recheck in §5.1; a concurrent exact replay does not require that claim to remain unspent.
6. Prepare the provider request without executing it; resolve credential readiness (`auth.evidence`). Before returning a new-attempt preflight refusal following a miss, perform the same winner recheck. These checks do not bypass a failure of current caller/result admission.
7. Durably create an `AttemptRecord` in `Prepared` (host-generated attempt id, instance, request id, operation, connection, input digest, approval mode/reference, idempotency key). For a keyed attempt, reserve the unique namespace/key and link its fingerprint to this attempt in the same atomic host transaction; recheck a concurrent winner through step 4 instead of creating a second attempt. No business dispatch is permitted in Prepared. If anchoring fails, this candidate has cause `Unavailable` and classification `not_attempted`; even an ambiguously acknowledged prepare cannot grant dispatch. If a winning original already exists, its classification follows §4's observation table instead.
8. When required, spend the approval or event claim exactly once. `approval: not_required` skips spending, not the attempt ledger. Failed or uncertain spending prevents opening the dispatch gate; atomically abort the prepared attempt. A spent approval remains spent even when the attempt aborts.
9. After admission, preflight and required spending succeed, atomically and durably compare-and-set `Prepared → Dispatching`. Only the live worker receiving definite success for this transition may invoke the connection-bound capability once. Merely reading `Dispatching` or receiving an ambiguous write acknowledgement grants no permission to send. Cancellation/recovery competes through `Prepared → Aborted`; a worker losing that race must never send. No timeout wrapper, restart worker or duplicate caller may reacquire dispatch permission.
10. Invoke the capability at most once, with no automatic resend. The durable gate precedes this call: `Dispatching` means **may have sent**, not proof that bytes reached the provider.
11. Record the definitive provider success as `Completed`, a definitive refusal proving no business effect as `Failed`, or the absence of definitive evidence as `Indeterminate`. A generic provider error, HTTP 5xx, malformed answer or partial result is not by itself a known refusal. An observed success follows the operation's declared success meaning, including a declared asynchronous acceptance; it must not promise eventual work completion.

The ledger port serializes these transitions and fences stale writers. Dispatch and abort cannot both win. A lost acknowledgement of opening the gate, or an unreadable ledger when the gate might have opened, leaves recovery uncertain; it cannot be treated as a failed write proving non-dispatch. Recovery never sends an anchored request. Approval spending is not the dispatch fence, and no cross-store transaction with the provider is claimed.

Canonical input digest: SHA-256 over the invocation input after schema validation, serialized with sorted keys and no insignificant whitespace, using the existing adapter v1 canonical-JSON rule (`spec-kinds/adapter/v1/semantics.md`, revision paragraph). Record that algorithm as `adapter-v1-canonical-json` in the fingerprint; do not silently change it. Equal input digests identify equal canonical input under that algorithm, **not** the complete authorized request. Approval still binds its other subjects; keyed replay additionally requires the namespace and every fingerprint coordinate in §5.1. A future canonicalization change needs a new identifier and cannot reuse a live key as if the request were unchanged.

Authority boundary: `effects`, `risk`, and `approval` are description metadata. They inform admission and UX; they authorize nothing (`docs/design.md:358`). The receiver decides admission from its own configuration and policy at execution time.

Outcome truthfulness: the classification is about the **identified business attempt**, not merely the transport call observing it. Apply this table independently of approval mode or spend status. A read of `Prepared` alone is not terminal proof: another worker could still open its gate.

| Evidence available to the observer | Ledger decision / state | Classification | Cause / response meaning |
|---|---|---|---|
| Validation, admission or preflight refuses before dispatch permission | no record, or fence `Prepared → Aborted` | `not_attempted` | preserve the specific refusal cause |
| Attempt anchoring unavailable, including ambiguous prepare acknowledgement | no gate can have opened | `not_attempted` | `Unavailable`; any surviving prepared row is later aborted |
| Service deadline/cancel wins the atomic abort before the gate opens | `Aborted` | `not_attempted` | `Timeout` / cancellation; late dispatcher must lose |
| Recovery finds `Prepared` and successfully fences it to `Aborted` | `Aborted` | `not_attempted` | recovery before dispatch permission, even if approval was spent |
| Gate opened, but crash happens before the capability call | `Dispatching → Indeterminate` | `unknown` | `outcome_unknown`; durable evidence cannot distinguish this from a sent request |
| Gate may have opened, acknowledgement/state cannot be established | leave unresolved; settle only when the ledger is readable | `unknown` | `outcome_unknown`, diagnostic cause `Unavailable` |
| Gate opened and response is lost, service deadline wins, or process crashes without terminal evidence | `Dispatching → Indeterminate` when storage permits | `unknown` | `outcome_unknown`, including a diagnostic `Timeout` |
| Definitive provider success | `Completed` when recorded | `applied` | success according to the operation's declared semantics |
| Definitive provider refusal with evidence that no business effect occurred | `Failed` when recorded | `refused` | preserve the safe provider refusal cause |
| Provider answer does not establish effect outcome, including ambiguous 5xx or partial work | `Dispatching → Indeterminate` | `unknown` | `outcome_unknown`; a failure status alone is insufficient |
| Terminal record already durable and replay admitted under §5.1 | preserve `Completed` / `Failed` / `Aborted` / `Indeterminate` | `applied` / `refused` / `not_attempted` / `unknown` respectively | replay preserves the original classification |
| Duplicate waiter expires with no terminal observation of the original attempt | original state unchanged | `unknown` | `outcome_unknown`; the waiter must not claim the original never ran |

The live observer can know more than recovery. If it received a definitive provider answer but terminal persistence fails, it must preserve the known `applied` or `refused` classification in any response it can deliver, and record/report the ledger failure separately through host diagnostics. It must not manufacture provider uncertainty or claim the result is durably replayable. Recovery without that answer still uses `unknown`. Loss of the response to the caller likewise leaves the caller uncertain even if the host recorded a terminal result. A binding must test all three viewpoints.

Recovery uses durable dispatch state and outcome evidence, never “spent means sent” or “unspent means unsent.” An old `attempted` record without the new fence is `Indeterminate` unless separate durable evidence proves non-dispatch and fences all possible dispatchers; this applies equally to `not_required`. A crash after spending but before the new gate is provably pre-dispatch after a successful abort; a crash after the gate but before send is indistinguishable from a crash after send. The four terminal states record the first settled ledger observation and never authorize retry. Late evidence after `Indeterminate` may be retained as diagnostics; changing the terminal classification requires a separately specified reconciliation mechanism, absent from this first profile.

Cancellation: report an aborted business attempt only after the atomic pre-gate abort succeeds. If the gate has opened, the cancellation acknowledgement means `received` only. It neither proves provider termination nor rolls back an external effect. A duplicate waiter's cancellation/deadline ends that wait; it cannot cancel or abort the original attempt.

### 4.1 Session-establishing mutation outcomes

Each binding must state its definitive business-effect evidence separately from the conditions for publishing a ready session result. Native protocol evidence and its exact success/no-effect interpretation belong to the adapter; a transport success or allocated local resource alone cannot establish the selected commitment. All declared streams and application bindings must additionally be ready before publishing a ready-session result. The [SIP dial contract](../../../adapters/sip/contracts/dial/v1alpha1/semantics.md#41-dial-effect-and-ready-session-result) is one concrete binding, with its own native effect definition.

If the SIP effect is proved but application binding, media negotiation or safe result delivery fails, retain `applied` and return an ordinary safe error without a fabricated ready handle. This is the compatibility owner's existing applied-plus-error case. Ringing or other partial work without definitive establishment/no-effect evidence is `unknown` with `outcome_unknown`, even when the observer knows that a narrower external effect occurred. A session-level `offer_rejected`, terminal reason, cancellation or missing handle alone proves neither `refused` nor `not_attempted`; use §4's dispatch/effect evidence table. A definite no-effect refusal must cover the entire attempted dial, including any provisional outreach; a final refusal status alone cannot prove that the target never rang.

Session readiness and terminal decisions remain serialized by the supervising owner. Full readiness emits one immutable ready receipt; a terminal that wins first prevents that receipt. A terminal after the ready decision cuts off/tears down the session under the session contract and cannot retract the original receipt or known dial effect. A receipt is historical establishment evidence, not a lease, a promise of future liveness or permission to disclose expired one-use authority. Current result admission and safe delivery still apply. If a terminal is observed before result encoding, or a successful result can no longer safely be delivered, report `applied` with a safe delivery error and no usable handle. Losing the response leaves the caller uncertain even if the live host knew `applied`; recovery without that evidence follows §4. No case authorizes automatic redial or revives a spent approval.

## 5. Ordering, retry, limits

| Concern | Rule |
|---|---|
| Retry | never automatic. A caller retry with a live scoped key observes its reservation; it cannot re-dispatch. After a known-result reservation expires, §5.1 permits a freshly admitted new attempt. A keyed operation refuses a missing key; for `none`/`natural`, a new invocation is a new attempt. `retry_after_seconds` on `RateLimited` is advisory and never triggers mutation resend |
| Concurrency | two admitted invocations with the same namespace/key and fingerprint: at most one obtains dispatch permission. The other waits for the original's terminal record; on waiter deadline it returns that outcome only if observed and still authorized, otherwise unknown or an admission refusal as §5.1 requires. A deadline alone is `outcome_unknown` with diagnostic cause `Timeout`, never `Capacity`/`not_attempted` for a possibly dispatched original. The waiter sends no additional business request |
| Ordering | none across operations; a session-establishing mutation follows `sessions/v1alpha1` and returns a session reference only for a safely deliverable ready result (§4.1); its business effect can be applied even when that result fails |
| Limits | non-generic request 64 KiB and 20 s service deadline; `realization: generic` selects 256 KiB and 40 s execution / 30 s provider / 5 s connect, including generic mutations, as defined by [service compatibility](../../service/compatibility.md#7-limits-and-compatibility-obligations); known-result replay retention is 86,400 s from durable terminal settlement, not a timeout for pending/unknown reservations (§5.1) |
| Attempt record size | bounded; the input digest is stored, not the input |

### 5.1 Key namespace, fingerprint and replay admission

The receiving mutation host owns the keyed ledger and its durable unique index; the adapter and provider do not define its namespace. Persistence of a reservation and its Prepared attempt is one atomic responsibility. An acknowledged reservation is bound to exactly one immutable attempt id; the attempt's audit evidence is independent of replay-cache deletion. Recovery never replaces an unresolved reservation with a new attempt. ESS records this as `connectors.idempotency.KeyReservation` **referencing**, not owning, `AttemptRecord`.

**Namespace:** `(receiver_instance, admitted_authority, trusted_origin)`. `admitted_authority` includes tenant (or configured local absence), optional realm, stable caller identity and executor identity. An absent realm differs from `default`; absent tenant is not a wildcard. Direct origin is tagged `direct` with the receiver's configured identity. Federated origin is tagged `federated` with the authenticated gateway/origin authority identity, independently of the resolved receiver. Credential bytes, credential rotation, connection-pool handles and caller-supplied headers never define these identities. Request input cannot choose tenant, realm, caller, executor or origin. The [F03 one-hop binding](../../service/delegation.md) supplies exact trusted context and canonical leaf subjects; until it is implemented, a broad gateway credential cannot establish individual caller/origin authority and keyed federation remains unavailable. Executor absence is explicit null, never a synthetic identity.

The index key is `(namespace, caller_key)`. `caller_key` is a required nonempty opaque string for keyed operations, compared exactly without trimming, case folding or Unicode normalization. It remains subject to the selected operation’s advertised request bound. Store structured fields or use an injective, versioned encoding: `mutation-key/v1` is a canonical JSON array of tag, receiver, tenant, realm, caller, executor, origin tag, origin authority, and key, in that order; preserve JSON null for absence. Delimiter concatenation and a lossy hash-only index are insufficient. A hash may index the tuple only if equality is checked against the original tuple.

**Fingerprint:** resolved source-qualified operation reference, stable connection reference and its admitted metadata revision, contract/version and profile, descriptor revision, active host configuration revision, canonicalization identifier and canonical input digest, plus required nullable F03 route binding (gateway instance, stable route id and semantic route revision; null for direct). Store this extended fingerprint under `mutation-request/v2`; if hashed, compare the stored coordinates as well. Operation/connection/revision changes are **not namespace changes**: while a reservation is live, reusing its key for any different fingerprint is a safe, axis-free `idempotency_conflict`, with no result disclosure or additional dispatch. Request correlation id, gateway presentation-only descriptor revision, deadline, key itself, approval token/expiry and rotating credential bytes are excluded. The earlier proposed mutation-request/v1 has no runtime/persisted binding here; a future reader must nevertheless distinguish it and refuse automatic reinterpretation as v2. A new fingerprint version cannot silently reuse a live old-version key as unchanged authority. Metadata/configuration revisions must change when a route, external identity, destination, resource interpretation or operation meaning changes. A same-identity credential refresh alone is not a semantic revision. The separate private auth publication fence may advance and invalidate dispatch evidence without changing this fingerprint; safe same-target route evidence renewal is likewise distinct from a semantic route revision (design §31.2). An implementation unable to detect an effect-relevant binding change must refuse keyed dispatch; silently preserving the fingerprint is not allowed.

**Admission and approval:** before lookup disclosure and again before returning a replay or completing a wait, the host checks current caller authority, operation enablement and connection/resource/result visibility. A cache hit is not authorization. Replays preserve the original payload/classification and original attempt correlation; `replayed` is delivery metadata, not a replacement classification and never a claim that an original refusal/unknown was success. Current credentials need not be refreshed merely to return an already-stored result: current host authorization to observe that result must hold. Changed access policy takes effect even if the descriptor fingerprint is unchanged. Approval/event-claim evidence authorizes a new dispatch, so an exact admitted replay neither requires still-live original evidence nor spends another claim. New execution, including key reuse after expiry, must satisfy the current approval requirement independently. Final access checks and response delivery use the host's admitted policy snapshot; a revocation observed before that decision wins. They do not claim atomicity with a later policy change.

**Miss/refusal race:** a cache miss is not a reservation. Before returning an execution-approval or preflight refusal for a candidate that previously missed, recheck the authoritative namespace/key index under current disclosure admission. An exact live winner is observed through its normal wait/replay/quarantine path; a different live fingerprint gives the safe conflict; only confirmed absence of a governing live reservation permits the candidate's original axis-free refusal. That authoritative observation is the refusal decision's serialization point: a later reservation is a later decision, not a reason to spin indefinitely. The recheck cannot use a stale cache or treat an unavailable read as absence. If it cannot establish whether a winner governs, report conservative outcome uncertainty with the safe ledger cause, without disclosing a record. Current admission refusal always wins and cannot be rescued by a cache hit. This ordering covers the interleaving where A reserves and spends a shared claim after B's miss but before B verifies it; B must observe admitted A rather than refuse solely because A spent the claim.

| Boundary / observed state | Decision |
|---|---|
| Same key/body, different admitted caller, executor, tenant, realm, receiver instance or trusted origin | Separate namespace; evaluate as a new independently admitted candidate. Never return or suppress work using the other namespace's record |
| Same namespace/key, different operation, connection, input or any fingerprint revision | Conflict while the reservation is live, including Pending and Quarantined; no other request/result details disclosed |
| Candidate misses, a winner reserves/spends, then candidate's approval or preflight fails | Authoritative admitted recheck observes the live winner or safe conflict; original refusal is allowed only at a confirmed no-winner decision point |
| Client presents stale descriptor revision | `StaleDescription` before lookup disclosure; after refresh, reuse of a live key with the changed fingerprint conflicts rather than silently executing |
| Current operation/connection/resource/result access revoked or origin unverified | Refuse admission before disclosure; no new business attempt and no claim about the original's effect. The refusal's subject is the denied observation, not “original not attempted” |
| Exact match, Pending | Wait for the linked original with its attempt/reservation ids fixed. Deadline yields unknown if no authorized terminal observation exists; waiter cancellation cannot alter it |
| Exact match, Replayable within retention | Return original durable Completed/Failed/Aborted result and classification after current admission; no approval re-spend |
| Exact match, Quarantined | Return the original Indeterminate/unknown observation after current admission; never re-dispatch, even after 86,400 s |
| Durable known terminal record, but clock/expiry or index ownership uncertain | Keep reservation live and refuse unsafe reuse; do not infer expiry from a missing read or a wall-clock jump |
| Expired known-result reservation, any candidate fingerprint | CAS retires that exact generation; a new independently admitted candidate may reserve the same key with a new reservation/attempt id. This is a new effect, outside the old deduplication window |

**Retention:** Pending has no replay expiry. On the first durable Completed/Failed/Aborted settlement, atomically mark the reservation Replayable and fix `settled_at` and `replay_expires_at = settled_at + 86400 s` using the host's trusted clock. Replays do not slide the deadline. Known results are replayable while `now < replay_expires_at`; at equality they may expire. Indeterminate becomes Quarantined and has no automatic expiry, reset, or reconciliation in this first profile. If result/reservation settlement lags or fails, the host must recover it from the linked durable attempt, using its original settlement time, before expiry/reuse; an unreadable or unmatched row stays conservative. Expiry and replacement compare reservation id as well as namespace/key, so a stale waiter or GC worker cannot act on a replacement generation. Retiring replay storage does not delete the audit ledger. Hosts must not use time uncertain across restart or a forward wall-clock jump as expiry proof; uncertain retention extends rather than shortens the guarantee.

After known-result expiry the host no longer promises to deduplicate that key; callers must treat resubmission as a new effect and never automatically retry an unknown outcome. Pending/Quarantined reservations remain indexed across restart even when payloads are compacted. A bounded store at capacity refuses **new** reservations before dispatch; it cannot evict unresolved entries to make room. The separately owned [execution audit](../../service/audit.md) has no automatic expiry or cascade from replay retirement; optional attempt correlation is a reference, not ownership. No fresh key guarantees that an earlier unknown effect did not occur.

[F02 verification](idempotency-verification.md) distinguishes the typed ESS lifecycle checks from namespace derivation, policy evaluation, atomic uniqueness, timestamp and runtime fault tests that still need a binding.

### 5.2 Exact lifecycle intent and repeat guarantees (F11/E08)

`none` means absence of a repeat guarantee, not proof that every dispatch changed the world. A refused or already-satisfied request can still have no new effect. `natural` must name the stable source/target, fixed parameters, success meaning and assumptions under which repetition has the same intended state effect. It does not promise identical responses, historical deduplication, at-most-once transitions despite intervening changes, or a host replay record. `keyed` is the separate receiver reservation/observation protocol in §5.1; a provider precondition alone does not implement it.

A selected desired-state operation can define success as the provider's acknowledgement of the requested operation, including its already-satisfied check. That acknowledged intent reports `applied`, with an explicit already-satisfied result for a no-op; it must not fabricate a physical change or a synchronized current-state observation. The adapter must distinguish its declared desired state from measured state and document the exact provider evidence establishing its selected success meaning. Concurrent changes can occur before the acknowledgement is emitted as well as afterward. Such a reply concerns that request only; it does not settle an earlier lost response, guarantee lasting state or justify an implicit follow-up inspect. A generic HTTP success code is insufficient outside the selected endpoint contract.

Each adapter owns its selected idempotency kind and native preparation/repeat proof; current bindings are indexed under [adapters](../../../adapters/README.md). A generated timestamp is not an idempotency key. All kinds retain current admission, required approval, the attempt ledger, the one-shot gate and the ban on automatic resend. None/natural invocations cannot opt into replay by supplying a key; an unsupported key is invalid_input. Keyed invocations require a key.

Correlation request_id, native resource identity/revision, prepared mutation marker and receiver idempotency key are different coordinates. A new request_id alone never changes a keyed business fingerprint or grants another dispatch. Same live key with changed operation, input/preconditions or semantic binding conflicts. Same live key/input observes the original attempt under current result admission, without generating a new marker, refreshing provider preconditions or spending another approval. A deliberate new key/new invocation is new intent requiring admission and approval; it does not repair the original uncertainty. Expired known-result keys, permanently pending/quarantined reservations and spent approvals keep §5.1's existing distinct rules.

## 6. Conformance scenarios

Positive:

- Approval bound to exact input dispatches once; `mutation.classification: applied`; `Prepared`, `Dispatching`, `Completed` are durably ordered.
- Exact namespace/key/fingerprint replay under current admission returns the original result/classification with replay metadata; provider fixture sees one request. Missing/expired original approval does not trigger a second spend; a denied replay discloses no original result.
- Recovery after spending but before the durable gate wins `Prepared → Aborted`; after the gate (whether before or after the send) yields `Indeterminate`. A spent approval cannot be re-spent. Repeat for `not_required` and `event_claim`; spending never determines the outcome.
- A definitive provider refusal yields `Failed` / `refused`; an ambiguous 5xx yields `Indeterminate` / `unknown`.

Adversarial (`docs/design.md:982`):

- Approval for input A presented with input B → `approval_refused`, no dispatch.
- Approval for connection X presented on connection Y → `approval_refused`, no dispatch.
- Stale descriptor revision → `StaleDescription` before approval verification.
- Same live namespace/key, different fingerprint → `idempotency_conflict`, no additional dispatch. Repeat across operation, connection and revision boundaries; repeat the same body/key across distinct caller/origin namespaces to prove isolation.
- B misses, A reserves the exact key/fingerprint and spends their shared event claim, B sees the spent claim: B rechecks the authoritative reservation and observes A under current admission. Repeat with preflight failure, conflicting winner, revoked access, unavailable index and a winner arriving after the confirmed no-winner refusal decision.
- Provider fixture drops the response → `outcome_unknown`; the host never resends. A separate caller invocation without a key is a new effect subject to admission; “no automatic retry” is not a service-side deduplication guarantee.
- Refused preflight credential → the specific credential refusal cause and `not_attempted`; provider fixture sees zero business requests.
- Service deadline before the gate wins abort and fences a late dispatcher. After the gate it yields `unknown`, even if the future was dropped before it reported a send.
- Duplicate waiter deadline while the original is prepared, dispatching or terminal: no additional send and no mutation of the original; return known terminal evidence if observed, otherwise `unknown`.
- Lost ledger acknowledgements: an uncertain prepare cannot permit a send; an uncertain gate cannot permit this worker to send or let recovery assume abort. A failed terminal write preserves the live observer's known result but leaves recovery without it uncertain.

[Verification record](verification.md) maps F01/E01/E11 to these observations, compiled ESS traces, and the still-unimplemented runtime fault tests. Scenario compilation does not exercise a provider, a crash, a real deadline, or a durable store.

## 7. Compatibility and projection

- [Service compatibility](../../service/compatibility.md) selects the proposed `v1alpha2` routes and closed fields. Semantic `operations/v1alpha1` stays at its current family version. Optional fields are still rejected by the old strict reader. Legacy projection is off by default and admits only explicitly verified unchanged reads; it has its own revision and invoke allowlist. Hiding mutation metadata or removing operations from describe alone is insufficient.
- Old `ConnectorOperation v0alpha1..v0alpha3` approval and rate metadata (`docs/design.md:113`) map onto `approval` and `retry_after_seconds`; a compatibility facade is a separate binding (`docs/design.md:1104`).

## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| `AuthenticatedHttp` gains `post`/`put`/`patch`/`delete` with bounded body | `crates/connectors-sdk/src/lib.rs:53-56` (GET only today) |
| Separate extended `Operation`, `Invocation`, `Response` codecs implement the required/optional fields in the compatibility owner | Current legacy structures: [Operation](../../../crates/connectors-core/src/lib.rs#L61) (lines 59–68), [Invocation](../../../crates/connectors-core/src/lib.rs#L92) (90–98), [Outcome](../../../crates/connectors-core/src/lib.rs#L102) (100–105), and [Response](../../../crates/connectors-core/src/lib.rs#L108) (107–113; strict custom reader follows); these do not implement the proposed extended codecs |
| Host ledger port: `prepare`, atomic `open_dispatch`/`abort_prepared`, `record_outcome`, `recover`; separate one-time `spend` port | new host module; common conformance tests for in-memory and durable bindings, with crash/durability tests on the durable binding. No approval spend may substitute for the dispatch fence |
| Service deadline/cancellation supervisor must coordinate with the ledger, fence pre-gate dispatch and preserve post-gate uncertainty; do not classify by whether a future returned | `crates/connectors-host/src/server.rs` (read-only timeout wrapper today); future mutation binding must settle the abort/open race and any terminal-record race atomically |
| HTTP/transport timeout must consume the mutation observation, preserve `outcome_unknown`, and never overwrite it with a generic timeout after possible dispatch. If the connection closes, the client treats missing response as unknown | `crates/connectors-host/src/http.rs` and future mutation clients; an earlier transport deadline needs the same coordinated abort or conservative unknown response |
| Duplicate waiting is observation of the original attempt; waiter timeout/cancellation cannot mutate its lifecycle or spend another claim | future host idempotency coordinator; assert original outcome identity and zero additional sends |
| Receiver owns atomic namespace/key reservation plus Prepared attempt, fingerprint comparison, current replay admission, retained result/index, clock and generation-safe expiry | future host ledger and admission ports, including restart, lost-acknowledgement and capacity tests in §5.1; adapter implementations supply resolved operation/connection semantics but cannot choose caller authority |
| Safe error cause and business-effect classification remain separately observable, including ledger-write failure after a known provider result | future mutation codec and host diagnostics implement the exact proposed fields/version in [service compatibility](../../service/compatibility.md) |
| Federation forwards input value, key and approval strings unchanged under the [F03 proof](../../service/delegation.md); only the leaf verifies/spends approval and owns the business attempt. The gateway never resends on a lost answer | future binding of `crates/connectors-host/src/federation.rs`; current static read federation is not this authority |
| Future adapter declaration binding and compiler enforce the required extended metadata and §3 effect rules before advertisement/execution; legacy readers remain strict and unchanged | `spec-kinds/adapter/v1/schema.json`, `crates/connectors-spec/src/lib.rs`; required extended descriptor fields are owned by service compatibility, not optional legacy additions |

## 9. ESS entities

| Entity / value | Notes |
|---|---|
| `connectors.mutations.AttemptRecord` | Modeled in [mutations.yaml](../../../ess/domains/mutations.yaml). Identity is a host-generated UUID `AttemptId`; request id and instance are correlation fields, not deduplication authority. Each attempt references exactly one existing `ServiceConfiguration` through `instance_id` and its selected `Connection` through the existing host-qualified `connection_ref`. These links grant no current use or deletion ownership; independently retained binding identity survives local revocation. Instance/connection equality is a host predicate. Ledger evidence must outlive request handling |
| Lifecycle | `Prepared → Dispatching → Completed | Failed | Indeterminate`; `Prepared → Aborted`. Each transition has a trusted host command and observed event. No transition permits abort or redispatch after the gate |
| `ApprovalMode`, `EffectKnowledge`, `Observation`, `AttemptStates` | Modeled values and an internal ledger view used by authored scenarios. The view is not a public lookup API; Observation does not choose a wire field |
| `ExecutableEffect`, `SemanticEffect`, `EffectDeclaration`, `SessionEstablishmentObservation` | Private values in mutations.yaml: closed effect vocabularies, declaration excerpt and separately optional ready-receipt reference. They add no declaration entity fields or Session/Attempt relation. Completeness, profile compatibility, evidence provenance, readiness/terminal ordering and receipt disclosure remain UNMAPPED executable predicates |
| `IdempotencyKind` | Shared private classification in mutations.yaml; it does not enumerate provider operations |
| Adapter intent/precondition values | Typed container lifecycle intent and conditional Deployment preparation live in the [adapter-owned models](../../../adapters/README.md), outside shared ESS. Target proof, parameter bounds, marker generation/fixity, provider result interpretation and replay predicates remain UNMAPPED; no provider-resource entity or persistent relation is invented |
| `OperationDeclaration.effects`, `.risk`, `.idempotency`, `.approval` | UNMAPPED additions; the existing declaration entity does not yet contain these fields. Operation reference qualification remains unresolved, so a bare `operation_id` is not asserted as an ESS relation |
| `connectors.idempotency.KeyReservation`, `KeyNamespace`, `RequestFingerprint` | Modeled in [idempotency.yaml](../../../ess/domains/idempotency.yaml): Pending → Replayable → Expired, or Pending → Quarantined; each reservation references one immutable AttemptRecord. Scoped uniqueness, state coupling, admission and expiry arithmetic are explicit host obligations, not proven by the ESS graph |
| ApprovalRedemption | Modeled in [delegation.yaml](../../../ess/domains/delegation.yaml), referencing one executing instance and one Prepared attempt. Atomic issuer/reference uniqueness and one approval per attempt remain receiver-port obligations |
| Connection, approval issuer and SaaS principal | Connection is modeled in [auth_bindings.yaml](../../../ess/domains/auth_bindings.yaml), including the attempt reference above. Issuer and SaaS principal configuration remain outside this selected model scope. Typed authority/fingerprint values preserve historical correlation; they do not grant current authority or constitute approval verification |
| Atomic persistence, actual send cardinality, evidence provenance, deadline races, observation classification | UNMAPPED executable obligations: the ESS graph/typed values do not prove these host behaviors or compute §4's classification table. The gate compiles model and scenario references; runtime fault tests remain required |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Service binding | proposed `v1alpha2` routes; explicit verified legacy read projection only, off by default, with invoke enforcement |
| Idempotency retention | 86,400 s first-profile default; per-adapter override forbidden until measured |
| Whether `event_claim` admission lives in this contract or in `events` | here, as an admission profile; `events` supplies the reference format when that family exists |
| Attempt ledger backend | durable local file first, SQLite second, same port tests |

Persistence ownership is consolidated in [design §31](../../../docs/design.md#31-host-persistence-ownership-and-atomicity). MutationAttemptPort and MutationKeyPort share the required atomic metadata group; ApprovalSpendPort and ExecutionAuditPort remain separately acknowledged boundaries. This inventory does not supply a backend or execute its atomicity predicates.
