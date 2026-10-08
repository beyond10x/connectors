# Local metadata over Entity Runtime and Eventlog SQLite

Status: selected implementation design for the ESS evolution metadata adoption.

## Authority boundary

The local CLI and owner use one recorded Entity Runtime authority backed by the
shared Eventlog SQLite provider. The authority is identified by the existing
`local_authority.authority_id`; migration does not mint a replacement product
authority. Eventlog mints and retains its own stream identity. The immutable
binding is:

- logical scope `connectors.local-metadata/1`;
- tenant equal to the existing authority UUID;
- the provider-minted Eventlog stream identity.

The shared `entity-executor` evaluates every authored create and named command. The shared
`entity-eventlog` synchronous bridge owns the asynchronous executor and the
`eventlog-sqlite` connection. Atomic owner changes use one recorded batch. No
Connectors executor, event store, mutable snapshot file or second SQL writer is
introduced.

Complete provider captures use the explicit checked `u64::MAX` limits on all
four Eventlog capture resources. The predecessor has bounded owner-specific row
sets but no shared aggregate or lifetime-history cap; choosing a smaller shared
cap would make the entire authority unreadable while its owning port still
admits state. Provider accounting still rejects arithmetic overflow, and the
existing owner-specific admission limits remain unchanged.

The current SQL tables become a compatibility projection in a private in-memory
SQLite connection. Existing narrow owner code can continue to use its checked
queries, constraints and triggers while it is migrated: a successful SQL
transaction is only a proposed next projection. It is durable only after the
projection difference has been encoded as concrete Entity Runtime actions and
the recorded batch is acknowledged. A projection or append failure refuses the
caller. Reopening always rebuilds the projection from recorded subjects, so an
unacknowledged projection never authorizes dispatch, publication, reuse or
absence.

## Authoritative recorded types

The authored ESS entity identity and lifecycle are the runtime identity and
lifecycle. A SQLite table name, row image, index or migration marker never becomes
an Entity Runtime type. The pinned ESS Entity Runtime lowerer produces the checked
definitions used by the host; the host does not maintain a second hand-written
definition of those transitions.

| Compatibility projection | Authoritative ESS entity |
| --- | --- |
| `registry_instances` | `connectors.declarations.ServiceConfiguration` |
| `registry_profiles` | `connectors.auth_bindings.AuthProfile` |
| `registry_connections` | `connectors.auth_bindings.Connection` |
| `registry_acquisitions` | `connectors.auth_bindings.Acquisition` |
| `registry_generations` | `connectors.credentials.CredentialGeneration` |
| `registry_materials` | `connectors.auth_bindings.CustodyVersion` |
| `registry_uses` | `connectors.credential_evidence.ReadUse` |
| `registry_cursors` | `connectors.cli.ConnectionListCursor` |
| `local_runtime_instances` | `connectors.cli.LocalRuntimeRecord` |
| `registry_clock` / `mutation_clock` | `connectors.clock.LocalClockFloor` with distinct owner identities |
| `mutation_attempts` | `connectors.mutations.AttemptRecord` |
| `mutation_keys` | `connectors.idempotency.KeyReservation` |
| `execution_audits` | `connectors.execution_audit.AuditRecord` |
| `approval_redemptions` | `connectors.delegation.ApprovalRedemption` |
| `local_approval_issuers` | `connectors.approval_issuers.ApprovalIssuer` |
| `local_approval_keys` | `connectors.approval_issuers.ApprovalSigningKey` |
| `local_approval_policies` | `connectors.local_approval_policy.LocalApprovalPolicy` |

The serialized binding, profile declaration, request fingerprint, audit record,
approval subject, policy selection and operation list columns decode to their
existing closed ESS values before an action is proposed. Their JSON spelling is
only a compatibility encoding. The original primary key decodes to the existing
ESS identity; it is never replaced by a table-row identity.

A connection's binding takes its configuration revision from its instance. Only
while the two differ, after a [configuration
upgrade](local-connection-registry.md#configuration-upgrades) of another connection
of the instance, does the connection record its own as `configuration_revision`.

`registry_uses` retains a bounded captured read, its selected connection,
generation and custody version, the publication fence, the original expiry,
and whether dispatch opened or the use was released. The host's
`capture_read` checks current readiness and scope before creating that row;
`dispatch_read` rechecks generation, fence, material and readiness and opens
transport at most once. This retained use is not the authored transient
`DispatchAdmission`: the SQL row has no operation/request identity or
per-use permission evidence, and it cannot resume a provider operation after
restart. Import and projection preserve only the retained facts and state
bits. They never reconstruct an admission decision or invent an evidence
snapshot from the current Connection baseline, which may change or disappear
on repair or revoke.

Approval subjects retain their existing canonical proof bytes in the SQL
projection, including explicit `null` for absent route and authority coordinates.
The Entity Runtime Optional schema admits these as omitted members; the boundary
removes only those known nulls on capture and restores them on projection after
validating the original canonical subject. Legacy runtime bootstrap JSON with
alternate object-key order remains admitted by typed value; projection writes
the original production writer's struct order. Profile declarations and audit
records likewise project through their existing typed writers, retaining the
bytes used by repeated profile registration and exact audit acknowledgement.

The owner maps each admitted SQL transaction to the authored create or lifecycle
operations. Mutation preparation is one recorded batch containing
`PrepareAttempt` and, when keyed, `ReserveKey`. Opening dispatch executes only
`OpenDispatch`. Settlement executes the matching `RecordCompletion`,
`RecordRefusal` or `RecordUncertainty` together with `RetainResult` or
`QuarantineKey`. An unknown result has no settled timestamp or replay expiry.
Registry candidate allocation records the exact candidate reference while its
Acquisition remains Completing, atomically with the corresponding custody and
generation subjects; it does not grant another exchange. Entity Runtime
therefore refuses `Prepared -> Completed` and a
terminal attempt reopening dispatch even if a caller or projection is faulty.
Registry publication/revoke/final dispatch comparisons and their coupled entity
actions retain one batch. Approval spend, provider dispatch and custody remain
separately acknowledged boundaries.

The authored ESS system remains `ess/1`: no ESS envelope, identity spelling or
canonicalization rule changes. The new local component, concrete ReadUse
retention entity, and the added fields/commands are source content projected
into version 1 Entity Runtime definitions. This is an unpublished M7 draft
correction before adoption, so no existing ER reader or persisted level 9
installation has the superseded `registry_uses` mapping. Legacy levels 1–8
enter those definitions only as
explicit imported anchors at their retained lifecycle states; no old Eventlog
reader exists whose definition version could be silently reinterpreted.

`local_authority`, `schema_migrations`, `connectors_er_authority`, Eventlog's own
tables, SQLite indexes and triggers are provider bookkeeping. The two clock rows
are owner anti-regression observations. The retained ReadUse is a concrete owner
fact because conflating it with a transient admission both fabricated absent
evidence during legacy migration and made revoke unable to preserve a
historical use. No generic row type is introduced.

The `registry_clock` row is a projection, built again from Entity Runtime on
every open, so it is not where the registry's anti-regression floor is kept
between commands. That floor is the file `registry-clock.floor` beside
`metadata.lock` (`connectors.clock.RegistryClockFloorFile`, JSON
`{"last_seen_ms": <ms>}`). It is read and replaced only under the lifecycle lock,
by write-temporary, fsync and rename. A registry transaction refuses a sample
below the higher of the file and the row, then writes the file before its
Entity Runtime batch commits. The row, and so the recorded
`connectors.clock.LocalClockFloor` `s:registry`, advances only to a sample more
than 60,000 ms past its recorded value, so it lies at most that far below any
admitted sample. A missing or unreadable file counts as the row plus 60,000 ms:
samples are refused for at most that long after the file is lost, and never
below any sample admitted before. A file rolled back by hand admits at most
60,000 ms of regression, because the row still bounds it. An unchanged floor is
not recorded again. A batch prepared by a handle that did not hold the lock
throughout (an observation's unlocked replay) is refused as a stale revision
when another handle has changed a subject other than a runtime record since its
baseline; the check reads the event feed before anything is appended.

Credential bytes never enter these records. `material_version`, `custody_scope`
and other already admitted opaque references remain metadata; Secret Service
continues to own the corresponding material.

## Compatible migration from levels 1–8

Inspection of a level 1–8 database is read-only and neither creates Eventlog
tables nor advances a schema. The first admitted mutating open holds the existing
owner-only lifecycle lock for the entire transition and performs these phases:

1. Validate the exact application id, owner UID, schema level, migration digests,
   table shape and integrity. Unknown, future or altered sources refuse.
2. Capture every row from every table installed at that level in deterministic
   table/key order. Capture includes the authority UUID, schema level and exact
   retained audit ordering. Missing later tables mean “not installed”, not empty
   historical authority.
3. Create the Eventlog tables under the fixed `connectors_er` prefix, create and
   attach the shared recorded projector, and provision the immutable binding.
4. Decode each retained product row to its existing ESS identity, typed fields
   and current lifecycle state, then import that terminal instance through the
   shared legacy-anchor facade. A retry observes and verifies an earlier
   acknowledged anchor; conflicting bytes refuse. No source row is translated
   into a fabricated business event, committed receipt or cross-table global
   order. Provider bookkeeping and derived indexes are not imported as entities.
5. Take a provider-complete snapshot, rebuild the private SQL compatibility
   projection from it, and compare the reconstructed projection with the captured
   source byte-for-byte by typed value and deterministic order.
6. In one legacy SQLite transaction, append migration level 9 with the exact
   migration digest, immutable source level and digest, current projection level,
   and Eventlog binding, then set `user_version=9`. The level 1–8 tables remain unchanged as the
   recovery source. Older writers refuse level 9 rather than becoming a mixed
   generation.

Interruption before step 6 leaves the legacy source selected. Reopening repeats
the same capture and idempotent imports, verifies equivalence and switches once.
Interruption after step 6 reopens only the recorded authority. A level 9 marker
whose binding, source digest or recorded snapshot disagrees refuses; it is never
reset to an empty store. Fresh setup creates the level 1 authority envelope and
then follows the same transition with an empty admitted source, so fresh and
migrated installations enter the identical authority path.

No predecessor configuration or credentials are imported. Migration reads only
this repository's already selected local metadata file and carries only opaque
credential references.

Later owner admission advances only the marker's projection level after its
recorded batch is acknowledged; the source level and digest never change. A
restart rebuilds every already installed compatibility table at that projection
level. If interruption occurs before the marker advance, the next admitted open
repeats the schema installation and rebuild rather than interpreting the absent
projection rows as deleted product entities.

## Process and recovery behavior

The existing `metadata.lock` protects physical SQLite validation and last-close
sidecar retirement, and remains held across legacy migration and business
writes. On an established level 9 authority, passive inspection closes the
physical handle while holding that lock, then replays Entity Runtime outside
it. The two pure registry clock observations prepare a fresh replay the same
way, then reacquire that lock before sampling time and re-evaluating their
guards. Every write requires a handle that holds the lock and never released
it: a passive inspection and an unrelocked observation can neither persist nor
obtain the registry floor file. Before any batch, including one that changes
nothing, which then appends nothing, a writer reads the event feed past its
baseline. Since every writer holds the lock, nothing can append between that
read and its batch, so the read sees every change that precedes it. A change
to any subject refuses the batch as a stale revision, so a prepared
observation cannot return stale registry state after an intervening business
write, at any millisecond. A runtime-record write can land between an
observation's unlocked replay and its relock; the observation's feed check and
post-commit equivalence therefore admit changes to `LocalRuntimeRecord` rows,
which no registry observation reads or writes, exactly as the runtime-state
write excludes the unread registry clock. Only a proved uncommitted revision
conflict repeats the pure observation with a fresh replay and a finite bound.
Eventlog SQLite owns the runtime handles and serializes guarded appends. An
acknowledged batch is checked against its immutable receipt and a
fresh complete snapshot;
only differences with later recorded positions can supersede its projection.
The bridge has a
bounded queue and each metadata operation has a finite 30-second wait. A rejected
or queued call has no durable effect. A deadline, worker loss or provider uncertainty
after dispatch returns `OutcomeUnknown`; recovery reads the exact owner subject or
batch identity and never resends a provider effect solely because a process
restarted.

## Durable open checkpoints: which open verifies what

Entity Runtime 0.29.0 can persist an open checkpoint: the observation a
`ProviderTracked` handle last verified, with Eventlog's proof that only
acknowledged appends followed it. A store has them once they are enabled, which
installs Eventlog's continuity tables and triggers (`connectors_er_capture_*`).
`setup init` enables them on every database it creates; an existing database
gets them only through `setup checkpoints-enable --confirm one-way`. Enabling is
one-way for older releases: connectors 0.32.0 and earlier (Entity Runtime 0.28.0
and earlier) refuse to open the store afterwards, and no command removes them.
The host persists a handle's verified observation as the checkpoint when it
returns the handle to its process pool.

| Open | What the provider verifies | What the host verifies before the handle answers |
|---|---|---|
| Any fresh open in any process: a command, a second process, the owner when it starts, recovery after a refused handle | a `ProviderTracked` open: the checkpoint and the suffix after it, or the whole store when there is no usable checkpoint | the whole store: a complete snapshot, which reads and verifies every event, bound blob and index row; Entity Runtime 0.29.0 treats a complete read as a complete verification (its CHANGELOG, and `CapturePolicy::ProviderTracked` in `entity-eventlog` `adapter/tracked.rs`), so no open uses a separate `FullVerification` policy |
| A pooled handle reused in the process that verified it | — | the events appended since its head, by subject, or a complete snapshot when one cannot be attributed; the physical page scan (`quick_check`, foreign keys) of its fresh open is not repeated |
| A live `ProviderTracked` handle reading after another connection appended | the appended suffix, proven continuous by Eventlog's journal | — |

So the checkpoint shortens the provider's own open, and every fresh open still
verifies completely: a raw edit of `metadata.sqlite3` that bypasses SQLite while
no handle is open is refused by the first open after it, a command run directly
or the owner when it starts. The owner holds its handles for its whole run, so
it verifies the whole store once per start; no open, the owner's included,
answers from a checkpoint without that complete read. A command run while an
owner of its own build runs reads through it ([owner transport](../contracts/cli/v1alpha1/owner.md))
and does not open the store; without one it opens the store directly.
Page damage written to the file by something other than SQLite while a process
holds a verified handle for it is accepted by that process's later opens, which
do not repeat the page scan, until its next fresh open, which refuses it.

Owner start, measured on stores grown by read invokes (release build,
`owner_start_of_a_grown_store`, 2026-10-08): 763–834 ms at 600 events,
1,134–1,393 ms at 1,200 and 6,148–7,401 ms at 6,000, about 1.1 ms per event,
within the 10-second spawn deadline the CLI waits for a starting owner.

Due expiries of transient subjects (`ExpireReadUse`, `ExpireConnectionListCursor`)
are recorded in batches of at most 128 before the batch carrying the caller's own
change, because one batch's time grows with its members: on Entity Runtime 0.30.2
every due expiry in one batch took 14.4 s at 6,000 events, and batches of 128
took at most 1.5–2.5 s each (release build, `first_owner_open_of_a_grown_store`,
2026-10-08; on 0.29.0, 396 expiries in one batch took 101.9 s at 1,201 events,
and the bound was 32). Each expiry
batch's receipt is read back; the projection check runs once, after the
caller's batch. A refused or uncertain expiry batch leaves earlier ones
committed and the caller's change unapplied; the next write records the rest.

The local owner, adapter children and keyring remain separate processes with their
current socket, supervision, startup coalescing, bounded shutdown and explicit
stop suppression contracts. Only metadata persistence changes. Public results,
ordinary diagnostics and retained fixtures continue to exclude Secret Service
material, private locators and recorded provider payloads.

## Verification boundary

Acceptance uses disposable level 1 through level 8 databases, a nonempty level 8
fixture, interruption at each transition phase, fresh setup, post-migration writes,
close/reopen, recorded projection rebuild and tamper/future-schema refusal. The
actual CLI/owner cases then prove repair/revoke/publication, mutation uncertainty,
audit, approval spending and stop suppression through the selected authority.
Qualified Secret Service cases use an owned bus and keyring. The exact source,
generated ESS roots, both lockfiles and the full locked MSRV gate are one final
vector; source-only success is not completion.
