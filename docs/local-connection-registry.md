# Local connection registry

The host's `local::registry` module binds the existing connection, acquisition,
credential evidence and custody contracts to one private SQLite authority. It
supports static-entry profiles. Provider work and keyring I/O happen outside
metadata transactions. Adapter libraries receive authenticated capabilities;
they never open this database or select arbitrary keyring items.

The production CLI uses this authority for connection list, describe, status and
terminal local revoke. Protected connect/repair, explicit evidence revalidation
and the CLI owner's integration with the private adapter transport remain unfinished. A native
fixture publishes a fictional connection through the coordinator and qualified
Secret Service, restarts both, and exercises separate production CLI status/revoke
processes. It does not prove the persistent GitLab acceptance journey.

## Metadata and migration

Migration two retains the original local authority UUID and records its exact SQL
digest. Fresh setup installs both migrations. Passive inspection accepts recognized
headers but cannot migrate version one or interpret it as an empty connection
registry. An admitted acquisition may migrate an existing recognized database;
missing, changed or future authority refuses. SQLite uses WAL and FULL
synchronization under the [local filesystem binding](local-runtime-foundation.md).

The tables bind existing semantic owners:

| Records | Meaning |
|---|---|
| Instances and profiles | Exact configured instance, configuration revision and immutable reviewed static profile |
| Connections | Public reference/revision/identity and private publication fence, selected generation, custody version and evidence |
| Acquisitions | One-use completion owner, fixed original deadline, captured generation and allocated candidate |
| Generations and materials | Immutable capture identity and exact version coordinates; acknowledgement and retirement facts |
| Uses | Bounded read retention/admission guards; no business write approval or audit claim |
| Cursors | Opaque bounded list continuation tied to the exact selection and metadata epoch |

Before custody acknowledgement, a material row records only the acquisition's
allocated candidate. It is not a logical StoredCustodyVersion or dispatchable
credential. Neither its presence nor a successful read can fabricate a durable
write receipt. Credentials stay exclusively in Secret Service; SQLite contains
only nonsecret metadata. Public results exclude private scope, version, fence,
generation, capture and completion-owner coordinates.

Each operation holds an immediate transaction and bounded metadata handle.
A durable clock floor rejects wall-clock regression. It records time even when
a semantic action refuses, using a savepoint to roll back that action. Passive
observations may update this floor and cursor records; they do not start owners,
read credentials, migrate schema or perform provider authentication.

## Acknowledgement boundaries

1. Begin allocates a private connection/acquisition or captures an exact existing
   repair target and private fence. Public revision is not that fence.
2. Consume commits one-use completion before native validation. Original acquisition
   expiry never slides. Baseline validation must observe the consumed capture,
   selected authority, profile, identity and required native grants.
3. Prepare commits exact candidate coordinates before any secret write. The
   guarded backend rechecks current completion and fence under its physical writer
   lock, writes immutably, reads back the bytes and synchronizes storage.
4. Acknowledge records the sealed successful custody receipt. This publishes no
   connection or dispatch authority.
5. Publish atomically selects generation/material/evidence, advances the private
   fence and records the completed acquisition. Repair preserves public semantic
   revision and external identity. Lost acknowledgement is resolved through status;
   it never authorizes repeating capture, validation exchange or a provider effect.

Concurrent repairs have one publication winner. Changed identity, insufficient
scope or a failed repair leaves the still-valid prior credential selected. Native
baseline snapshots retain optional observed grants and credential expiry under
[evidence section 4.5](../contracts/auth/evidence/v1alpha1/semantics.md#45-retained-scope-and-expiry-observations).
Unknown grants/expiry do not mean an empty grant set or immortal credential. The
generic bound is 64 distinct scopes, each at most 256 UTF-8 bytes; native profiles
own implications and interpretation. Known credential expiry immediately cuts off
readiness, regardless of another evidence deadline.

Revoke serializes with publication and final read dispatch, changes the public
semantic revision, clears active authority, closes pending acquisitions and fences
retirement. It needs neither provider access nor keyring availability. Repeated
revocation returns the same terminal result. A previously dispatched read can
already be in flight; revocation cannot undo it or allow another dispatch.

## Read use and retirement

Capture pins one exact generation/material/fence and original deadline, at most
120 seconds and no later than its baseline evidence. The host must resolve that
material and check current operation policy/permission before consuming the
one-use final dispatch guard. These APIs neither implement transport nor authorize
a business mutation. Positive missing/invalid/revoked/insufficient/uncertain
credential observations cut off that exact version. Outage is never inferred to
mean missing. Restart does not resume a captured provider operation.

Replacement, terminal revoke, failure or an admitted expiry sweep establishes a
durable retirement decision. Retention is at least 24 hours from that decision,
including an unknown candidate. Physical deletion requires the exact retirement
fence, terminal acquisition, no selected active material and no unexpired use.
The physical writer lock serializes this metadata check with delayed writers.
Acknowledged deletion clears byte-size metadata but retains identity/fence history.
An unknown response retains the cleanup obligation for guarded reconciliation.

The initial bounds are 1,000 connections per instance, 10,000 total connections,
100,000 acquisition histories, eight retained material versions per connection,
1,000 concurrent read-use records and 10,000 live cursors. Exhaustion refuses;
it does not evict authority or pretend an incomplete list is complete. Pages accept
1–500 members, bind adapter/instance/configuration/limit/epoch, and expire at the
original five-minute deadline. Publication, revoke and known invalidity invalidate
continuations. These are implementation bounds, not additional provider semantics.

## Verification scope

Tests cover migration admission, private allocation and separate publication,
reopening, lost publication acknowledgement, identity/scope/expiry refusal,
competing repair publication, terminal revoke, three-way revoke/repair/dispatch
races, bounded cursors, known missing material and full retirement retention.
The disposable native fixture additionally covers real custody/registry/CLI
composition, locked and unlocked restart, unknown write/delete acknowledgement,
delayed-writer refusal and exact deletion after restart. See the
[native test instructions](local-secret-service.md#disposable-qualification).

Business approval, audit, idempotency, OAuth, refresh, configuration upgrades and
the supervised provider journeys need their own implementation and evidence.
