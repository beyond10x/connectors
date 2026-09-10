# Local CLI foundation

The production `connectors` binary consumes the generated local CLI parser for
setup, adapter and connection management, protected connect/repair, cached
operation discovery, saved-credential revalidation and supervised GitLab reads.
This is incremental work under
`initiative:complete-local-connectors` and `story:persistent-gitlab-journey`.
Dedicated sandbox acceptance and the broader runtime plan
remain open. See the [GitLab CLI guide](local-gitlab-cli.md) for the runnable surface.

## Try the implemented commands

Build with `cargo build --locked -p connectors`, then use the resulting binary.
The separately installed older `connectors` is not upgraded by this command.

```sh
target/debug/connectors --output json setup init
target/debug/connectors --output json setup check
target/debug/connectors --output json adapters list
```

An explicit absolute `--config` and `--state-dir` select another local placement.
Otherwise the binding uses the absolute XDG configuration/state directories, or
the corresponding directories beneath the user's home. There is no current
directory search or migration from an existing installed Connectors deployment.

Initialization creates an empty `connectors-local/1` TOML configuration bound to
the effective Linux UID, owner-only directories and a private SQLite database.
It refuses an existing configuration without replacing it. Failed or uncertain
filesystem publication is not reported as successful initialization. A database
left by interrupted initialization contains no credentials or usable connection.

Configured entries follow the
[reviewed configuration example](../contracts/cli/v1alpha1/fixtures/config-valid.toml).
Use actual installed executable paths and digests when adding entries. The
example's fictional executables are not runnable artifacts. Inspection never
installs or starts an adapter, and `setup check` reports unavailable or changed
artifacts as failed prerequisites. Credentials never belong in configuration or
executable arguments.

`adapters describe --adapter ALIAS` reports the configured entry and its cached
descriptor when available; cache facts are always stale. `adapters status --adapter
ALIAS` asks only an existing owner, or reports `owner_unavailable` when absent.
It never starts a process or infers readiness from a PID/cache. Inventory contains at most 64
entries. Its default page bound is 100; limits outside 1–500 are invalid. Until
the cursor owner is implemented, a limit smaller than the configured inventory
returns `capacity`, and a supplied cursor returns `stale_cursor`. It never silently
truncates a page or asserts exhaustion.

## Storage binding

The host owns `metadata.sqlite3`, its SQLite sidecars and `metadata.lock` in the
private state directory. The lock serializes the bounded setup/inspection handle's
whole lifetime, including schema installation and SQLite's last-close sidecar
retirement; OS process exit releases it. These handles must not be held across
provider work. SQLite owns the durable authority.
Its first migration records the local authority identity, UID and migration
digest. Migration two adds the [connection registry](local-connection-registry.md),
acquisition/custody publication, retirement and bounded read-use guards. Migration
three adds cached bootstraps and durable stop suppression. Approval
spending and business write dispatch remain unfinished.

The binding selects SQLite WAL, `synchronous=FULL`, foreign keys and in-memory
temporary storage, with two-second lock/busy bounds. Versioned migrations commit
atomically. A foreign database, unknown future schema version, changed migration
digest or different owner is refused; no automatic downgrade or reset occurs.
Inspection does not create missing state or migrate an existing database.
The Cargo lock selects rusqlite 0.40.2 and bundled SQLite 3.53.2. The runtime also
refuses an ambient SQLite override older than 3.51.3, which fixed the
[WAL-reset concurrency defect](https://www.sqlite.org/wal.html#walreset).

WAL with FULL synchronization asks SQLite to synchronize each commit. This relies
on the filesystem and storage honoring synchronization, and does not constitute
a transaction with a keyring or provider. The selected metadata groups and
separate acknowledgement boundaries remain those in
[design section 31](design.md#31-host-persistence-ownership-and-atomicity).
See SQLite's [WAL documentation](https://www.sqlite.org/wal.html) and
[synchronous setting](https://www.sqlite.org/pragma.html#pragma_synchronous).

Path admission walks directory descriptors with `O_NOFOLLOW`, rejecting symlinks,
foreign-owned or writable-by-other-user ancestors, broad private-file modes and
hard-linked private files. Existing directories are not chmodded. New files are
written and fsynced under a temporary name, published with `RENAME_NOREPLACE`,
then followed by a directory fsync. Tests run beneath a task-owned `TMPDIR`, not
a shared writable temporary ancestor. Same-UID malicious process/debugger access
is outside this boundary, as in the CLI contract.

## Credential and runtime boundary

`setup check` observes only an already-running Secret Service on
`/run/user/<uid>/bus`, or the private configuration's explicit local
`secret_service_socket`: it verifies the socket's owner and kernel peer UID, then the
unique service owner's UID, and inspects the default non-session collection's lock
state. It never activates a service, opens a secret-transfer session, unlocks a
collection or reads a secret. A missing service/collection, inaccessible bus or
wrong owner reports `unavailable`; a locked collection reports `locked`.
This initial profile does not use an environment-supplied D-Bus address or a remote bus.

An available collection is not qualified persistent custody. The additional
`persistent_custody_qualification` prerequisite checks the exact qualified daemon
artifact and admitted encrypted storage. It reports ready only for that binding;
an available service from another implementation remains unqualified. The
[Secret Service API](https://specifications.freedesktop.org/secret-service/latest-single/)
defines service operations and secret transport, but supplies no generic fsync
acknowledgement contract for every implementation. Backend qualification therefore
remains necessary under the selected
[CLI contract](../contracts/cli/v1alpha1/semantics.md).

The host now has an initial [Secret Service custody implementation](local-secret-service.md)
for a pinned GNOME Keyring binary: scoped immutable writes, exact reads and explicit
filesystem synchronization. Its disposable daemon tests exercise crash/restart and
failure handling. The registry binds publication and guarded retirement, with
separate native tests covering both and CLI acquisition status/revoke after restart.

Protected connect/repair sources require current configuration, profile permission,
target metadata, qualified custody and an admitted owner capture before reading a
file, deliberate stdin pipe or foreground controlling terminal. The generated
parser receives only a nonsecret marker; the paired handler consumes the clearing
buffer and private capture channel once. The owner validates the document through
the native adapter, then coordinates durable custody and metadata publication.
Unsafe sources return `protected_entry_unavailable`; interruption restores terminal
echo and exits 130. No generated demonstration capture is used in production.
`connections list --adapter ALIAS` returns safe authoritative summaries and bounded
opaque cursors. `describe --connection REF`, `status --connection REF` or
`status --acquisition REF` observe the selected owner records. `revoke --connection
REF --expected-revision REV` commits terminal local revocation, even without a
keyring or provider. These subcommands also require `--adapter ALIAS`. They never
launch an adapter. Missing or unrecognized registry state returns
`metadata_unavailable`; missing references return `not_found`. Operation discovery
reads the exact cached bootstrap, with `description_unavailable` before the first
admitted launch. Invocation requires the descriptor revision, schema identity,
current operation/profile permissions and a ready connection. The host checks the
original JSON text, pins exact material and commits the final read guard before
dispatch. Expired evidence refuses; implicit identity probes and credential repair
are not part of a business read.

The original explicit `describe --endpoint --token-file`, `invoke --endpoint
--token-file --operation --input`, and `serve --config` retain their implementations.
They use the existing service binding described in [running services](running-services.md).

## Verification

The host now supplies a [private adapter binding](../contracts/cli/v1alpha1/private-adapter.md)
with a sealed, digest-verified executable snapshot, inherited Unix channel, exact
child ownership and bounded bootstrap/request handling. GitLab's
[local executable mode](../adapters/gitlab/contracts/auth/v1alpha1/semantics.md#local-executable-configuration)
uses it for native validation and the three existing reads against an immutable
credential/target capability. Disposable TLS/process fixtures cover mismatch
refusal, cursor partitioning, stale stop, deadline loss and fresh child startup.
The [local owner](../contracts/cli/v1alpha1/owner.md) now binds that transport to
the production CLI. It coalesces startup under a retained lifetime lock, owns
children on persistent worker threads, and exposes bounded same-UID sockets.
Stop compares exact incarnation/configuration coordinates, commits suppression and
signals only the retained pidfd. Its control path remains responsive during a
provider read. Earlier queued jobs and pending captures cannot undo the stop fence.
Only a later admitted explicit connect/repair/revalidate/invoke resumes the entry.

The production-process tests cover fresh setup and metadata reuse across CLI
processes, exclusive initialization, configured inventory and passive status,
permission errors and safe source refusal. Host tests cover concurrent
initialization, stable authority identity, unsafe paths/private files/sidecars,
future schema refusal and owner/migration tampering. The registry and native
custody tests have separate scope described in their linked binding documents.
Additional production CLI fixtures combine a disposable GitLab HTTPS service and
qualified private keyring. They exercise protected entry, persisted reads, owner
and keyring restart, concurrent startup, failed repair, permission/schema refusal,
busy stop and terminal local revoke. PTY tests check hidden input and restored echo
after SIGINT. These fixtures do not satisfy dedicated provider sandbox acceptance
or qualify a different custody implementation. Actual commands and results belong
in the retained evidence record for this delivery.

Remaining management obligations include propagating one failed startup outcome
to all requests already waiting for that launch, renewing retained validation
evidence, reducing positive native credential-invalidity results into connection
readiness, and scheduling bounded expiry/retirement cleanup. The current read
binding and its fixture evidence do not close those contract cases.
