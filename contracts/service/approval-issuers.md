# Local approval issuer and signing-key management

This contract extends [delegation](delegation.md). Key management supplies trusted
key configuration and protected signing material. It does not admit a caller,
authorize an approval subject, qualify a clock or dispatch a provider operation.

## Identity and current authority

Each configured service instance has at most one retained local issuer in its
metadata authority. Its public issuer UUID, private custody allocation UUID and
each key's public UUID and private material-version UUID are independent random
identities. None is reused. Public issuer and audience strings are respectively
`connectors.local-issuer/<issuer UUID>` and `connectors.approval/<issuer UUID>`.
The issuer references the independently retained service configuration; neither
record cascades deletion into the other. Adapter/configuration identity changes
refuse management until the owning configuration workflow resolves them.

The issuer's revision changes at each admitted management decision. At most one
key is Active and at most one is Candidate. Public metadata contains the issuer,
audience, revision, key IDs, Ed25519 public keys and key states. Custody locators
and 32-byte signing seeds never appear in public output, SQLite secret columns,
arguments, environment variables or diagnostics. The selected local key profile
has validity bounds [0, 9007199254740991) Unix milliseconds and explicit current-key
revocation; these bounds are not clock evidence. Proofs retain delegation's short
lifetime and require independently qualified current time.

## Acknowledgements and lifecycle

`init` stages an initial key only for an issuer with no active or pending key.
`rotate` requires the exact current key and issuer revision; a pending candidate
prevents another allocation. Stage the candidate's identity, public key and private
custody coordinates durably before attempting its immutable secret write. A
definite durable custody acknowledgement permits a separate publication transaction
that makes Candidate Active and the previous Active Retired. Failed staging or
storage never replaces the current key. Publication is a hard cutover: retired
keys cannot verify or issue new approvals, including already issued unspent proofs.

An unknown secret write or publication acknowledgement returns no success claim.
`recover` names the exact candidate and revision. It reads that exact immutable
version, checks the 32-byte seed against the staged public key, synchronizes the
qualified backing store again and publishes under the still-current candidate
fence. Recovery never repeats CreateItem or generates replacement material.
An already active key can be observed but is not a recovery candidate.

`revoke` requires the exact current key and issuer revision; it retires that key
and any pending candidate atomically and leaves no active key. A staged write or
recovery from the old revision cannot revive authority. A later explicit `init`
requires the now-current revision and creates a fresh key identity.

`retire` names an exact Candidate or Retired key and current issuer revision.
Candidate cancellation first removes its publication eligibility. Only then may
the key move to Retiring and undergo guarded exact-version deletion. A definite
deletion acknowledgement moves it to Deleted; an uncertain result retains the
Retiring fence for exact retry. An Active key cannot be deleted. Historical
metadata and uniqueness tombstones remain; a bound of 128 keys per issuer refuses
new allocation rather than expiring records or reusing identities.

All current-key uses hold a non-clone process-local shared lease before reading
key policy, through signing or approval-spend acknowledgement. Every management
decision holds the corresponding exclusive lease. Lock order is issuer lease,
physical custody lock, then bounded metadata handle; close metadata handles before
secret I/O. Receiver policy borrows an already held lease rather than acquiring
metadata from inside a spend transaction. A key lease alone never grants caller
or subject authority. Handles from another process cannot be reused after fork.

## Local binding and CLI

SQLite migration seven retains migrations one through six unchanged, WAL/FULL and
the existing metadata authority. Only admitted key mutation installs it. Key status
does not migrate, create authority/key records, start an adapter, activate/unlock
Secret Service or read a seed. Existing SQLite lifecycle locking and sidecar
bookkeeping remain infrastructure work. Configured adapter selection is resolved
by the host, not the provider.

The CLI namespace is `approvals`: `key-init`, `key-status`, `key-rotate`,
`key-recover`, `key-revoke`, `key-retire`. All take `--adapter`. Except the first init, mutating commands
require `--expected-revision`. Rotate/revoke take `--expected-key`, recovery takes
`--candidate`, and retirement takes `--key`. Status returns bounded public history.
No key command issues approval proof evidence or advertises native write support.

Signing custody uses schema `org.beyond10x.Connectors.ApprovalSigningKey`, format
`approval-signing-custody/1` and label `Connectors approval signing key`. Existing
credential custody keeps its exact schema/format/attributes. Purpose participates
in scope equality, so identical UUID tuples cannot cross purposes. The selected
Secret Service qualification, durable synchronization and exact physical deletion
acknowledgements are those of [local custody](../../docs/local-secret-service.md).
Signing-key reclamation uses this contract's Retiring fence and exclusive key-use
lease. Credential acquisition termination, credential-use expiry and the credential
24-hour reclamation delay do not apply to signing keys. An exclusive lease proves
that no current key use remains; a retired key never becomes active again.

Runtime acceptance covers restart, locked/missing custody, known and unknown write
failure, crash before/after publication, failed rotation preserving the current
key, concurrent init/rotation/revoke/recovery/use, stale revisions, exact retirement,
purpose isolation, private-output checks and migration continuity. ESS structure
does not establish those runtime guarantees.
