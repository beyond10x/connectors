# Local Secret Service custody binding

The host's `local::keyring::custody` module implements immutable scoped writes and
exact-version reads against one qualified Linux Secret Service implementation.
It is infrastructure for the persistent GitLab journey. Connection publication,
protected CLI entry and guarded retirement are still unfinished. The CLI's
`persistent_custody_qualification` prerequisite remains failed; this module does
not yet establish a usable saved connection.

## Selected implementation

The initial binding accepts the Arch Linux `gnome-keyring 1:50.0-1` daemon with
SHA-256 `b9a71f6b4c4bfaf1759a99036b7b3ab6cf98b2b479c0c2a24f02f5f2ca53958b`.
Its upstream source is GNOME Keyring 50.0, commit
`2ff8b070763ae025b90916a7b98643865819b451`. Different implementations or daemon
binaries require their own qualification; a version string is insufficient.
The initial filesystem admission is the local ext family (`EXT4_SUPER_MAGIC`),
tested on ext4. In-memory and other unqualified filesystem types are refused even
if their fsync call succeeds. This is a deliberately narrow initial binding.

Admission authenticates the local bus socket and unique Secret Service owner to
the effective UID. Production connects only to `/run/user/<uid>/bus`, never an
environment-selected or remote bus. It pins the unique bus name, opens a pidfd for
that owner's PID, verifies the executable digest, and requires the same mount and
user namespaces and filesystem root. A dead owner cannot be replaced underneath
an existing capability. There is no service activation, unlock, or prompt.

Only an already-unlocked default collection resolving to the persistent `login`
collection is selected. The binding derives its storage directory from the
verified daemon's bounded environment, using the audited HOME/XDG and legacy
directory rules. It does not assume the client's environment matches the daemon's.
The temporary environment buffer is zeroized without diagnostics. Duplicate or
relative selected paths, library injection variables and the debug-only storage
override are refused. Every selected storage path component is then admitted
without symlinks; the collection file must be owner-only, regular and singly linked.

The binding verifies the
[encrypted binary file header](https://github.com/GNOME/gnome-keyring/blob/2ff8b070763ae025b90916a7b98643865819b451/pkcs11/secret-store/gkm-secret-binary.c)
before transferring any
credential and after writing. GNOME Keyring can store a collection with an empty
master password in textual form; that form is refused. No direct file read returns
credential material: only the nonsecret format header is inspected. Credential
values pass through Secret Service.

The implementation details come from the pinned upstream
[daemon initialization](https://github.com/GNOME/gnome-keyring/blob/2ff8b070763ae025b90916a7b98643865819b451/daemon/gkd-pkcs11.c),
[directory selection](https://github.com/GNOME/gnome-keyring/blob/2ff8b070763ae025b90916a7b98643865819b451/pkcs11/gkm/gkm-util.c),
[filename mapping](https://github.com/GNOME/gnome-keyring/blob/2ff8b070763ae025b90916a7b98643865819b451/pkcs11/secret-store/gkm-secret-module.c)
and [collection persistence](https://github.com/GNOME/gnome-keyring/blob/2ff8b070763ae025b90916a7b98643865819b451/pkcs11/secret-store/gkm-secret-collection.c).

## Acknowledgement and scoped identity

The existing [custody contract](../contracts/auth/custody/v1alpha1/semantics.md)
owns these semantics. The existing ESS `CustodyVersion` scope and store-version
tuple maps injectively to private lookup attributes: a format marker, local
authority UUID, allocated scope UUID and version UUID, plus the fixed native
`xdg:schema` marker `org.beyond10x.Connectors.Credential`. Setting that marker
explicitly prevents GNOME's binary-file compatibility reader from adding a
different default schema on reload. The host allocates and
records the tuple before a write. These are opaque store coordinates, not new
provider identities, public selectors or dispatch grants. Values and references
have no Debug or Serialize implementation. Secret byte buffers clear on drop.

`write_new` refuses an existing exact version and always calls `CreateItem` with
replacement disabled. A process-local mutex and directory flock serialize the
lookup/create gap among cooperating Connectors instances. There is no listing of
credential values; searches are confined to the admitted collection and exact
scope/version attributes. Duplicate matches or changed attributes refuse. A
capability for one scope cannot read or write another scope's version.

After the native write acknowledgement, the binding reads back that exact item
and compares the opaque bytes, then opens the current encrypted file and fsyncs
it, its directory and the admitted ancestor directories. The reopened file matters:
GNOME uses an atomic rename to replace the previous inode. Its inspected
[transaction implementation](https://github.com/GNOME/gnome-keyring/blob/2ff8b070763ae025b90916a7b98643865819b451/pkcs11/gkm/gkm-transaction.c)
syncs the temporary file but does not provide the directory barrier in that write
path. The Connectors binding supplies it before acknowledging custody. This
assumes the filesystem and storage honor fsync; process restart tests do not
simulate a power failure or prove hardware behavior.

Any failure after `CreateItem` may have been sent returns `OutcomeUnknown`.
Neither a positive method response nor a readable candidate alone becomes a
durable acknowledgement. No automatic write retry or overwrite follows. A
candidate is never connection authority until the separate metadata coordinator
has acknowledged its publication. Failed repair must preserve a still-valid
published version.

The Secret Service plain transfer session is restricted to the authenticated local
bus. OS, D-Bus and service-owned buffers are outside the application's clearing
guarantee. Values remain opaque byte strings of 1–65536 bytes. This GNOME version
returns `text/plain` even for binary secrets, as its
[secret response code](https://github.com/GNOME/gnome-keyring/blob/2ff8b070763ae025b90916a7b98643865819b451/daemon/dbus/gkd-secret-secret.c)
shows. That label is accepted only under this qualified implementation; it never
selects a decoder, changes bytes, or grants authority. Each D-Bus method has a
two-second timeout, and backend errors expose only closed safe categories.

Physical item deletion exists only in the isolated qualification tests. Production
must first bind the metadata retirement fence, 24-hour retention, and no-valid-use
or recovery checks specified by custody section 4.1. Tests of deletion persistence
are not evidence that those coordinator guards have been implemented.

## Disposable qualification

The host test `disposable_secret_service_restart_and_failures` creates its own
private bus, encrypted keyring, fictional password and daemon processes beneath
the task's `TMPDIR`. It never connects to the desktop keyring. Every owned child
is killed and reaped on success or failure; temporary fixture files are removed
after the children exit. The test is opt-in because it needs the exact qualified
OS binary. Run it explicitly in addition to the repository gate:

```sh
CARGO_BUILD_JOBS=2 cargo test --locked --offline -p connectors-host --lib \
  local::keyring -- --include-ignored --nocapture
```

Set `TMPDIR` and `CARGO_TARGET_DIR` to the task-owned paths described in the
[development guide](development.md). Qualification results and exact inputs belong
in dated evidence receipts; an ignored test is not a passing runtime check.
