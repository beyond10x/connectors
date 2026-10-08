# Local connection registry

The host's `local::registry` module binds the existing connection, acquisition,
credential evidence and custody contracts to one private SQLite authority. It
supports static-entry profiles. Provider work and keyring I/O happen outside
metadata transactions. Adapter libraries receive authenticated capabilities;
they never open this database or select arbitrary keyring items.

The production CLI uses this authority for connection list, describe, status,
terminal local revoke, protected connect/repair and explicit saved-material
revalidation. Its persistent owner binds the private adapter transport. Disposable
GitLab HTTPS and qualified Secret Service fixtures cover saved reads after restart
and real evidence expiry; dedicated GitLab sandbox acceptance remains open.

## Metadata and migration

Migration two retains the original local authority UUID and records its exact SQL
digest. Fresh setup installs all three migrations, including the later runtime
cache/suppression binding. Passive inspection accepts recognized
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
| Uses | Bounded read and explicit revalidation retention/admission guards; distinct private one-use handles, no business write approval or audit claim |
| Cursors | Opaque bounded list continuation tied to the exact selection and metadata epoch |

Before custody acknowledgement, a material row records only the acquisition's
allocated candidate. It is not a logical StoredCustodyVersion or dispatchable
credential. Neither its presence nor a successful read can fabricate a durable
write receipt. Credentials stay exclusively in Secret Service; SQLite contains
only nonsecret metadata. Public results exclude private scope, version, fence,
generation, capture and completion-owner coordinates.

Each operation holds an immediate transaction and bounded metadata handle.
A durable clock floor rejects wall-clock regression; it is kept in
`registry-clock.floor` beside the metadata lock and bounded by the recorded floor
([local ER metadata](local-er-metadata.md)). The production binding
samples that clock after acquiring the transaction; a caller timestamp sampled
before lock contention cannot establish regression. Request and capture deadlines
remain their original absolute values. It records time even when
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

## Configuration upgrades

A connection is bound to the configuration revision of its instance that it was
admitted under. When the configuration moves to a new revision (a rebuilt bundle,
a changed selection, a new provider executable or CA file), the connection reports
`pending` with `stale` set, and reads, repair and approvals refuse it.

Explicit revalidation upgrades it in place. It captures the connection's current
custody version, never a credential entry, runs the new provider's declared
validation against it, and publishes the connection under the configured binding
only when:

- the instance id, adapter id and provider authority are unchanged;
- the profile declaration is unchanged: the same id and declaration revision, so
  the same scheme, capability, purpose, subject, entry fields, minimum scopes and
  evidence lifetime. The registry keeps only the declaration's revision, which
  digests all of these, so any revision change refuses, a minimum-scope change
  included; and
- the validated external identity equals the connection's recorded identity.

Publication advances the public revision, because the binding changed
([CLI semantics](../contracts/cli/v1alpha1/semantics.md)), and revalidate answers
with the new one. It keeps the external identity, generation and custody
version, records new baseline evidence and a new private fence, and moves the
instance to the configured revision in the same metadata commit. Reads are then
admitted under the new binding and refused under the old one.

Any other binding difference refuses before provider work as `lifecycle_conflict`
with `next_action = create_connection`: neither revalidation nor repair can move
the connection, a new connection under a new instance id can. A different
validated identity refuses as `identity_mismatch`, also with `next_action =
create_connection`, because repair refuses the changed binding; outside an
upgrade it keeps `repair_connection`. A credential the new provider refuses
answers its usual code, again with `next_action = create_connection`. None of
these changes anything: the credential is not proved invalid under the
configuration it was admitted under, so it is not invalidated. A revoked connection is never upgraded,
including one revoked while the new provider validates.

Every other connection of the instance keeps its own revision until it is
revalidated itself. The metadata record holds a connection's revision only while
it differs from its instance's (`configuration_revision` on
`connectors.auth_bindings.Connection`); an upgrade records each such connection
with the revision it keeps. A revoked connection is terminal: it records no
revision of its own and follows its instance, so an upgrade never re-records it.
A new connection of the instance is admitted under
the instance's current revision, or under the configured revision when the two
differ. A connection published under the configured revision moves the instance
to it in the same metadata commit, as an upgrading revalidation does; a
connection begun under an older revision never moves the instance back. Changing
the provider authority or the profile declaration still needs a new instance id.

A refused upgrade (a credential the new provider refuses, or another identity)
is recorded on the connection, so reads, approval targets and launches answer
`next_action = create_connection` instead of sending the caller back to
revalidation; a repair or a successful upgrade clears the record. A stale
connection whose credential is known invalid or has expired is answered the same
way, since revalidation cannot move it.

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
races, bounded cursors, known missing material, full retirement retention and
configuration upgrades (`registry/upgrade_tests.rs`).
The disposable native fixture additionally covers real custody/registry/CLI
composition, locked and unlocked restart, unknown write/delete acknowledgement,
delayed-writer refusal and exact deletion after restart. See the
[native test instructions](local-secret-service.md#disposable-qualification).

Static-entry profiles include the OAuth-acquired non-rotating variant: the CLI
obtains `{client_id, client_secret, refresh_token}` by browser consent inside the
capture window and submits it as an ordinary entry, which the registry stores
like any other and never refreshes ([Google OAuth guide](catalog-google-oauth.md)).
Business approval, audit, idempotency, coordinator OAuth flows, host-owned
refresh, configuration upgrades across a changed provider authority or profile
declaration, and the supervised provider journeys need their own implementation
and evidence.
