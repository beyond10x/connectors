# Local mutation ledger

The host's SQLite mutation store implements the attempt and keyed-reservation
metadata group from [design section 31](design.md#31-host-persistence-ownership-and-atomicity).
It is an internal Rust embedding port. The CLI still offers reads; this store alone
does not admit a business write.

Its model owners are [mutation attempts](../ess/domains/mutations.yaml) and
[key reservations](../ess/domains/idempotency.yaml). The normative execution and
retention rules remain [operations sections 4 and 5.1](../contracts/operations/v1alpha1/semantics.md).
The new settlement clock value states a host evidence requirement; structural ESS
validation does not qualify a clock or establish durable dispatch behavior.

## Authority and persistence

Version 4 extends the existing private SQLite authority, preserving the bytes and
digests of migrations 1–3. Reads do not create or upgrade missing state.
Only admitted mutation preparation installs version 4; ordinary setup and read
management retain version 3 and can inspect recognized version 4. WAL, FULL
synchronization, owner-only files and bounded lifecycle locking remain the
[existing metadata binding](local-runtime-foundation.md).

The trusted host supplies resolved namespace and fingerprint values. They are
never deserialized from a public request as caller authority. The exact key is
the contract's tagged `mutation-key/v1` JSON array, preserving nulls and opaque
key bytes. Fingerprints use `mutation-request/v2` with every coordinate retained;
the operation is qualified by a tagged tuple of instance, adapter and operation.
Digests alone do not decide tuple equality. Registry instance and connection
references retain historical meaning after revocation; no cascading delete or
unqualified operation foreign key is introduced.

Preparation atomically inserts Prepared and its optional Pending key. Only a
definite commit returns the opaque preparation handle. An existing-key result
returns observation data, never a handle. The separately acknowledged gate
consumes that handle, competes with abort, and returns its non-Clone receipt only
to a definite live winner. Handles bind the metadata authority and process ID;
they cannot be reconstructed from persisted identifiers or used after fork.
The local connection revision, configuration and publication fence must still
match at the gate. None of this substitutes for caller policy, credential-use
admission, approval spend or acknowledged audit admission.

The store neither calls providers nor gives them metadata or credentials. A
future coordinator must combine its receipt with those other admitted
capabilities and enforce one send. Approval spending and audit admission retain
their separate acknowledgements; they are not silently bundled into preparation.
The provider call is always outside a metadata handle or transaction.

## Observation, recovery and time

Current result-disclosure admission must precede a lookup and be checked again
before public delivery. An exact key hit preserves the original request and
attempt. A different fingerprint yields an axis-free conflict. The coordinator
must repeat authoritative lookup before returning an approval/preflight refusal
after an earlier miss. These are host obligations, not grants made by storage.

Terminal settlement atomically fixes the attempt outcome, bounded safe payload
and Replayable/Quarantined key state. The first terminal observation is immutable.
Only native evidence can justify applied or refused; transport status alone is
insufficient. If settlement fails after a definitive native answer, the live
caller retains that answer and reports persistence failure separately. Recovery
without it can only record uncertainty.

Recovery takes an exact attempt reference after the coordinator has established
its recovery authority. Prepared is fenced to Aborted; Dispatching becomes
Indeterminate. Recovery never returns a gate receipt or sends. Indeterminate keys
remain Quarantined without automatic expiry or reconciliation.

Known settlement requires a host clock interval containing actual current time.
The interval is bounded to four seconds and sampled inside the transaction. The
upper bound fixes conservative settlement time; checked addition fixes expiry
86,400 seconds later. Only a current trusted lower bound at or beyond expiry may
retire the exact reservation generation. The store records a clock watermark,
rejects regression, and never infers expiry from an unavailable clock or read.
Clock failure during known settlement leaves the earlier durable state intact;
it cannot erase the live caller's known effect. Unknown settlement needs no expiry
clock. The port has no default SystemTime clock: local production qualification
remains required before keyed writes can be advertised.

Retention does not slide on replay. A stale retirement request compares both
the exact tuple and reservation ID, so it cannot retire a replacement. Retiring
a reservation never deletes the attempt identity or an independent audit record.
Capacity refuses new attempts/reservations instead of evicting unresolved ones.
The default is 10,000 retained attempts per instance and 256 KiB per safe result;
trusted configuration may select 1–100,000 attempts and 64 bytes–1 MiB per result.
Expiry releases the live-key index slot, retaining the attempt and safe outcome;
it does not reclaim the independent historical attempt capacity. Metadata records
and safe replay payloads have explicit byte bounds; provider
credentials, approval evidence and raw input have no storage field.

## Delivery boundary

The SQLite tests exercise durable faults and races using a deterministic trusted
clock fixture. A test counter can demonstrate receipt use, but is not a native
GitLab write. Local approval issuance/verification/spending, audit persistence,
production clock qualification, generated mutation CLI/wire surfaces and their
fully admitted dispatch composition remain the next GitLab foundation work.
Dedicated GitLab sandbox acceptance and the C14 native create/update head guard
are still open. GitLab precedes Kubernetes, PostgreSQL, MCP and remaining providers.
