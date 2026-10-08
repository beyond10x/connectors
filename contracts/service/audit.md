# Execution audit contract

**Status:** specified, with a private local SQLite persistence binding and
executable failure fixtures. Public dispatch integration, query service and
exporter remain unimplemented. See the [local binding](../../docs/local-execution-audit.md).

This contract owns the durable execution-audit record required by the governed
service binding. Public response projection remains owned by
[service compatibility](compatibility.md#5-extended-responses-audit-and-mutation-observation),
and mutation effect truth remains owned by
[operations](../operations/v1alpha1/semantics.md).

## 1. Identity and selected aggregate

One `AuditRecord` belongs to one service instance and has one owner-allocated
opaque public `audit_ref`. The public host-qualified identity is
`{instance, audit_ref}`; two instances may use the same opaque ref. The audit
owner derives a private `audit_record_ref` with an injective canonical
qualification of that exact pair. This private key is the entity/command/view
identity and is never projected. Resolution from the public pair compares both
members exactly; unavailable or ambiguous resolution is never absence or a
match. A gateway and its executing leaf allocate independent records under their
own instances; neither record owns, aliases or substitutes for the other.

The selected private encoding is unpadded base64url of a tagged, length-prefixed
byte sequence: the fixed `connectors.execution-audit-record/v1` tag, then a
unsigned 32-bit big-endian byte length and UTF-8 bytes for `instance_id`, then
the same for `audit_ref`. The fixed tag is its literal ASCII bytes, without a
terminator; no normalization, padding or trailing bytes are permitted.
Hashing or delimiter-only concatenation is insufficient. Decoding must recover
the exact pair and reject a noncanonical representation.

The first binding selects one aggregate with an acknowledged anchor and at most
one logical final observation:

| Part | Safe fields | Meaning |
|---|---|---|
| Anchor | private `audit_record_ref`, public `audit_ref`, `instance_id`, `anchor_kind`, optional `activity`, optional `access` (`read` or `write`), `hop_role`, optional `request_id`, optional verified `principal_ref`, optional `operation_id`, optional `connection_ref`, optional `descriptor_revision`, `recorded_at`, optional `attempt_id` | Immutable facts known at the acknowledging hop, except that an absent `attempt_id` may be set once by the attempt link below. `access` is `write` exactly when the admitted operation's host declaration includes `external_write`; a describe has none. `attempt_id`, when present for a mutation, references the existing `AttemptRecord`; neither record owns or deletes the other. |
| Final observation | owner-allocated `observation_id`, `outcome`, optional safe `code`, `recorded_at` | Exactly one logical observation of the local hop's result. It contains no result body, provider message, secret, native target, input, credential, protected correlation or audit-export state. |

`anchor_kind` is `admitted_execution` or `early_refusal`. An admitted invocation
has verified principal and operation coordinates; admitted describe has a
verified principal but no operation. An early authentication/decoding refusal
records only the stage and facts independently verified at that point. Caller
text never fills a trusted field. An `early_refusal` record cannot satisfy the
pre-dispatch audit gate.

When the admitting owner has resolved a selected Connection, `connection_ref`
references that existing host-qualified record. Otherwise it is absent; caller
text cannot create the reference. The link grants no current use and implies no
deletion ownership. The independently retained Connection identity survives
local revocation. Optional attempt and connection references must agree with
the anchor's instance and, when both are present, with the attempt's connection.

**Attempt link.** Where the anchor is acknowledged before the mutation's
attempt exists ([v1alpha2 §4](v1alpha2/semantics.md#4-admission-and-execution-rules),
rule 4), the anchor carries no `attempt_id`. After the attempt is recorded and
before dispatch, the owner links it with `LinkAnchorAttempt`
(`connectors.execution_audit`):

1. Only an `Anchored` record with `access: write` can be linked. A read anchor,
   a describe anchor or one without `access` refuses (`AttemptLinkRefused`) and
   stays unchanged.
2. A record with no `attempt_id` stores the supplied one; this is the only
   change the link makes.
3. Repeating the link with the same `attempt_id` changes nothing and answers as
   already linked; a different `attempt_id` refuses (`AttemptLinkConflict`) and
   the first link stays. A record is linked to at most one attempt.
4. A `FinalObserved` record refuses (`AuditStateConflict`). A store that fails
   or does not answer refuses (`AttemptLinkUnavailable`) without a change.
5. The linked attempt must agree with the anchor's instance and connection as
   the paragraph above says. A link grants no dispatch, approval or replay
   authority.

The lifecycle is `Anchored -> FinalObserved`; the attempt link stays in
`Anchored`. `Anchored` means the anchor write has a definite durable
acknowledgement. It does not grant dispatch by itself;
the current admission, attempt and dispatch owners retain their separate gates.
`FinalObserved` means the one final observation was durably acknowledged. An
unknown business outcome can still have a complete audit observation.

## 2. Ordering and failure

Every audited read, including successful describe, and every mutation requires
an acknowledged `admitted_execution` anchor before provider or delegated-leaf
dispatch. A failed or ambiguous anchor write grants no dispatch: the owner's
decision is `capacity` (§4) or `unavailable` (the store fails or does not
answer), and neither creates a record or placeholder. A write that was never
answered may still have committed a record; no response references it, and it
cannot satisfy a later gate. For a mutation the order is anchor, attempt,
attempt link, dispatch, final observation. The response
uses `audit_status: unavailable` and null `audit_ref` unless transport failure
prevents a response. Unavailable is absence of an acknowledged anchor; no empty,
placeholder or caller-proposed `AuditRecord` represents it.

The final observation append is a separate operation after the local outcome is
known. Append failure leaves the acknowledged record in `Anchored`, projects
`incomplete`, and preserves the known read or mutation result. It cannot change
mutation effect classification, reopen admission, re-spend approval or trigger a
provider/delegation redispatch. A complete record projects `complete`. Explicit
unaudited static reads remain `not_required` and create no record.

Gateway and leaf anchor/final writes are separately acknowledged and are not a
cross-host transaction. The gateway's record describes its local admission and
forwarding observation. The leaf's record describes leaf admission and local
execution. A trustworthy returned leaf reference may be projected as
`source_audit`; it never replaces the gateway record.

## 3. Idempotent final-observation recovery

The owner resolves `{instance, audit_ref}` to its private `audit_record_ref`,
allocates `observation_id` before the first append and retains the exact bounded
observation across acknowledgement uncertainty. For one private record:

1. If no final observation exists, atomically store the supplied observation and
   acknowledge it.
2. Repeating the same `observation_id` with byte-identical semantic fields
   returns the original acknowledgement without another append.
3. The same id with different fields, or a different id after one final exists,
   refuses as a conflict and leaves the original unchanged.
4. Recovery may read this record under internal owner authority and retry only
   this append. Reading `Anchored` or `FinalObserved` never supplies admission,
   attempt or provider-dispatch authority.

This selects idempotent acknowledgement recovery for one logical final
observation. Multiple revisions, amendments and reconciliation are deferred.

## 4. Bounds, retention and disclosure

| Bound | First binding |
|---|---|
| `audit_record_ref` | private injective encoding above, at most 512 ASCII bytes |
| `instance_id`, `audit_ref` | exact service instance id and owner-allocated opaque ref, at most 128 UTF-8 bytes each for this binding |
| `observation_id` | UUID identity; internal append/recovery coordinate, not public projection |
| `request_id`, `principal_ref`, `connection_ref` | at most 128 UTF-8 bytes each |
| `operation_id` | at most 256 UTF-8 bytes |
| `descriptor_revision` | at most 128 UTF-8 bytes |
| final safe `code` | at most 64 ASCII bytes |
| encoded aggregate | at most 4 KiB, excluding backend indexes |
| retained records | at most 100,000 per service instance in the first binding |

The first binding selects no automatic expiry and no audit deletion command.
Replay-payload/index retirement, approval/nonce compaction, connection lifecycle
and attempt retention cannot cascade-delete or rewrite audit. An implementation
at its declared record capacity refuses new audited work before admission is
acknowledged and dispatch is possible; it never evicts an existing record to
make room. A future retention/compaction policy requires a separately reviewed
proof that no linked owner still needs the fact.

Public and diagnostic output exposes only the acknowledged host-qualified
reference and compatibility status. Record contents require separately admitted
internal owner access. This contract introduces no public record lookup, audit
export, search, retention-management or backend-selection API.

## 5. Conformance scenarios

- A read anchor fails before dispatch: no provider read occurs; response has
  `unavailable` and null reference; no placeholder record exists.
- A mutation anchor is acknowledged, then its attempt dispatches and succeeds;
  final append succeeds once: audit is complete and optional attempt correlation
  names the existing attempt without owning it.
- Final append acknowledgement is lost; retry with identical observation id and
  fields returns the original acknowledgement and no second observation.
- Retry with a changed outcome or another observation id refuses without
  overwriting the first observation and without provider dispatch.
- Gateway and leaf each acknowledge their own record. Losing either final append
  changes only that hop's audit status; it does not merge references or repeat
  forwarding/provider work.
- Instances `alpha` and `beta` each allocate public `audit_ref: same`: their
  private `audit_record_ref` values are distinct; appending or recovering one
  cannot observe, mutate or complete the other.
- Capacity is exhausted: the next audited read and mutation refuse before
  dispatch; no retained audit, attempt, approval or replay evidence is evicted.
- An early malformed/authentication refusal may record its known stage, but
  unverified request, principal, operation and connection coordinates remain
  absent and the record cannot satisfy an execution audit gate.
- `audit-anchor-store-unavailable`: the store fails or does not answer the
  anchor write, for a read and for a write → no record is referenced, no attempt
  is recorded and no provider dispatch occurs; the response has `unavailable`
  and a null reference.
- `audit-link-attempt-once`: a write anchor is acknowledged, its attempt is
  recorded and linked → the record's `attempt_id` names that attempt; the
  record stays `Anchored` until its final observation.
- `audit-link-attempt-repeat-unchanged`: the same link is repeated → no change
  and no second link.
- `audit-link-attempt-conflict`: a link to another attempt after the first →
  `AttemptLinkConflict`, the first `attempt_id` unchanged.
- `audit-link-read-anchor-refused`: a link on a read, describe or
  `access`-less anchor → `AttemptLinkRefused`, the record unchanged.
- `audit-link-after-final-refused`: a link on a `FinalObserved` record →
  `AuditStateConflict`, the record unchanged.
- `audit-link-store-unavailable`: the store fails the link →
  `AttemptLinkUnavailable`, the record unchanged.

## 6. ESS and implementation boundary

`connectors.execution_audit.AuditRecord` models the private qualified identity,
separate public ref, optional attempt/connection references, one-way lifecycle and bounded
semantic values. `LinkAnchorAttempt` models the attempt link, and
`AnchorDecision` the `allow`, `capacity` and `unavailable` answers. ESS
validates declared types, the instance/attempt references, state-transition
causation and the link's read-anchor, conflict, repeat and state answers. Cross-record instance/connection equality remains
an owner predicate.
It does not enforce optional-field co-presence, byte/count bounds, trusted fact
provenance, durable acknowledgement, atomic uniqueness, exact-once append,
byte-identical retry comparison, actual clocks, capacity refusal, retention or
dispatch ordering. Those remain explicit persistence-port and conformance
obligations. The local SQLite port implements the selected record and append
semantics; complete governed dispatch remains a separate integration obligation.
