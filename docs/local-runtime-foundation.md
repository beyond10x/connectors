# Local CLI foundation

The production `connectors` binary consumes the generated local CLI parser for
setup, passive configured inventory and connection list/describe/status/revoke.
This is incremental work under
`initiative:complete-local-connectors` and `story:persistent-gitlab-journey`.
It does not complete the persistent GitLab journey or the broader runtime plan.

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

`adapters describe --adapter ALIAS` reports only the configured entry, with no
runtime descriptor. `adapters status --adapter ALIAS` reports `owner_unavailable`;
it does not infer a stopped or ready process. Inventory contains at most 64
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
acquisition/custody publication, retirement and bounded read-use guards. Approval
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
`/run/user/<uid>/bus`: it verifies the socket's owner and kernel peer UID, then the
unique service owner's UID, and inspects the default non-session collection's lock
state. It never activates a service, opens a secret-transfer session, unlocks a
collection or reads a secret. A missing service/collection, inaccessible bus or
wrong owner reports `unavailable`; a locked collection reports `locked`.
This initial profile does not use a caller-supplied D-Bus address or a remote bus.

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

Protected connect/repair sources refuse with `cli_source` before reading any file,
stdin or terminal. No generated demonstration capture is used in production.
`connections list --adapter ALIAS` returns safe authoritative summaries and bounded
opaque cursors. `describe --connection REF`, `status --connection REF` or
`status --acquisition REF` observe the selected owner records. `revoke --connection
REF --expected-revision REV` commits terminal local revocation, even without a
keyring or provider. These subcommands also require `--adapter ALIAS`. They never
launch an adapter. Missing or unrecognized registry state returns
`metadata_unavailable`; missing references return `not_found`. Cached operation
inspection returns `description_unavailable`. Local operation invocation has no
admitted source/schema/dispatch binding yet. These refusals are unfinished
capabilities, not acceptance evidence for connection handling.

The original explicit `describe --endpoint --token-file`, `invoke --endpoint
--token-file --operation --input`, and `serve --config` retain their implementations.
They use the existing service binding described in [running services](running-services.md).

## Verification

The production-process tests cover fresh setup and metadata reuse across CLI
processes, exclusive initialization, configured inventory and passive status,
permission errors and safe source refusal. Host tests cover concurrent
initialization, stable authority identity, unsafe paths/private files/sidecars,
future schema refusal and owner/migration tampering. The registry and native
custody tests have separate scope described in their linked binding documents.
These tests do not claim provider authentication, process supervision or the GitLab
restart journey. Actual commands and results belong in the retained evidence
record for this delivery.
