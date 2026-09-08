# operations/v1alpha1 — `mutation` profile

- **Status:** proposed, not implemented. Design document; no Rust binding, schema, or fixture exists yet.
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

Outcome additions to `Outcome` (`crates/connectors-core/src/lib.rs:100-104`):

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
| `outcome_unknown` | dispatch started; no terminal provider answer was observed |
| `not_attempted` | refused before any provider dispatch; distinct from every other failure so callers can retry safely |

Existing codes keep their meaning; `Unavailable` and `Timeout` are reserved for failures known to have happened *before* dispatch. After dispatch, a lost answer is `outcome_unknown`, never `Timeout`.

## 4. Rules

Execution sequence (specializes `docs/design.md:362-371`):

1. Validate framing, version, bounds, input shape, caller context (base contract).
2. Resolve operation and connection under receiver-owned configuration (base).
3. Check the operation is enabled for mutation in the active configuration revision. A mutation disabled by configuration is `Forbidden`, not `NotFound`.
4. If `approval` is `required`: verify the presented approval against caller, operation id, connection, canonical input digest, descriptor revision, and expiry. If `event_claim`: verify the triggering event reference is admitted and unspent. Refuse with `approval_refused` without naming the axis.
5. If `idempotency.kind` is `keyed`: look up the key within retention. Same canonical input: return the recorded outcome with `effect: replayed`. Different input: `idempotency_conflict`. No dispatch in either case.
6. Prepare the provider request without executing it; resolve credential readiness (`auth.evidence`).
7. Durably record `attempted` (request id, operation, connection, input digest, approval reference, idempotency key, time). A store that cannot take the record refuses with `Unavailable` and `not_attempted` semantics.
8. Spend the approval or event claim exactly once. A spend that fails after step 7 is recorded as `aborted`; nothing dispatched.
9. Dispatch through the connection-bound capability (`auth.capability`).
10. Record the terminal outcome (`completed`, `failed`, `outcome_unknown`) keyed to the attempted record.

Canonical input digest: SHA-256 over the invocation input after schema validation, serialized with sorted keys and no insignificant whitespace, the same canonicalization the descriptor revision uses (`spec-kinds/adapter/v1/semantics.md`, revision paragraph). Two inputs with the same digest are the same request for approval and idempotency purposes.

Authority boundary: `effects`, `risk`, and `approval` are description metadata. They inform admission and UX; they authorize nothing (`docs/design.md:358`). The receiver decides admission from its own configuration and policy at execution time.

Outcome truthfulness: four states must remain distinguishable to the caller: not attempted, known refusal, known success, effect possibly attempted with unknown outcome (`docs/design.md:375`). An HTTP timeout after dispatch, a lost response, or a process crash between steps 9 and 10 is `outcome_unknown`. Startup recovery classifies attempted records with no terminal row: spent approval → `indeterminate`; unspent → `aborted`.

Cancellation: a cancel request received before step 9 aborts the attempt (`aborted`). After step 9 the cancellation acknowledgement states `received` only; it never claims the external effect was rolled back.

## 5. Ordering, retry, limits

| Concern | Rule |
|---|---|
| Retry | never automatic. A caller retry with the same idempotency key is a replay lookup, not a re-dispatch. Without a key, a retry is a new effect. `retry_after_seconds` on `RateLimited` applies to reads; for mutations it is advisory and never triggers resend |
| Concurrency | two concurrent invocations with the same idempotency key: one dispatches, the other waits for its terminal record or returns `Capacity` after the service deadline; never two dispatches |
| Ordering | none across operations; a session-establishing mutation (`effects` includes `session_establishment`) returns a session reference and follows `sessions/v1alpha1` |
| Limits | request body 64 KiB and 20 s service deadline as in the base; idempotency retention is a first-profile default of 86,400 s to be confirmed per adapter (`docs/design.md:1146`) |
| Attempt record size | bounded; the input digest is stored, not the input |

## 6. Conformance scenarios

Positive:

- Approval bound to exact input dispatches once; `effect: applied`; attempted and completed rows exist in order.
- Replay with the same key and input returns `effect: replayed` and the original result; provider fixture sees one request.
- Recovery after a crash injected between spend and dispatch yields `aborted`; after dispatch yields `indeterminate`, and the approval cannot be re-spent.

Adversarial (`docs/design.md:982`):

- Approval for input A presented with input B → `approval_refused`, no dispatch.
- Approval for connection X presented on connection Y → `approval_refused`, no dispatch.
- Stale descriptor revision → `StaleDescription` before approval verification.
- Same key, different input → `idempotency_conflict`, no dispatch.
- Provider fixture drops the response → `outcome_unknown`; a second identical invocation without key is refused by the caller-facing rule "no automatic retry" (the service does not resend; the fixture sees one request).
- Refused preflight credential → `not_attempted`; provider fixture sees zero requests.

## 7. Compatibility and projection

- Descriptor fields are additive and optional; the current `Descriptor` deserializer uses `deny_unknown_fields` (`crates/connectors-core/src/lib.rs:70-72`), so clients at the current binary refuse descriptors that carry them. Either the wire version becomes `v1alpha2` or the current client is released with the optional fields before any adapter advertises a mutation. Default taken: `v1alpha2` wire with `v1alpha1` projection dropping mutation operations entirely (a client that cannot see effects must not be able to invoke one).
- Old `ConnectorOperation v0alpha1..v0alpha3` approval and rate metadata (`docs/design.md:113`) map onto `approval` and `retry_after_seconds`; a compatibility facade is a separate binding (`docs/design.md:1104`).

## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| `AuthenticatedHttp` gains `post`/`put`/`patch`/`delete` with bounded body | `crates/connectors-sdk/src/lib.rs:53-56` (GET only today) |
| `Operation`, `Invocation`, `Outcome` gain the optional fields above | `crates/connectors-core/src/lib.rs:59-104` |
| Host attempt/approval ledger port: `record_attempt`, `spend`, `record_outcome`, `recover` | new host module; two bindings (in-memory, durable file or SQLite) through the same port tests |
| Federation forwards `idempotency_key` and `approval` unchanged and never resends on a lost downstream answer | `crates/connectors-host/src/federation.rs` |
| Adapter spec kind gains the optional descriptor fields; compiler refuses `profile: mutation` without `effects` containing `external_write` | `spec-kinds/adapter/v1/schema.json`, `crates/connectors-spec/src/lib.rs` |

## 9. ESS entities

| Entity / value | Notes |
|---|---|
| `OperationDeclaration.effects`, `.risk`, `.idempotency`, `.approval` | value fields on the existing entity (`ess/domains/declarations.yaml`) |
| `AttemptRecord` (identity: request id + instance) with lifecycle `attempted → completed | failed | indeterminate | aborted` | first entity with a real lifecycle in this repo |
| `ApprovalRedemption` (identity: approval reference digest) | one-time; relation `spent_by → AttemptRecord`, cardinality one |
| Approval issuer and SaaS principal | UNMAPPED: issuer identity and tenant relation are host/product concerns (`docs/design.md:383`) |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Wire version bump versus additive optional fields | `v1alpha2` wire; `v1alpha1` projection hides mutation operations |
| Idempotency retention | 86,400 s first-profile default; per-adapter override forbidden until measured |
| Whether `event_claim` admission lives in this contract or in `events` | here, as an admission profile; `events` supplies the reference format when that family exists |
| Attempt ledger backend | durable local file first, SQLite second, same port tests |
