# operations/v1alpha1 — `mutation` profile

- **Status:** proposed; mutation runtime and wire binding are not implemented. The host attempt lifecycle is modeled in [ESS](../../../ess/domains/mutations.yaml), with [authored scenarios](scenarios/) and an explicit [verification boundary](verification.md).
- **Base contract:** `operations/v1alpha1` as implemented in [service v1alpha1](../../service/v1alpha1/semantics.md) (describe/invoke wire boundary, error envelope, limits, stale-description refusal). This document adds one profile and does not restate the base.
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `operations/v1alpha1` |
| Profile | `mutation` |
| Existing profiles | read profiles declared per adapter (`kubernetes-list`, `gitlab-*`, `postgresql-native-text`), all promising "no external business mutation" (`contracts/service/v1alpha1/semantics.md`, Wire boundary) |
| Relation | additive: an operation declares `profile: mutation` and gains the fields and rules below; read operations are unchanged |

A mutation is an operation whose dispatch can change external state. The profile exists because the base wire contract explicitly promises no business mutation, no retry, and no idempotency; those promises cannot cover Jira issue creation, a Kubernetes rollout restart, or a SIP dial.

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| Operation metadata `direction`, `risk`, `idempotency`, `effects`, `semantic_effects` | `../connectors/providers/b10x.toml:741-752` (`sip-dial`), `providers/jira.toml:376` (`jira-issue-create`, `effects = ["write","network"]`) | preserve as typed descriptor fields (`effects`, `idempotency`, `risk`); drop `direction` (redundant with effects) |
| Approval verification against issuer, subject, operation, Connection, canonical input digest, expiry; refusal names no axis | `../connectors/crates/domain/src/approval.rs:1-10` | preserve |
| One-time redemption as a bounded append whose payload is the attempted-audit row; anchor before claim, claim before dispatch | `approval.rs:12-35` | preserve the ordering guarantee; the storage mechanism (keyed byte cells) is not preserved (`docs/design.md:961`) |
| Recovery scan classifying anchored attempts as `indeterminate` or `aborted` | `approval.rs:37-44` | preserve |
| Local event-triggered one-time `event:` claims at the dispatch seam | `../connectors/crates/connectors-runtime/src/claims.rs:1-15` | change: express as a second admission profile (`event-claim`) of the same approval binding, not a separate mechanism (`docs/design.md:1093`) |
| Hosted grant evaluation as a separate policy from approval | `docs/design.md:78` | preserve the separation; grant evaluation is host policy outside this contract |
| Automatic retries, rate-limit-triggered resend | `docs/design.md:377` | remove: no automatic retry of a mutation under any outcome |

## 3. Types

Descriptor additions to `Operation` (`crates/connectors-core/src/lib.rs:59-66`), all optional so read operations serialize unchanged:

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
| `effects` | `external_write`, `network`, `process`, `local_system`, `send_external`, `session_establishment` | what dispatch may do; `external_write` is the discriminator for this profile |
| `semantic_effects` | `human_visible`, `billable`, `irreversible` | admission and UX hints; never authority |
| `risk` | `low`, `medium`, `high` | admission hint |
| `idempotency.kind` | `none`, `keyed`, `natural` | `none`: every dispatch is a new effect; `keyed`: the receiver owns scoped reservation/replay under §5.1, with `retention_seconds` for durable known terminal results; `natural`: the provider operation is idempotent by its own semantics (documented per operation), without implying host replay storage |
| `approval` | `not_required`, `required`, `event_claim` | which admission profile must be satisfied before dispatch |

Illustrative invocation additions to `Invocation` (`crates/connectors-core/src/lib.rs:90-97`).
This is an incomplete proposed message: the version field is deliberately omitted.
The implemented `v1alpha1` reader refuses these additions; the exact version and
codec belong to `story:contracts-wire-compatibility`.

```json
{
  "request_id": "…",
  "operation": "issue.create",
  "revision": "<descriptor revision>",
  "input": {},
  "idempotency_key": "optional, required when idempotency.kind = keyed",
  "approval": { "reference": "opaque", "evidence": "opaque, issuer-signed" }
}
```

Illustrative outcome additions to `Outcome` (`crates/connectors-core/src/lib.rs:100-104`); the versioned codec is owned by `story:contracts-wire-compatibility`, not chosen by these examples:

```json
{ "status": "success", "result": {}, "effect": "applied" }
{ "status": "success", "result": {}, "effect": "replayed", "original_request_id": "…" }
{ "status": "error", "error": { "code": "outcome_unknown", "message": "…" } }
```

New error codes beyond `ErrorCode` (`crates/connectors-core/src/lib.rs:13-27`):

| Code | When |
|---|---|
| `approval_required` | operation declares `approval: required` and none was presented |
| `approval_refused` | presented approval failed verification; the message never names the failing axis |
| `approval_replayed` | the approval reference was already spent |
| `idempotency_conflict` | same namespace/key, different request fingerprint while its reservation remains live; disclose neither the stored request nor the differing axis |
| `outcome_unknown` | the relevant attempt may have dispatched and the observer lacks definitive outcome evidence |

An observation has two independent semantic dimensions: **classification** (`not_attempted`, `refused`, `applied`, `unknown`) describes knowledge of the business effect; **cause** describes why an error occurred (`Timeout`, `Unavailable`, an admission refusal, etc.). `not_attempted` is a classification, not another competing `Error.code`. For example, an unavailable attempt store before any dispatch permission gives cause `Unavailable` and classification `not_attempted`. It does not lose its cause to a second error code. These dimensions have a typed ESS home, `connectors.mutations.Observation`; its structure is not an approved public wire encoding.

The mutation response code for an uncertain outcome is `outcome_unknown`; a transport timeout can remain its safe diagnostic cause, but must not replace it with a plain `Timeout` that implies non-dispatch. An HTTP status or successful transport exchange alone proves neither success nor refusal of the business effect. Read-profile error meanings are unchanged. `not_attempted` proves only that this business attempt did not dispatch: it does not restore a spent approval or promise that resubmission will be admitted.

## 4. Rules

Execution sequence (specializes `docs/design.md:362-371`):

1. Validate framing, version, bounds, input shape, caller context (base contract).
2. Resolve operation and connection under receiver-owned configuration (base).
3. Check current caller authority, operation enablement, connection/resource access and description freshness. A mutation disabled by configuration is `Forbidden`, not `NotFound`. This admission precedes any replay result, conflict or existence disclosure.
4. If `idempotency.kind` is `keyed`, derive the trusted namespace and full fingerprint in §5.1 and inspect its reservation. A matching record is a replay/wait/quarantine observation, never a new dispatch. A conflicting record refuses. Replay and waiting require current access to the original result scope; they neither validate a new execution approval nor redeem the original one again.
5. Only a candidate for a new attempt verifies its required approval against caller, operation, connection, canonical input digest, descriptor revision and expiry, or verifies an admitted unspent event claim. A previously spent/expired approval cannot authorize a new attempt, including reuse after replay retention expires. Before returning a candidate's `approval_refused` after a cache miss, perform the authoritative winner recheck in §5.1; a concurrent exact replay does not require that claim to remain unspent.
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

## 5. Ordering, retry, limits

| Concern | Rule |
|---|---|
| Retry | never automatic. A caller retry with a live scoped key observes its reservation; it cannot re-dispatch. After a known-result reservation expires, §5.1 permits a freshly admitted new attempt. A keyed operation refuses a missing key; for `none`/`natural`, a new invocation is a new attempt. `retry_after_seconds` on `RateLimited` is advisory and never triggers mutation resend |
| Concurrency | two admitted invocations with the same namespace/key and fingerprint: at most one obtains dispatch permission. The other waits for the original's terminal record; on waiter deadline it returns that outcome only if observed and still authorized, otherwise unknown or an admission refusal as §5.1 requires. A deadline alone is `outcome_unknown` with diagnostic cause `Timeout`, never `Capacity`/`not_attempted` for a possibly dispatched original. The waiter sends no additional business request |
| Ordering | none across operations; a session-establishing mutation (`effects` includes `session_establishment`) returns a session reference and follows `sessions/v1alpha1` |
| Limits | request body 64 KiB and 20 s service deadline as in the base; known-result replay retention is 86,400 s from durable terminal settlement, not a timeout for pending/unknown reservations (§5.1) |
| Attempt record size | bounded; the input digest is stored, not the input |

### 5.1 Key namespace, fingerprint and replay admission

The receiving mutation host owns the keyed ledger and its durable unique index; the adapter and provider do not define its namespace. Persistence of a reservation and its Prepared attempt is one atomic responsibility. An acknowledged reservation is bound to exactly one immutable attempt id; the attempt's audit evidence is independent of replay-cache deletion. Recovery never replaces an unresolved reservation with a new attempt. ESS records this as `connectors.idempotency.KeyReservation` **referencing**, not owning, `AttemptRecord`.

**Namespace:** `(receiver_instance, admitted_authority, trusted_origin)`. `admitted_authority` includes tenant (or configured local absence), optional realm, stable caller identity and executor identity. An absent realm differs from `default`; absent tenant is not a wildcard. Direct origin is tagged `direct` with the receiver's configured identity. Federated origin is tagged `federated` with the authenticated gateway/origin authority identity, independently of the resolved receiver. Credential bytes, credential rotation, connection-pool handles and caller-supplied headers never define these identities. Request input cannot choose tenant, realm, caller, executor or origin. If a binding cannot establish the originating caller and origin without collapsing callers behind one broad gateway credential, keyed mutation is refused until the F03 federation binding supplies trusted context; this story does not invent that delegation protocol.

The index key is `(namespace, caller_key)`. `caller_key` is a required nonempty opaque string for keyed operations, compared exactly without trimming, case folding or Unicode normalization. It remains subject to the base request bound. Store structured fields or use an injective, versioned encoding: `mutation-key/v1` is a canonical JSON array of tag, receiver, tenant, realm, caller, executor, origin tag, origin authority, and key, in that order; preserve JSON null for absence. Delimiter concatenation and a lossy hash-only index are insufficient. A hash may index the tuple only if equality is checked against the original tuple.

**Fingerprint:** resolved source-qualified operation reference, stable connection reference and its admitted metadata revision, contract/version and profile, descriptor revision, active host configuration revision, canonicalization identifier and canonical input digest. Store these coordinates under `mutation-request/v1`; if hashed, compare the stored coordinates as well. Operation/connection/revision changes are **not namespace changes**: while a reservation is live, reusing its key for any different fingerprint is a safe, axis-free `idempotency_conflict`, with no result disclosure or additional dispatch. Request correlation id, deadline, key itself, approval token/expiry and rotating credential bytes are excluded. Metadata/configuration revisions must change when a route, external identity, destination, resource interpretation or operation meaning changes. A same-identity credential refresh alone is not a semantic revision. An implementation unable to detect an effect-relevant binding change must refuse keyed dispatch; silently preserving the fingerprint is not allowed.

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

After known-result expiry the host no longer promises to deduplicate that key; callers must treat resubmission as a new effect and never automatically retry an unknown outcome. Pending/Quarantined reservations remain indexed across restart even when payloads are compacted. A bounded store at capacity refuses **new** reservations before dispatch; it cannot evict unresolved entries to make room. First-profile reconciliation and audit retention remain separate work, and no fresh key guarantees that an earlier unknown effect did not occur.

[F02 verification](idempotency-verification.md) distinguishes the typed ESS lifecycle checks from namespace derivation, policy evaluation, atomic uniqueness, timestamp and runtime fault tests that still need a binding.

## 6. Conformance scenarios

Positive:

- Approval bound to exact input dispatches once; `effect: applied`; `Prepared`, `Dispatching`, `Completed` are durably ordered.
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

- Descriptor fields are additive and optional; the current `Descriptor` deserializer uses `deny_unknown_fields` (`crates/connectors-core/src/lib.rs:70-72`), so clients at the current binary refuse descriptors that carry them. Either the wire version becomes `v1alpha2` or the current client is released with the optional fields before any adapter advertises a mutation. Default taken: `v1alpha2` wire with `v1alpha1` projection dropping mutation operations entirely (a client that cannot see effects must not be able to invoke one).
- Old `ConnectorOperation v0alpha1..v0alpha3` approval and rate metadata (`docs/design.md:113`) map onto `approval` and `retry_after_seconds`; a compatibility facade is a separate binding (`docs/design.md:1104`).

## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| `AuthenticatedHttp` gains `post`/`put`/`patch`/`delete` with bounded body | `crates/connectors-sdk/src/lib.rs:53-56` (GET only today) |
| `Operation`, `Invocation`, `Outcome` gain the optional fields above | `crates/connectors-core/src/lib.rs:59-104` |
| Host ledger port: `prepare`, atomic `open_dispatch`/`abort_prepared`, `record_outcome`, `recover`; separate one-time `spend` port | new host module; common conformance tests for in-memory and durable bindings, with crash/durability tests on the durable binding. No approval spend may substitute for the dispatch fence |
| Service deadline/cancellation supervisor must coordinate with the ledger, fence pre-gate dispatch and preserve post-gate uncertainty; do not classify by whether a future returned | `crates/connectors-host/src/server.rs` (read-only timeout wrapper today); future mutation binding must settle the abort/open race and any terminal-record race atomically |
| HTTP/transport timeout must consume the mutation observation, preserve `outcome_unknown`, and never overwrite it with a generic timeout after possible dispatch. If the connection closes, the client treats missing response as unknown | `crates/connectors-host/src/http.rs` and future mutation clients; an earlier transport deadline needs the same coordinated abort or conservative unknown response |
| Duplicate waiting is observation of the original attempt; waiter timeout/cancellation cannot mutate its lifecycle or spend another claim | future host idempotency coordinator; assert original outcome identity and zero additional sends |
| Receiver owns atomic namespace/key reservation plus Prepared attempt, fingerprint comparison, current replay admission, retained result/index, clock and generation-safe expiry | future host ledger and admission ports, including restart, lost-acknowledgement and capacity tests in §5.1; adapter implementations supply resolved operation/connection semantics but cannot choose caller authority |
| Safe error cause and business-effect classification remain separately observable, including ledger-write failure after a known provider result | future mutation wire binding and host diagnostics; exact fields/version belong to `story:contracts-wire-compatibility` |
| Federation forwards `idempotency_key` and `approval` unchanged and never resends on a lost downstream answer | `crates/connectors-host/src/federation.rs` |
| Adapter spec kind gains the optional descriptor fields; compiler refuses `profile: mutation` without `effects` containing `external_write` | `spec-kinds/adapter/v1/schema.json`, `crates/connectors-spec/src/lib.rs` |

## 9. ESS entities

| Entity / value | Notes |
|---|---|
| `connectors.mutations.AttemptRecord` | Modeled in [mutations.yaml](../../../ess/domains/mutations.yaml). Identity is a host-generated UUID `AttemptId`; request id and instance are correlation fields, not deduplication authority. Each attempt references exactly one existing `ServiceConfiguration` through `instance_id`; ledger evidence must outlive request handling |
| Lifecycle | `Prepared → Dispatching → Completed | Failed | Indeterminate`; `Prepared → Aborted`. Each transition has a trusted host command and observed event. No transition permits abort or redispatch after the gate |
| `ApprovalMode`, `EffectKnowledge`, `Observation`, `AttemptStates` | Modeled values and an internal ledger view used by authored scenarios. The view is not a public lookup API; Observation does not choose a wire field |
| `OperationDeclaration.effects`, `.risk`, `.idempotency`, `.approval` | UNMAPPED additions; the existing declaration entity does not yet contain these fields. Operation reference qualification remains unresolved, so a bare `operation_id` is not asserted as an ESS relation |
| `connectors.idempotency.KeyReservation`, `KeyNamespace`, `RequestFingerprint` | Modeled in [idempotency.yaml](../../../ess/domains/idempotency.yaml): Pending → Replayable → Expired, or Pending → Quarantined; each reservation references one immutable AttemptRecord. Scoped uniqueness, state coupling, admission and expiry arithmetic are explicit host obligations, not proven by the ESS graph |
| Connection, ApprovalRedemption, approval issuer and SaaS principal | UNMAPPED: separate stories must settle entity identities and relation ownership/cardinality. Typed authority/fingerprint values do not constitute those entity relations or approval verification |
| Atomic persistence, actual send cardinality, evidence provenance, deadline races, observation classification | UNMAPPED executable obligations: the ESS graph/typed values do not prove these host behaviors or compute §4's classification table. The gate compiles model and scenario references; runtime fault tests remain required |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Wire version bump versus additive optional fields | `v1alpha2` wire; `v1alpha1` projection hides mutation operations |
| Idempotency retention | 86,400 s first-profile default; per-adapter override forbidden until measured |
| Whether `event_claim` admission lives in this contract or in `events` | here, as an admission profile; `events` supplies the reference format when that family exists |
| Attempt ledger backend | durable local file first, SQLite second, same port tests |
