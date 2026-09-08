# read-refresh-once/v1alpha1

**Status: proposed binding, not implemented or selected by any current adapter.**
This document resolves F15/E05 for `story:contracts-read-refresh-retry`. It composes
[capability](semantics.md), [refresh coordination](../../acquisition/v1alpha1/semantics.md#41-refresh-exclusion-authorization-and-recovery)
and [generation-bound evidence](../../evidence/v1alpha1/semantics.md#4-rules).
[Service compatibility](../../../service/compatibility.md) owns framing and public
error codes. The existing configured service retains its no-retry behavior.

## 1. Explicit selection and compatibility

`read-refresh-once/v1alpha1` changes an invocation's observable behavior. It is
available only through the proposed service **v1alpha2** binding and an explicitly
supported, authored native read profile. For a base native profile `P`, the combined
profile's exact public name is `P.read-refresh-once.v1alpha1`; `P` must be nonempty,
fit the selected descriptor's string bounds with this suffix, and must not itself
be a retry-combined profile. The adapter must author the combined profile under its
own contracts, retaining the precise native request/result schemas and semantics
of `P` and explicitly selecting every requirement here. A name ending in the
suffix does not establish implementation support, read safety or authority.

The receiver advertises that exact combined name in the existing singular
`Operation.profile`. It advertises the required managed credential profile through
the existing `requires_auth` and safe `Descriptor.auth_profiles` projection. The
client must support the complete combined profile and selected native schemas
before invoking the described operation with its exact projection revision. A
receiver offering both behaviors uses separately described operation selections;
it never chooses retry behavior from a request header, input member, guessed
profile suffix or a library default. No `retry`, `profiles[]` or other wire member
is introduced, and the adapter-kind reader gains no implicit field.

Changing an operation from `P` to the combined profile changes its descriptor
meaning/revision. An old revision refuses before provider dispatch. Existing
profiles, including `P` on v1alpha2, remain without automatic redispatch unless
their own separately versioned contract explicitly selects it. Legacy projection
must omit the combined profile; retaining old schemas or a read-only effect does
not make its changed authentication, errors and call count legacy-compatible.
Unsupported/unimplemented combinations are omitted and cannot be reached through
a guessed operation id. Dynamic connection readiness still follows the service's
separate metadata and invocation rules.

This repository currently advertises no such combined profile. Supporting the
v1alpha2 codec or renaming `AuthenticatedHttp` to `http-bearer` does not select it.

## 2. Eligibility and immutable invocation

The first binding admits only directly executed, credential-bearing HTTP bearer
reads on one managed connection whose provider-owned auth profile explicitly
supports the selected refresh grant and proves same-identity refresh lineage.
The native profile must establish that the business request is read-only; `GET`
or absence of a mutation flag alone is not that proof. One invocation constructs
one bounded GET request, with no business body, pagination, continuation, fan-out,
stream or caller-visible partial delivery. Permission checking may still require
the finite exact target set governed by evidence §4.4.

Mutations, process/session/media operations, generic realizations, anonymous,
static_config, client-credentials reacquisition, exec-plugin refresh and mediated
or federated execution are excluded. A gateway cannot repeat a forwarded read
under this binding, and a child cannot refresh a parent's material. Unsupported
combinations refuse before dispatch; they do not fall back to another connection,
auth scheme, receiver or profile. Ordinary mutation handling may separately follow
its admitted maintenance policy, but this binding grants it no refresh or repeat
business dispatch.

Initial activation/revalidation must already have established the connection's
identity and current credential generation. This invocation grants no implicit
identity or verify-operation probe, browser action or new acquisition. Refresh
validation must establish credential validity and grants from the bounded auth
result and the provider profile's proved lineage under evidence §4.3. A profile
requiring another provider identity/verification call to validate publication does
not support this first binding. Required verification for the successor must
already be independently established for that generation or redispatch refuses;
it is never copied from the source generation or performed implicitly here.

At initial admission retain one immutable invocation context: request id, selected
instance/operation/native and combined profile, descriptor meaning, connection,
provider authority/destination, auth profile, expected stable identity, caller and
verified executor scope, exact permission targets, canonical business request and
original limits. The second business request has the same method, admitted path,
query and application headers. Only the separately admitted credential placement
changes; a capability cannot rebuild a broader target from mutable configuration.
The read observes live provider state and promises no snapshot between dispatches.

## 3. One bounded sequence

1. Establish current host operation/connection/result admission, required evidence
   and permission coverage. A required preflight failure sends no business read.
   An unavailable, expired, replaced or unresolved-refresh generation refuses;
   there is no proactive refresh or auth upgrade inside this binding before the
   first read. Any separately admitted maintenance is outside this invocation.
2. A new generation-bound `DispatchAdmission` opens the first business dispatch
   once. Every result other than a **definitive, complete, bounded provider HTTP
   401** is terminal under the native read's normal result/error rules. A transport
   failure, timeout, redirect, 403, 429, 5xx, malformed response, response overflow
   or unknown status is not a refresh trigger. No partial result has been emitted.
3. For that one 401, recheck current host admission and the original immutable
   binding. The host may participate in **one** refresh attempt for the generation
   actually used by the first dispatch. It observes an existing attempt or creates
   one under acquisition §4.1; it cannot authorize a second exchange, poll the
   provider or enter a new attempt after the selected attempt is fenced/refused.
   Metadata waiting is bounded by the original deadline. Only the live coordinator
   invocation holding a committed exchange authorization can send the one token
   request. Other invocations wait or refuse; observing/replaying authorization
   never grants send authority.
4. Continue only after observing acknowledged `Published` for that source attempt,
   with its exact successor generation still current and the original binding and
   stable identity preserved. A concurrent publication already committed after the
   first dispatch may satisfy this step without a new token request. A configured
   replacement, reassignment or unrelated generation is not that refresh result.
   If the selected successor has itself been superseded, refuse; do not follow a
   chain of generations or refresh another source inside this invocation.
5. Apply evidence §4.3 to the new generation, including recomputed validity/grants,
   any permitted identity transfer with unchanged age, and invalidated permission
   and verification evidence. Recheck current host policy and exact target scope;
   obtain required current-generation permission evidence only within the original
   invocation's remaining allowances. Create a **new** `DispatchAdmission`, pin its
   exact material and atomically order its dispatch check against publication,
   replacement and revocation. The first admission is never reopened or repointed.
6. If every check and remaining limit permits, open exactly one redispatch. Its
   result is terminal, including another 401. There is no second refresh,
   credential reacquisition, alternate target, hidden retry or third business call.

The first 401 is an internal observation only if the sequence can continue; token
responses and private generation/attempt/fence details never enter public results
or errors. All HTTP/provider clients disable automatic retries and redirects,
including auth-library challenge/refresh resends. A retrying transport cannot
advertise this binding merely because the outer counter is bounded.

## 4. Shared time, call and byte budgets

The combined profile fixes the existing ordinary descriptor ceilings:
`{request_bytes: 65536, result_bytes: 4194304, execution_ms: 20000,
provider_ms: 15000, connect_ms: 5000}`. It is not a generic realization and does not
inherit that realization's larger limits. Native payload/target bounds may be
smaller as authored in the combined profile. A receiver cannot advertise the fixed
descriptor limits and silently substitute different execution limits; an earlier
inherited deadline or cancellation still wins. No new public limit fields or
caller overrides are introduced.

Let `T0` be the original adapter-execution start, as in service v1alpha1. The
absolute execution cutoff is `T0 + 20 s`; the provider-work cutoff is `T0 + 15 s`,
or an earlier inherited cutoff. Initial permission collection, first
read, refresh wait/exchange/publication observation, new permission collection and
redispatch all share that one provider-work cutoff. Each connect attempt is at
most `min(5 s, remaining provider-work time)`. Policy, custody, audit and local
coordination remain inside the original execution cutoff. No phase, joining
caller, new generation or fresh admission starts another 15/20-second window.
At cutoff or cancellation no later business dispatch may open.

| Resource across this one invocation | Ceiling and accounting |
|---|---|
| Business provider requests | 2 total: the initial request and at most one redispatch; reserve/consume a slot before any possible send, including failed sends |
| Refresh participation / token requests | One selected source attempt; at most one token exchange globally for that attempt/source authorization, including concurrent participants; a waiter sends zero |
| Permission targets / authorization requests | Existing F08 ceiling of 64 distinct targets, 64 calls and four concurrent checks, with smaller configured bounds preserved; **one original ledger** covers both phases |
| Other provider requests | Zero identity/verification/discovery/maintenance probes; eligibility and evidence requirements above remain admission conditions |
| Maximum provider HTTP requests attributable to the sequence | 67: at most 2 business + 1 token + 64 authorization requests; a shared token request is still one physical exchange, not one per waiter |
| Business/provider buffers | Public request and each complete serialized result retain 64 KiB / 4 MiB bounds; each provider business response is independently bounded at 4 MiB encoded and 4 MiB decoded, so the two responses consume at most 8 MiB in each accounting dimension |
| Auth provider buffers | Token and authorization request/response bodies are each at most 64 KiB in this first binding, or a smaller native bound; encoded and decoded response reads are bounded independently |

Body ceilings do not bound all HTTP metadata. The adapter-owned combined profile
contract must state numeric limits for request URL/header bytes and header count,
response status/header-line bytes, aggregate header bytes/count, and trailer/framing
bytes/count. Its provider-owned auth binding must select and enforce those limits
for the token and authorization clients as well as the business client. Missing
numeric declarations or a transport unable to enforce them makes that combined
profile unsupported. These are adapter authoring/transport obligations, not new
shared descriptor or adapter-kind fields supplied by this document.

The original exact authorization target selection is not expanded after refresh.
For call accounting, retaining the same resource target across a generation change
never resets its consumed per-invocation slot. Permission cache identity still
includes the generation: old-generation allows are invalid for the successor.
For example, if target X required an authorization call before the first business
read, another call for X is forbidden in this invocation. Use independently
established, fresh successor evidence if available, otherwise refuse `unavailable`.
If X was initially satisfied from a cache and its call slot remains unused, one
new-generation check is allowed after reserving the complete missing-check budget.
Cache hits still count toward the original target ceiling. F08's no-check-retry,
all-required-checks-before-dispatch and refusal/coverage rules remain unchanged.

Refresh does not refund consumed response bytes, network/check slots or elapsed
time. Native parsing/response bounds apply before a 401 is accepted as the trigger.
An auth profile needing larger buffers or undeclared checks is unsupported by this
first binding, rather than an implicit exception to its budget.

## 5. Terminal outcomes, authority and audit

Current host lookup/connection/result authority is required before disclosing any
refresh or business observation. Apply service v1alpha2's refusal precedence;
permission to inspect an earlier result is not permission to use its credential.
The following table assumes the caller still has that observation authority.

| Observation after the initial 401 | Public outcome / remaining business calls |
|---|---|
| Published successor, current identity/scope/evidence/binding and remaining budgets all admit | One redispatch; its native result or error is final |
| Second business 401 | `unauthorized`; no refresh or further business call |
| Refresh `reauthorization_required`, `invalid_or_revoked` or `uncertain`; invalid identity/lineage; discarded/unusable candidate; unresolved consumed source | `connection_not_ready`; no redispatch; the coordinator's conservative repair/reauthorization state remains authoritative |
| Refresh `insufficient_scope` or narrower established grants fail the operation | `insufficient_scope`; no redispatch or permission widening |
| Refresh `custody_unavailable`, coordinator metadata unavailable or unknown publication acknowledgement without a verified usable publication | `unavailable`; no redispatch; a possibly consumed source is never reopened |
| Required current-generation permission evidence unavailable, or remaining target-call budget cannot supply it | `unavailable`; no redispatch and no extra check |
| Exact target permission/resource scope denied | `forbidden`; no redispatch |
| Current host policy denies / is unavailable | `not_granted` / `unavailable`; no redispatch or private state disclosure |
| Source attempt fenced before exchange, or its published successor already superseded again | `unavailable`; no new attempt, chain traversal or redispatch |
| Original execution/provider-work cutoff before the next allowed action | `timeout`; no later dispatch; a required permission-check timeout retains F08's `unavailable` while the outer execution deadline remains open |
| Caller gone/cancelled | Stop this invocation; deliver no later result or redispatch; finalize/observe only the already-authorized coordinator protocol as its owner requires |

A normal second-read 404, 403, 429, 5xx, malformed response or timeout keeps the
native read's selected mapping. Refresh success does not turn such a result into
success, `not_found` into an auth failure, or a read into mutation `outcome_unknown`.
Unknown refresh/publication facts remain private causes mapped above; they are not
new ErrorCode members or text-encoded status fields.

There is one business invocation and one correlated response, not an automatic
second `/invoke`. Audited reads retain acknowledged admission before business
use and one final observation under service compatibility §5. The receiver must
retain enough safe audit observations to distinguish the first 401, whether a
second business dispatch opened, and the final result; failed/ambiguous audit
acknowledgement never grants redispatch. Final-audit failure preserves the known
read outcome with `audit_status: incomplete`, as already specified. Private token
material and generation/refresh-ledger references are excluded from public audit
projection. Coordinator durability/recovery cannot revive an expired/cancelled
business invocation or replay its credential exchange.

## 6. Conformance and model boundary

The [textual case audit](../../../../docs/evidence/read-refresh-retry-20260909/verification.md)
covers the old/new reader distinction, complete success/refusal sequence, races,
budgets and excluded modes. It specifies expected observations; it is not proof
that a runtime implements this binding.

The existing [RefreshAttempt](../../../../ess/domains/refresh.yaml),
[CredentialGeneration](../../../../ess/domains/credentials.yaml) and transient
[DispatchAdmission](../../../../ess/domains/credential_evidence.yaml) models retain
their identities, references and lifecycles. This sequence uses one source attempt
and separate terminal dispatch admissions; it introduces no new persisted owner
or lifecycle. ESS compilation validates those declarations, not the cross-model
predicate that permits the second dispatch. **UNMAPPED implementation obligations:**
exact profile/projection support, the immutable request/budget context, bounded
refresh waiting, provider lineage proofs, consumed authorization slots across
phases, audit observations and dispatch/cancellation atomicity. None can be
advertised solely because the shared model compiles.
