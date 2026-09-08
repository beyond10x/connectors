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
| `idempotency.kind` | `none`, `keyed`, `natural` | `none`: every dispatch is a new effect; `keyed`: a caller key deduplicates within `retention_seconds`; `natural`: the provider operation is idempotent by its own semantics (documented per operation) |
| `approval` | `not_required`, `required`, `event_claim` | which admission profile must be satisfied before dispatch |

Invocation additions to `Invocation` (`crates/connectors-core/src/lib.rs:90-97`):

```json
{
  "version": "v1alpha1",
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
| `idempotency_conflict` | same key, different canonical input within retention |
| `outcome_unknown` | the relevant attempt may have dispatched and the observer lacks definitive outcome evidence |

An observation has two independent semantic dimensions: **classification** (`not_attempted`, `refused`, `applied`, `unknown`) describes knowledge of the business effect; **cause** describes why an error occurred (`Timeout`, `Unavailable`, an admission refusal, etc.). `not_attempted` is a classification, not another competing `Error.code`. For example, an unavailable attempt store before any dispatch permission gives cause `Unavailable` and classification `not_attempted`. It does not lose its cause to a second error code. These dimensions have a typed ESS home, `connectors.mutations.Observation`; its structure is not an approved public wire encoding.

The mutation response code for an uncertain outcome is `outcome_unknown`; a transport timeout can remain its safe diagnostic cause, but must not replace it with a plain `Timeout` that implies non-dispatch. An HTTP status or successful transport exchange alone proves neither success nor refusal of the business effect. Read-profile error meanings are unchanged. `not_attempted` proves only that this business attempt did not dispatch: it does not restore a spent approval or promise that resubmission will be admitted.

## 4. Rules

Execution sequence (specializes `docs/design.md:362-371`):

1. Validate framing, version, bounds, input shape, caller context (base contract).
2. Resolve operation and connection under receiver-owned configuration (base).
3. Check the operation is enabled for mutation in the active configuration revision. A mutation disabled by configuration is `Forbidden`, not `NotFound`.
4. If `approval` is `required`: verify the presented approval against caller, operation id, connection, canonical input digest, descriptor revision, and expiry. If `event_claim`: verify the triggering event reference is admitted and unspent. Refuse with `approval_refused` without naming the axis.
5. If `idempotency.kind` is `keyed`: look up the key within retention. Same canonical input: return the recorded outcome with `effect: replayed`. Different input: `idempotency_conflict`. No dispatch in either case.
6. Prepare the provider request without executing it; resolve credential readiness (`auth.evidence`).
7. Durably create an `AttemptRecord` in `Prepared` (host-generated attempt id, instance, request id, operation, connection, input digest, approval mode/reference, idempotency key). No business dispatch is permitted in this state. If anchoring fails, the invocation has cause `Unavailable` and classification `not_attempted`; even an ambiguously acknowledged prepare cannot grant dispatch.
8. When required, spend the approval or event claim exactly once. `approval: not_required` skips spending, not the attempt ledger. Failed or uncertain spending prevents opening the dispatch gate; atomically abort the prepared attempt. A spent approval remains spent even when the attempt aborts.
9. After admission, preflight and required spending succeed, atomically and durably compare-and-set `Prepared → Dispatching`. Only the live worker receiving definite success for this transition may invoke the connection-bound capability once. Merely reading `Dispatching` or receiving an ambiguous write acknowledgement grants no permission to send. Cancellation/recovery competes through `Prepared → Aborted`; a worker losing that race must never send. No timeout wrapper, restart worker or duplicate caller may reacquire dispatch permission.
10. Invoke the capability at most once, with no automatic resend. The durable gate precedes this call: `Dispatching` means **may have sent**, not proof that bytes reached the provider.
11. Record the definitive provider success as `Completed`, a definitive refusal proving no business effect as `Failed`, or the absence of definitive evidence as `Indeterminate`. A generic provider error, HTTP 5xx, malformed answer or partial result is not by itself a known refusal. An observed success follows the operation's declared success meaning, including a declared asynchronous acceptance; it must not promise eventual work completion.

The ledger port serializes these transitions and fences stale writers. Dispatch and abort cannot both win. A lost acknowledgement of opening the gate, or an unreadable ledger when the gate might have opened, leaves recovery uncertain; it cannot be treated as a failed write proving non-dispatch. Recovery never sends an anchored request. Approval spending is not the dispatch fence, and no cross-store transaction with the provider is claimed.

Canonical input digest: SHA-256 over the invocation input after schema validation, serialized with sorted keys and no insignificant whitespace, the same canonicalization the descriptor revision uses (`spec-kinds/adapter/v1/semantics.md`, revision paragraph). Two inputs with the same digest are the same request for approval and idempotency purposes.

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
| Terminal record already durable | preserve `Completed` / `Failed` / `Aborted` / `Indeterminate` | `applied` / `refused` / `not_attempted` / `unknown` respectively | replay preserves the original classification |
| Duplicate waiter expires with no terminal observation of the original attempt | original state unchanged | `unknown` | `outcome_unknown`; the waiter must not claim the original never ran |

The live observer can know more than recovery. If it received a definitive provider answer but terminal persistence fails, it must preserve the known `applied` or `refused` classification in any response it can deliver, and record/report the ledger failure separately through host diagnostics. It must not manufacture provider uncertainty or claim the result is durably replayable. Recovery without that answer still uses `unknown`. Loss of the response to the caller likewise leaves the caller uncertain even if the host recorded a terminal result. A binding must test all three viewpoints.

Recovery uses durable dispatch state and outcome evidence, never “spent means sent” or “unspent means unsent.” An old `attempted` record without the new fence is `Indeterminate` unless separate durable evidence proves non-dispatch and fences all possible dispatchers; this applies equally to `not_required`. A crash after spending but before the new gate is provably pre-dispatch after a successful abort; a crash after the gate but before send is indistinguishable from a crash after send. The four terminal states record the first settled ledger observation and never authorize retry. Late evidence after `Indeterminate` may be retained as diagnostics; changing the terminal classification requires a separately specified reconciliation mechanism, absent from this first profile.

Cancellation: report an aborted business attempt only after the atomic pre-gate abort succeeds. If the gate has opened, the cancellation acknowledgement means `received` only. It neither proves provider termination nor rolls back an external effect. A duplicate waiter's cancellation/deadline ends that wait; it cannot cancel or abort the original attempt.

## 5. Ordering, retry, limits

| Concern | Rule |
|---|---|
| Retry | never automatic. A caller retry with the same idempotency key is a replay lookup, not a re-dispatch. Without a key, a retry is a new effect. `retry_after_seconds` on `RateLimited` applies to reads; for mutations it is advisory and never triggers resend |
| Concurrency | two concurrent invocations with the same idempotency key: at most one obtains dispatch permission. The other waits for the original's terminal record; on waiter deadline it returns that outcome if observed, otherwise `outcome_unknown` with diagnostic cause `Timeout`. It must not return `Capacity`/`not_attempted` for a possibly dispatched original. The waiter's transport has sent no additional business request, but that is not the original attempt's classification |
| Ordering | none across operations; a session-establishing mutation (`effects` includes `session_establishment`) returns a session reference and follows `sessions/v1alpha1` |
| Limits | request body 64 KiB and 20 s service deadline as in the base; idempotency retention is a first-profile default of 86,400 s to be confirmed per adapter (`docs/design.md:1146`) |
| Attempt record size | bounded; the input digest is stored, not the input |

## 6. Conformance scenarios

Positive:

- Approval bound to exact input dispatches once; `effect: applied`; `Prepared`, `Dispatching`, `Completed` are durably ordered.
- Replay with the same key and input returns `effect: replayed` and the original result; provider fixture sees one request.
- Recovery after spending but before the durable gate wins `Prepared → Aborted`; after the gate (whether before or after the send) yields `Indeterminate`. A spent approval cannot be re-spent. Repeat for `not_required` and `event_claim`; spending never determines the outcome.
- A definitive provider refusal yields `Failed` / `refused`; an ambiguous 5xx yields `Indeterminate` / `unknown`.

Adversarial (`docs/design.md:982`):

- Approval for input A presented with input B → `approval_refused`, no dispatch.
- Approval for connection X presented on connection Y → `approval_refused`, no dispatch.
- Stale descriptor revision → `StaleDescription` before approval verification.
- Same key, different input → `idempotency_conflict`, no dispatch.
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
| Connection, ApprovalRedemption, idempotency ownership, approval issuer and SaaS principal | UNMAPPED: separate stories must settle typed identities and relation ownership/cardinality. Opaque record fields do not constitute modeled relations or approval verification |
| Atomic persistence, actual send cardinality, evidence provenance, deadline races, observation classification | UNMAPPED executable obligations: the ESS graph/typed values do not prove these host behaviors or compute §4's classification table. The gate compiles model and scenario references; runtime fault tests remain required |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Wire version bump versus additive optional fields | `v1alpha2` wire; `v1alpha1` projection hides mutation operations |
| Idempotency retention | 86,400 s first-profile default; per-adapter override forbidden until measured |
| Whether `event_claim` admission lives in this contract or in `events` | here, as an admission profile; `events` supplies the reference format when that family exists |
| Attempt ledger backend | durable local file first, SQLite second, same port tests |
