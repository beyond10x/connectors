# MCP inbound mutation and replay — server/v1alpha1

**Status: authored mapping and document checks only. Mutation advertisement remains
withheld until a complete executable inbound binding is implemented and verified.**
This document adds no MCP server, ledger, approval format or provider dispatch.
Owner: `story:mcp-inbound-mutation-replay`. The
[projection](projection.md) owns qualified capability names, revisions, exact
primary `2026-07-28` versus interoperability `2025-11-25` envelopes and result/error
channels. The [local binding](semantics.md) owns single-owner stdio admission,
framing, cancellation and supervision. Resources and prompts remain read-only.
An accepted document does not make an operation implemented or advertise a tool.

The [mutation profile](../../../../../contracts/operations/v1alpha1/semantics.md)
§§4–5.2 and [local coordinator](../../../../../contracts/service/local-mutations.md)
are authoritative for attempts, explicit business keys, approval, dispatch,
recovery and retention. The existing guarded GitLab private-protocol-two path is
implemented; that is separate evidence and is not an MCP binding.
The [named cases](mutation-cases.json) are document metadata. Their vocabulary
comes from the existing shared models, not invented ESS commands/session traces.

## Request identity and the protected boundary

A mutating tool request selects exactly the operation represented by projection's
canonical instance/adapter/operation name and carries its exact current descriptor
revision in the already selected MCP `_meta` coordinate. Its business `input` is
validated against that operation's declared schema. For `idempotency.kind:keyed`,
the operation's selected tool argument wrapper carries the existing
`idempotency_key` member as a **required nonempty opaque string**, compared exactly
without trimming, case folding or Unicode normalization. No MCP ID, generated
timestamp, hash of the request body, resource identifier or session ID fills a
missing key. The key is a caller-chosen replay coordinate, not authority. The
operation request bound applies before ledger use. Unknown wrapper members and
unsupported key combinations refuse under the selected closed codec.

The JSON-RPC request `id` correlates this wire exchange only. The local host issues
a fresh host request identity for each admitted invocation; it generates its own
attempt identity only for a new anchored attempt. None is interchangeable with
the explicit business key. A new MCP/host request ID with the same live qualified
key and fingerprint observes the same original. The same MCP ID with a deliberate
new key identifies potentially new business intent, not a retry guarantee. Reusing
an MCP ID must also obey the selected protocol's request-correlation rules; it
cannot force another invocation or bypass a refusal. A repeated notification,
unsupported call or malformed request gains no mutation authority.

For `none` and `natural`, supplying a business key is `invalid_input`; no retained
key lookup or automatic resend is fabricated. Natural repeat semantics remain
native-operation guarantees, not receiver deduplication. This document's recorded
replay promise applies to **explicitly keyed** mutations while their reservation
governs. Every kind prohibits automatic redispatch after uncertainty. A deliberate
new unkeyed invocation or new key can create a new effect under current admission;
it does not prove an earlier unknown effect did not happen.

Approval stays with the existing protected local issuance/receiver path. Protected
proof bytes, signing keys, credential locators and provider secrets are never
ordinary MCP arguments, prompt content, resource URIs or transport metadata.
This document does not select a new public approval handle or callback channel.
Until the executable inbound binding can supply the existing protected proof
source to the admitted coordinator, a required-approval mutation is unbound and
omitted. It cannot silently downgrade approval, trust a tool annotation, or accept
a caller-written approval subject as verified authority. Existing canonical
subject reconstruction and protected verification/spending remain mandatory.

## Qualified reservation identity

The receiver owns the existing `(namespace, caller_key)` index. Namespace is
`(receiver_instance, admitted_authority, trusted_origin)` from the current admitted
local owner context. Authority preserves explicit tenant/realm/executor absence;
null differs from default or empty strings. Direct origin is tagged `direct`, its
authority is the receiver identity and its route is null. The wider federated type
exists, but this local document does not activate delegated ingress, another
caller, a gateway or a caller-to-Connection assignment.

Use existing `mutation-key/v1`: an injective canonical JSON array of format tag,
receiver, tenant, realm, caller, executor, origin tag, origin authority and key.
Keep structured coordinates alongside any hash index and compare the actual tuple.
Delimiter concatenation and hash-only equality do not satisfy the owner contract.
MCP input cannot select tenant, realm, caller, executor, origin or another provider
Connection. The operation and connection are resolved under existing host policy.

Use existing `mutation-request/v2` for the complete fingerprint. Preserve resolved
source-qualified operation identity, stable connection and its admitted metadata
revision, contract/version, profile, descriptor revision, active configuration
revision, `adapter-v1-canonical-json` input digest and nullable route binding.
Input canonicalization uses the already validated input, sorted object keys and
no insignificant whitespace. The digest alone is not the full authorized request.
Operation/connection/revision changes change the fingerprint, not the namespace:
a governing live key then conflicts, rather than obtaining an independent send.
Credential rotation with unchanged semantic identity does not create new intent.
A private publication fence can invalidate dispatch without changing this
fingerprint. No inability to detect semantic binding changes is silently accepted.

MCP and host correlation IDs, delivery deadlines, approval token/expiry and
rotating credential bytes are excluded from replay identity. Existing typed fields
are inventoried below, including the nested authority/origin/route values; source
`resolved_host` means trusted context or validated resolved request meaning, never
caller-supplied authority. This inventory adds no product type or stored field.

| Owner | Field | Source |
|---|---|---|
| namespace | receiver_instance | resolved_host |
| namespace | authority | resolved_host |
| namespace | origin | resolved_host |
| authority | tenant | resolved_host |
| authority | realm | resolved_host |
| authority | caller | resolved_host |
| authority | executor | resolved_host |
| origin | kind | resolved_host |
| origin | authority_ref | resolved_host |
| fingerprint | operation_ref | resolved_host |
| fingerprint | connection_ref | resolved_host |
| fingerprint | connection_revision | resolved_host |
| fingerprint | contract_ref | resolved_host |
| fingerprint | profile | resolved_host |
| fingerprint | descriptor_revision | resolved_host |
| fingerprint | configuration_revision | resolved_host |
| fingerprint | canonicalization_version | resolved_host |
| fingerprint | input_digest | resolved_host |
| fingerprint | route | resolved_host |
| route | gateway_instance | resolved_host |
| route | route_id | resolved_host |
| route | route_revision | resolved_host |

The identity cases compare explicit namespace/key/fingerprint premises. They do
not execute the receiver's canonical encoder or simulate a caller assignment.

| Correlation case | Business identity decision |
|---|---|
| new-correlation-same-key | same_reservation |
| same-correlation-new-key | distinct_intent |
| same-key-changed-meaning | idempotency_conflict |
| same-spelling-other-authority | distinct_intent |

## Admission, replay and candidate execution

For syntactically valid bounded framing/authentication/version, use projection's
current policy/target/result observation authority before descriptor revision;
then permitted private bound lookup/enablement, operation input validity and
containment, then key inspection. Current denial wins over stale/existence/key
or outcome disclosure. Current policy unavailable is `unavailable`, never cached
success. Admitted stale revision yields `stale_description` before private lookup
or key decisions. Refusals before admitted key/result observation omit mutation
metadata and cannot claim a possibly existing original was not attempted.

An exact live fingerprint match selects the existing reservation and immutable
attempt. Replayable returns its retained authorized observation; Pending waits
for that original; Quarantined returns unknown. A different live fingerprint is
axis-free `idempotency_conflict`, with no stored request/result disclosure. Recheck
current result admission before returning replay or finishing a wait; revocation
observed before that decision wins. A cache hit is no grant. A waiter deadline or
cancellation changes neither the original lifecycle nor its reservation and gives
no dispatch or approval-spend permission. If no authorized definitive observation
is available, waiter expiry is `outcome_unknown` with a safe timeout cause. A
revoked waiter instead receives the current admission refusal without old data.

Exact retained observation performs no new provider permission/precondition probe,
credential access/refresh, proof verification/spend, marker generation or business
dispatch. Missing, expired or already-spent original approval does not prevent an
otherwise admitted exact replay. Provider dependencies may be unavailable while
retained observation succeeds. Recovery of an unsettled original uses only the
existing quiescent owner/metadata recovery path, never an MCP-triggered resend.

A miss is not a reservation. Only a candidate new attempt validates required
approval against the full existing canonical subject and performs new-attempt
preflight. Before returning an approval or preflight refusal after a miss,
authoritatively recheck the namespace/key under current disclosure admission.
An exact concurrent winner follows normal wait/replay/quarantine observation; a
different winner conflicts; confirmed absence permits the original refusal.
Unavailable recheck is conservative uncertainty with the safe ledger cause, never
absence. Current admission refusal still wins. The recheck is one serialization
point, not an unbounded retry loop.

Candidate execution remains the existing owner sequence: atomic unique reservation
plus Prepared attempt; acknowledged audit admission; required proof spend; a
definite live one-shot `Prepared → Dispatching` winner under current dispatch
fences; at most one capability call; durable terminal observation. Existing
approval/audit ordering and failure obligations are inherited in full, not
reordered by this outline. `approval:not_required` skips spending, never the
attempt ledger. Reading Dispatching, a replay, an ambiguous acknowledgement or a
restart does not reacquire a dispatch winner. Prepared cancellation/recovery must
win the abort fence before reporting definite non-dispatch. A spent approval stays
spent after abort. No cross-store atomic transaction with a provider is claimed.

The decision table stops before creation/spend/dispatch. `candidate_new` means
only that the next existing candidate checks may proceed; it grants no send.
`observe_winner` delegates to the actual winner's admitted state, not assumed
success. Thus every row has zero **extra** sends/spends during observation.

| Case | Decision | Extra dispatches | Extra spends |
|---|---|---|---|
| policy-before-stale | not_granted | 0 | 0 |
| scope-before-stale | forbidden | 0 | 0 |
| revision-before-key | stale_description | 0 | 0 |
| disabled-before-key | forbidden | 0 | 0 |
| input-before-key | invalid_input | 0 | 0 |
| key-required | invalid_input | 0 | 0 |
| none-rejects-key | invalid_input | 0 | 0 |
| natural-rejects-key | invalid_input | 0 | 0 |
| exact-replay | retained_result | 0 | 0 |
| pending-waits | wait_original | 0 | 0 |
| quarantine-never-resends | outcome_unknown | 0 | 0 |
| conflict | idempotency_conflict | 0 | 0 |
| revoked-before-delivery | forbidden | 0 | 0 |
| missing-approval | approval_required | 0 | 0 |
| refused-approval | approval_refused | 0 | 0 |
| spent-approval | approval_replayed | 0 | 0 |
| winner-after-miss | observe_winner | 0 | 0 |
| conflict-after-miss | idempotency_conflict | 0 | 0 |
| unreadable-after-miss | outcome_unknown | 0 | 0 |
| preflight-winner | observe_winner | 0 | 0 |
| expired-new-approval | approval_replayed | 0 | 0 |
| candidate-new | candidate_new | 0 | 0 |
| unreadable-ledger | outcome_unknown | 0 | 0 |

## State, retention and uncertainty

The following is an observation inventory of actual shared ESS states. A state
name is never itself a fresh dispatch permission. An active Prepared attempt may
still be owned by a worker; a duplicate cannot label it not_attempted simply by
reading it. Only an authorized definitive observation or the owner’s successful
fenced abort resolves that question. Dispatching means the gate may have opened,
not that provider bytes certainly arrived. Missing gate acknowledgement remains
unknown; recovery never sends the anchored request.

| Owner | State | Observation | Retention | Dispatch permission |
|---|---|---|---|---|
| attempt | Prepared | unknown_until_observed_or_fenced_abort | audit_independent | false |
| attempt | Dispatching | unknown_without_definitive_evidence | audit_independent | false |
| attempt | Aborted | not_attempted | audit_independent | false |
| attempt | Completed | applied | audit_independent | false |
| attempt | Failed | refused | audit_independent | false |
| attempt | Indeterminate | unknown | audit_independent | false |
| reservation | Pending | wait_original | no_automatic_expiry | false |
| reservation | Replayable | retained_result | settlement_plus_86400_seconds | false |
| reservation | Quarantined | outcome_unknown | no_automatic_expiry | false |
| reservation | Expired | candidate_new | known_result_only | false |

Known Completed/Failed/Aborted settlement sets Replayable with the original trusted
settlement instant and fixed expiry at `settled_at + 86400 seconds`. Replay does
not extend retention. At exact expiry a known result may be retired; subsequent
submission is potentially new intent requiring current admission and fresh
approval, never an automatic retry. Pending and Quarantined have no automatic
expiry, eviction-to-make-room, reset or reconciliation in this profile. Trusted
clock uncertainty across restart or a wall-clock jump extends retention rather
than proving expiry. Settlement recovery preserves the original settlement time.
Replacement/expiry compares reservation generation identity, so a stale waiter or
collector cannot act on its replacement. Replay storage retirement does not
remove the independent attempt/audit record or its retained binding identity.

Keep three viewpoints distinct:

- A caller that loses its MCP reply has no definitive observation, even if the
  host durably stored success. If the transport cannot answer, no fictional error
  frame is claimed delivered; the caller must retain uncertainty. A later
  explicit admitted keyed observation may recover the actual original result.
- The live host with a definitive provider answer retains known `applied` or
  `refused` even if terminal persistence/audit or safe delivery fails. Do not turn
  that knowledge into unknown, invent acknowledged audit or promise durable replay.
- Recovery lacking that definitive evidence reports unknown after possible
  dispatch. Indeterminate/Quarantined never becomes successful or redispatchable
  merely because another MCP request arrives. Generic 5xx, timeout, malformed
  output, partial work or cancellation is not proof of a definite refusal.

Preserve the original service status/result/code, classification and original
attempt/request correlation. `replayed` is delivery metadata, never a replacement
classification. Known applied effect plus an error stays applied; unknown is
`outcome_unknown` and cannot be success. Current delivery audit and original
business audit remain distinct; incomplete/unavailable audit does not reverse
known effects or fabricate acknowledgement. Credentials remain locally owned.

## MCP observations and error distinctions

Use projection's full safe service Response in the resolved tool result,
`isError:true` for an application error and matching structured/text values.
Its exact primary/legacy result discriminants stay with projection. Errors before
resolved tool execution use its safe JSON-RPC carrier while retaining the shared
code/data where admitted; no generic protocol string erases the business fact.
A protocol-only malformed request remains a protocol error, not a false recorded
attempt. Original mutation metadata is absent before admitted original observation.
These five required codes are checked against the actual closed shared vocabulary
and the already reviewed projection mapping:

| Error | Resolved tool channel | Observable meaning |
|---|---|---|
| approval_required | tool-error | A new attempt lacks required approval; an admitted retained replay needs none. |
| approval_refused | tool-error | New-attempt approval verification failed; disclose no failing axis. |
| approval_replayed | tool-error | The approval was spent; a new attempt cannot spend it again. |
| idempotency_conflict | tool-error | A live namespace/key has a different fingerprint; disclose neither result nor differing axis. |
| outcome_unknown | tool-error | No definitive authorized observation establishes the original effect; never redispatch automatically. |

Other current policy, revision, target, storage and native errors retain the full
projection vocabulary. Diagnostic timeout/unavailable causes do not replace
`outcome_unknown` or manufacture not_attempted. Approval and conflict messages
remain safe and do not identify a private failed axis.

## Ownership, prerequisites and proof boundary

`KeyReservation` references one immutable `AttemptRecord`; replay expiry owns
neither attempt deletion nor audit retention. `AttemptRecord` references its
existing configured instance and Connection. These are existing links, not a new
MCP request ledger or caller relation. The configured single-owner placement is
unchanged. All eight unresolved MCP relation markers and the absence of a selected
session-to-Connection relation remain intact. In particular,
`decision-blocker:mcp-caller-connection-assignment` is not resolved by a business
key, a qualified tool name or this contract. Outbound process ownership is also
unchanged. No host actor/tenant assignment, public approval capability or persistent
product type is introduced.

Sources: [operations §§4–5.2](../../../../../contracts/operations/v1alpha1/semantics.md),
[local mutation coordination](../../../../../contracts/service/local-mutations.md),
[canonical approval subjects](../../../../../contracts/service/delegation.md#2-canonical-subject-and-revisions),
[governed refusal precedence](../../../../../contracts/service/v1alpha2/semantics.md#321-visibility-lookup-and-refusal-precedence-e12),
[service observation encoding](../../../../../contracts/service/compatibility.md),
[attempt model](../../../../../ess/domains/mutations.yaml),
[reservation model](../../../../../ess/domains/idempotency.yaml), and
[projection](projection.md). Protocol version and archived schema citations remain
with the reviewed projection; this document adds no upstream protocol guarantee.

The Rust guard executes only document consistency and copied-input negative
controls. It derives actual lifecycle states, namespace/fingerprint field sets,
idempotency kinds and error vocabulary, and requires complete unique Markdown
mapping tables. It demonstrates that authored contradictions and extra dispatch
claims are rejected; it does not run an MCP server, provider, policy engine,
canonical encoder, atomic ledger, approval verifier, recovery process or clock.
Future executable conformance must prove concurrent same-key single dispatch,
qualified scope separation, every fingerprint conflict, winner-after-miss races,
current admission on delivery, all three lost-reply viewpoints, protected proof
custody, dispatch/abort fences, exact retention boundaries, restart/quarantine and
both protocol codecs. Until that complete binding is verified, mutation tools
remain absent from inbound advertisement.

## Observer-specific document cases

These rows state the specified observer's knowledge. In caller-lost-reply, the
fixture records that the host stored applied but the caller received no reply:
that caller cannot yet claim known effect or known replayability. A later admitted
lookup may establish both. In live-applied-audit-failure, terminal business evidence
is durable while delivery audit is incomplete: the known business result remains
replayable under current admission, and the safe error preserves applied. A
positive reply_received value means the observation can be represented, not that
a real MCP response was delivered by this test. Audit details and actual delivery
still require executable binding tests.

| Observation case | Classification | Code | Known result replayable | Reply received |
|---|---|---|---|---|
| caller-lost-reply | unknown | outcome_unknown | false | false |
| live-applied-store-failure | applied | unavailable | false | true |
| live-applied-audit-failure | applied | unavailable | true | true |
| recovery-after-dispatch | unknown | outcome_unknown | false | true |
| fenced-abort | not_attempted | timeout | true | true |
| definitive-refusal | refused | forbidden | true | true |
