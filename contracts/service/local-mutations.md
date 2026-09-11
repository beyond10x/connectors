# Local approval and mutation coordination

Binding for the next GitLab runtime slice. The policy metadata/lease port is
implemented in `crates/connectors-host/src/local/approval_policy.rs`, with trusted
CLI policy admission, subject preparation and protected issuance in
`crates/connectors-host/src/local/owner/approval_issuance.rs`. The development CLI
joins guarded GitLab merge through explicitly selected private protocol two;
the public service remains read-only. The wider failure and sandbox acceptance
remain implementation work. This
composes [approval proofs](delegation.md), [issuer custody](approval-issuers.md),
[bounded time](clock.md), [audit](audit.md) and the
[mutation ledger](../operations/v1alpha1/semantics.md). It adds no new business
attempt, redemption or credential owner. Native adapters still own provider
preconditions and interpretation of actual effects.

## Published local policy

An explicitly managed local approval policy supplies the missing issuance and
receiver authority. It is separate from signing-key possession and from mutable
TOML admission settings. At most one policy identity is retained for a configured
service instance, referencing that instance and its existing local issuer. It
has a random non-nil UUID and a positive, monotonically increasing integer
revision in 1..=9007199254740991, never wrapped or reused. Exhaustion refuses
publication without changing the retained policy. Its record contains the configured owner UID,
exact native configuration/descriptor revisions, executable selection digest,
clock-configuration digest and a sorted distinct set of at most 256 write
operation IDs. Empty operations revoke all new local approval/write admission.
Each listed operation must exist with a supported required-approval write profile;
unknown, read-only and event-claim operations cannot enter this policy.

`approvals policy-set --adapter <alias> --input-file <file>` accepts a closed
`{operations:[...]}` document. The first set requires absence; subsequent sets
require `--expected-revision`. The host resolves all binding coordinates from
current private configuration, admitted cached native metadata and the retained
issuer. Caller input cannot select a principal, issuer, clock key or binding
revision. `approvals policy-status` reads the current public policy without
migration, secret access or service startup. Policy-set is an owner-authorized
management action, not a provider operation or evidence that a human reviewed
each future input. It installs its own versioned SQLite migration only when
admitted. Existing migrations remain byte-identical and WAL/FULL remains required.

Replacing the policy uses a bounded exclusive process-shared lease plus one
durable revision CAS. Current policy uses hold the corresponding non-clone,
process-bound shared lease through issuance publication or the final dispatch
commit. Lock order is policy, issuer/key, custody, then bounded metadata handle;
close metadata before secret/provider/clock-network I/O. A busy update refuses
within its budget; it does not claim revocation. An uncertain storage acknowledgement
grants no success. A later status reads the authority; it cannot reconstruct a
use lease. No policy identity or revision is reclaimed/reset to evade old proofs.

TOML profile/operation permissions continue to restrict admission. Editing TOML
does not replace the published write policy or revoke an already admitted use;
policy-set is the serialized write-policy update. A new request must match the
current configuration, descriptor, executable and clock selections as well as
the published policy. Changed selections refuse until explicitly republished.
Existing read configuration behavior remains unchanged. Privileged tampering
with SQLite, locks, processes or kernel identity is outside this local trust
profile, as it is for the existing credential and issuer bindings.

## Caller and canonical subject

The local principal is the configured owner UID authenticated by private file
ownership and Unix peer credentials. Its stable public reference is
`connectors.local-owner/<public issuer UUID>/<canonical decimal UID>`. The
issuer UUID is already public and permanently bound to this service instance;
private custody coordinates and the metadata authority UUID are not exposed.
Tenant, realm and executor are absent. Remote principals or caller-supplied
identity headers/fields cannot use this profile. Cloned authorities with divergent
history remain forbidden by the persistence contract.

The host reconstructs the existing CanonicalApprovalSubject from admitted
metadata and the original strictly decoded business input. The origin is direct,
route is null, and current_authority is the snapshot
`{id:"connectors.local-policy/<policy UUID>/<revision>",sha256:<canonical policy digest>}`.
The approval uses required mode and the existing canonical JSON/input digest.
Policy changes therefore invalidate new uses of old proofs without changing the
stable caller namespace or permitting an old business key to name a new effect.
Historical snapshot strings do not constitute a lookup or current grant.

`approvals prepare` takes the selected adapter, explicit saved connection,
operation, descriptor revision, schema identity and input carrier. It returns the
existing ApprovalPreparation value, with its exact canonical subject and digest.
It reads admitted metadata only: no time query, provider credential, provider
verification, owner/child startup, key reservation, business attempt or approval
spend. An unavailable/revoked connection or unsupported target refuses. Expired
baseline evidence may still describe the exact retained identity; preparation
never establishes readiness. The preparation limits and later independent input
size checks are those in delegation's helper contract. This local command does
not advertise the public delegated helper protocol.

`approvals issue` accepts the same target/input plus `--approve-subject <digest>`
and `--proof-output <path>`. It independently reconstructs preparation; a different
digest refuses. The authenticated local owner's explicit issuance action approves
this exact subject under the current published policy. No assertion of an
independent human interaction is made. A supplied preparation file or friendly
label is never the source of authority. This allows deliberate machine issuance
under the operator's local policy as well as manual owner use.

Acquire fresh authenticated time before policy/key/custody leases. Under current
policy and active-key leases, read the exact qualified seed and issue the fixed
300-second proof profile. Publish a closed `{reference,evidence}` document to a
new private regular file under an owner-checked private directory, without
overwriting an existing path or following links; synchronize file and directory
before success. The proof is never printed, logged or stored in SQLite. Return
only its safe reference, subject digest and definite publication disposition.
Failure/uncertainty grants no issuance-success claim; a leftover protected file
is not automatically reissued, overwritten or deleted. Key/policy/time changes
are rechecked before acknowledgement, and the leases remain held through it.

## Invocation, original-result admission and audit

The authored local operation interface gains an optional opaque idempotency key
and an independent protected approval-document source. The existing private
protected-source pattern retains proof bytes outside ordinary parser JSON values.
Provider input contains neither. Reads reject write-only approval/key fields;
legacy describe/invoke/serve wire values are unchanged and gain no write path.
Required missing, refused and replayed approvals have distinct safe failures.

Every local write invocation receives a fresh host request identity. Resolve
current caller, policy, target, input schema and result-disclosure admission before
looking up an exact business key. Follow the existing namespace/fingerprint and
winner-recheck rules. An admitted existing result is observed without new provider
preflight, credential access, proof verification/spend or dispatch. Conflict,
pending and quarantine never authorize a new send. Result disclosure still needs
current policy and target admission; it cannot be obtained by naming an old key.

An unsettled exact-key observation may start the local owner for metadata recovery.
A busy instance worker returns the pending observation. Otherwise recovery is
queued on that instance's one retained worker; earlier native exchanges and their
non-clone preparations/gate winners must have ended before it runs. A private
exclusive borrow of that worker's child slot binds this quiescent interval. The
owner retains its lifetime lock until kernel process exit, including unwinding,
so a replacement process cannot overlap an old dispatcher. A busy/idle hint alone
is never recovery ownership.

The recovery task never starts a native child, lifts stop suppression, reads
credentials, verifies/spends another proof, or sends provider work. Starting an
owner still honors separately configured automatic startup; with on-demand
adapters the recovery path starts no native child. Under current result-access
policy it applies the existing exact-attempt fences: Prepared becomes Aborted;
Dispatching becomes Indeterminate with a quarantined key. Known non-dispatch
settlement requires a fresh configured trusted-clock sample to establish the fixed
replay retention interval. Acquire that sample before policy/metadata locks. If
time is unavailable, leave Prepared pending and report uncertainty with its safe
cause. Quarantine needs no clock and has no expiry. A failed/uncertain recovery
acknowledgement grants no send or immediate retry; observe the authoritative original once.
Terminal results retain their passive path without owner startup or clock access.
The original invocation budget includes owner startup, queueing and recovery.

The owner also runs periodic metadata recovery without requiring a caller to
return with a key. It waits five seconds between read-only passes using the existing
instance/state index, visiting at most eight instances and returning at most 64 pending
references for one instance. Its transient instance/attempt cursor rotates across
retained records, including unkeyed attempts. Scanning never installs a mutation
schema or reads stored result payloads. Metadata handles close before clock I/O.

For a live instance worker, at most one maintenance batch is queued on that
retained worker. Earlier native exchanges must finish before it can apply a
recovery fence. With no live worker, the supervisor instead holds its
worker-creation lock while recovering that instance; a definitely exited retained
thread supplies no dispatch authority and is not replaced implicitly. Each
transaction checks the exact retained attempt/instance binding. The batch checks
a 250 ms work budget between attempts, with the existing bounded metadata wait
inside an attempt; remaining records are revisited by later scans. Owner cleanup
stops and joins the maintenance thread as well as its instance workers.

This housekeeping fences existing records even after a connection is revoked or
an adapter is removed from configuration. It does not disclose their results,
spend proof, resolve credentials, launch an instance worker or adapter child, lift
suppression, expire a key, delete history, or create new business authority. Known settlement still
needs the currently selected trusted clock; absent/changed time leaves Prepared
pending while Dispatching can be quarantined without time. A later scheduled
pass may revisit an unresolved attempt only after fresh positive pending evidence,
never merely because an earlier acknowledgement failed. Current admission still
governs every subsequent result request. Pending/quarantined entries and storage
pressure never grant reuse. Original-audit reconciliation and the complete
acknowledgement-failure matrix remain separate required work.

For a new candidate, verify required proof, current credential readiness and native
input. Acknowledge the admitted execution audit anchor before native preflight,
because its declared reads are provider dispatches under the audit contract.
Native preflight cannot write. After it returns the exact immutable native
preparation, acquire/recheck current time, policy/key and connection-bound use.
Confirm the original acknowledged audit receipt and its unchanged facts, then atomically
prepare the attempt and any business-key reservation. Spend approval for that exact
Prepared attempt, acknowledge the connection-use dispatch admission, then win the
separate durable Prepared-to-Dispatching gate under the still-current binding.
Only definite acknowledgements and the live gate winner permit the child commit.
The original monotonic budget covers every phase. Leases, preparation or retries
never restart it. A failed current check cancels the native preparation and keeps
all already-spent uniqueness facts intact.

Safe success/failure output adds the existing MutationObservation and SourceAudit
projections where authoritative admission permits them. Native results must pass
their selected schema before disclosure. Audit completion remains a separate
acknowledgement: final audit failure cannot change a known provider effect into
not-attempted or permit a resend. Store original safe outcome independently of
each later observation's current audit envelope. No raw input, proof, credential,
private locator or provider exception belongs in audit or diagnostics.

## Private preparation and one-use dispatch

The versioned private adapter extension has separate prepare and commit phases.
Prepare carries the original operation/revision/partition/deadline, captured
credential material and strictly validated input. Native code owns all provider
preflight reads and returns either a definite preflight refusal or a process-local
prepared write. The child retains its immutable typed request and credential;
the response exposes only a fresh opaque preparation UUID bound to the request,
child incarnation, input and deadline. It is not a durable/resumable session.
At most one preparation is live per owned child exchange.

Commit names that exact preparation on the same authenticated channel and
contains no replacement input, credential, target, deadline or request body.
The child consumes the preparation before any write and receives one non-clone
authenticated write capability. Generated/native dispatch consumes that capability
once. The ordinary authenticated GET capability gains no write method. There is
no preflight-to-write callback that can bypass the host's approval/audit gate.
Cancellation, deadline, EOF or child/owner loss destroys pending native state;
another process/channel can never reconstruct it.

Channel loss after the host gate or commit transmission is possible-write
uncertainty, even if the native request might not have left the child. Recovery
uses the existing attempt and key states; it cannot resend commit. A terminal
native response supplies applied/refused/unknown only under its adapter-owned
evidence rules. A definite preflight refusal reports no provider write. An
uninterpretable/lost post-commit response remains unknown. Exact-target provider
observation is a separate admitted read and cannot by itself rewrite a quarantined
attempt into an attributed success.

Old private/read and public service profiles retain their closed codecs and
read-only capabilities. The extension must be selected explicitly and tested
against old peers; no unknown field, automatic downgrade or legacy invoke path
may open a write. New format-specific framing and generation syntax are owned by
their protocol/spec-kind documents before runtime activation.

## Required verification

Prove protected issuance and consumption through production CLI handlers; policy
CAS/restart/revocation and held-use exclusion; wrong caller/subject/clock/key,
expired/missing/reused approval, schema and permission refusals; no provider work
for exact admitted keyed replay; key conflicts and changed connection identity;
every audit/spend/gate acknowledgement fault; native preparation substitution,
duplicate commit, cancellation/deadline, old-peer refusal and owned-child cleanup;
and lost write response/restart with one provider effect. Existing key and read
journeys must keep passing. SQLite, protocol fixtures and ESS do not substitute
for dedicated provider sandbox acceptance.
