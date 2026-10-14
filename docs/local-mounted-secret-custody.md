# Local mounted-secret custody binding

The `mounted_secret` binding reads credentials that a platform delivers as files in a
read-only mounted directory: a Kubernetes Secret volume, or a read-only bind mount. It is
the custody of a headless or container host, which has no session bus, no keyring daemon
and no unlocked `login` collection for the [Secret Service binding](local-secret-service.md).
It is Linux only. Its code is `crates/connectors-custody/src/mounted.rs`; the host's
`local::keyring::custody` coordinator selects it and keeps its receipts.
[Custody semantics §5.2](../contracts/auth/custody/v1alpha1/semantics.md#52-the-read-only-mounted-secret-binding)
owns its guarantees. [The container setup](local-container.md) shows how to run it.

## Selection

The binding is never chosen automatically. Only a `connectors-local/4` owner configuration
selects it, with a `[custody]` table:

```toml
format = "connectors-local/4"
owner_uid = 10001

[custody]
binding = "mounted_secret"
directory = "/run/connectors/custody"
```

- `binding` is required in the table and closed: `secret_service`, `macos_keychain`,
  `windows_credential_manager` or `mounted_secret`, and the build must support it.
- `directory` is required for `mounted_secret` and refused for every other binding. It is an
  absolute path with no symbolic link in any component, so name the mount point itself
  (`/run/…`), not a path through a link such as `/var/run`.
- `secret_service_socket` beside `binding = "mounted_secret"` refuses the configuration.
- A `[custody]` table in a `connectors-local/1` to `/3` file refuses the configuration.
- Without `[custody]`, custody is the platform binding exactly as before. A missing or
  unusable platform binding still answers `custody_unavailable`, even when a valid mounted
  directory exists.

No command-line flag, environment variable or request selects a binding, a directory or an
item. The directory is an infrastructure binding of the owner configuration, like
`secret_service_socket`, not a per-credential locator.

`setup check` reports `custody_backend = "mounted_secret"` and the prerequisite
`mounted_custody`, ready when the directory passes its checks on a read-only mount. The
Secret Service's `keyring` and `persistent_custody_qualification` prerequisites are not
listed under this binding.

## Items

An item is named by public, owner-configured coordinates only: `<adapter alias>/<profile>`,
such as `tavily/tavily.api-key`. Both are selectors (`[A-Za-z0-9._-]{1,128}`, never `.` or
`..`). The item holds the credential document the profile's protected file source accepts,
at most 64 KiB. One item backs one connection: a second `connections connect` for the same
alias and profile is `custody_read_only` while the first is not revoked, also when two
connects run at once: the rule is checked in the commit that publishes the connection.

Two layouts are admitted, and the binding tells them apart by the presence of `..data`:

- **Projected** (a Kubernetes Secret volume, the kubelet's atomic writer):
  `<directory>/..data -> ..<timestamp>`, with items at
  `<directory>/..<timestamp>/<alias>/<profile>`. The pod spec maps each Secret key to its
  path with `items: [{key: tavily-key, path: tavily/tavily.api-key}]`. The top-level links
  the kubelet adds are never opened.
- **Flat** (a read-only bind mount): `<directory>/<alias>/<profile>` as regular files.

## Checks

Every open is `openat` from an already admitted descriptor with `O_NOFOLLOW`, and every
check runs on the opened descriptor. The item itself is first opened with `O_PATH`, which
reads nothing, so whatever stands at its name (a device node or a FIFO included) is never
opened for reading before checks 5 and 6 admit it:

1. Every directory above `directory` is owned by root or the effective user and is not
   group- or other-writable.
2. `directory` is owned by root or the effective user and is on a read-only mount. A
   group- or other-writable bit is admitted only because that mount is read-only: a Secret
   volume's root can be a world-writable sticky directory, and nothing under it is trusted
   except through `..data`.
3. In the projected layout, `..data` is a symbolic link owned by root or the effective user
   whose target is one component that starts with `..`, contains no `/` and is not `..`. That
   sibling is a directory owned by root or the effective user and not group- or
   other-writable. No other symbolic link is resolved.
4. The `<alias>` directory is a real directory under the same rule.
5. The item is a regular file with one link and at most 64 KiB. It has no write bit for
   anyone and no other bit, and is owned by the effective user or root. Group read is
   admitted only on a root-owned file whose group the process holds as its effective or a
   supplementary group: how Kubernetes presents a `defaultMode: 0400` Secret to a non-root
   pod with `fsGroup`, as `0440 root:<fsGroup>`.
6. The item's filesystem is mounted read-only (`ST_RDONLY`): a writable filesystem is
   refused even when every mode bit is right.
7. Only then is that same inode opened for reading, through its `/proc/self/fd` entry
   rather than by name again, and refused unless it is the inode the checks ran on. It is
   read once and bounded before use. The binding therefore needs `/proc` mounted, as it is
   in every container.

A missing directory, an item that fails a check or a mount that is not read-only answers
`custody_unavailable`. Nothing retries against another binding. Failures carry neither the
bytes, the path nor the layout.

## Versions and rotation

Custody versions are immutable, but a mounted item can be replaced underneath Connectors.
The binding therefore pins every version it publishes: an HMAC-SHA-256 of the validated
bytes under a random 32-byte key, both kept owner-only in `custody-mounted/` in the state
directory. A pin is not the credential; a leaked metadata store alone does not allow
offline guessing of a weak password, but a copy of the whole state directory, key and pins
together, does (see the threat model).

A read re-reads the item and returns its bytes only while they match the pin. Otherwise
the read is refused with `custody_unavailable` and `next_action = revalidate_connection`,
and the changed bytes are never sent. Rotating a credential is therefore: update the
Secret, then run `connections revalidate`, which re-reads the item, validates it with the
provider and publishes it as a new version.

## Read-only behaviour

| Command or path | Under `mounted_secret` |
|---|---|
| `connections connect --adapter A --profile P` | Takes no protected source flag; a source flag is `invalid_input` at `arguments`. The owner reads item `A/P`, the provider validates those bytes, and the version is published from that read, not from a write. |
| `connections repair` | The same for the existing connection. |
| `connections revalidate` | Re-reads the item; changed bytes are validated and published as a new version, unchanged bytes refresh the evidence as before. |
| `connections revoke` | Metadata only, as on every binding. The mounted item is untouched; delete or rotate the Secret as well. |
| An OAuth-acquired (refresh-rotating) profile | `custody_read_only` at `custody`, before any provider exchange. |
| `approvals key-init`, `key-rotate`, `key-recover`, `key-retire` | `custody_read_only`: approval-signing seeds are not mounted. |
| Signing an approval | `custody_read_only` before any custody is opened; the platform keyring is never reached in its place. |
| A write, confirm or delete of a version | Not compiled: the binding implements only the read capability. |

`custody_read_only` (exit 1, `stage = custody`, `next_action = check_configuration`) is
distinct from `custody_unavailable`, so an operator can tell "this binding cannot write"
from "the store is down".

A retired version of this binding stays retired: no reclamation deletes custody material
on any binding today, and this one has nothing of its own to delete.

## Threat model

The adversary is anyone other than the configured owner user, plus a careless operator.

| Concern | `secret_service` | `mounted_secret` |
|---|---|---|
| Who supplies the bytes | The owner, through a protected entry source; Connectors writes them. | The platform that mounts the volume (the Kubernetes API server and kubelet, or the host making the bind mount); Connectors writes nothing. |
| At rest | An encrypted GNOME Keyring collection; the textual empty-password form is refused. | **Plain bytes** in a tmpfs (a Secret volume) or on the mounted filesystem. Confidentiality at rest is the platform's (etcd encryption, node access control). This is the material weakening, and why the binding is never automatic. |
| Who else on the host can read | Any process of the same user that can reach the unlocked collection; root. | Root on the node; whoever can read the Secret through the Kubernetes API or exec into the pod; with group read, every process in the pod holding the `fsGroup`. Not other users on the node. |
| Same-user process | Can call the Secret Service for the same items. | Can read the file directly. The same promise boundary. |
| Tampering or substitution | Item attributes are scoped and verified; the writer refuses an existing version. | Read-only mount, no write bits, root or owner ownership, and `..data` resolved only to a root- or owner-owned sibling. A substituted item is caught by the pin: until a revalidation re-pins it, reads of the old version are refused. |
| Writes to the item | Connectors writes the version it stores. | None, ever: the binding has no write, confirm or delete, and opens the item only to read it. |
| A copy of the state directory | Holds metadata only; the material is in the keyring. | The pin key and the pin records share the owner-only `custody-mounted/` subdirectory of the state directory. A copy of the whole state directory therefore holds both, which permits offline guessing of a weak mounted credential against its pins. Keep the state directory owner-only, as Connectors requires, and out of backups and images; a metadata store copied without `custody-mounted/` permits no such guessing. |
| Symbolic links | Every storage component is admitted without symbolic links. | `O_NOFOLLOW` everywhere; the single admitted indirection is `..data` to one `..`-prefixed sibling. |
| Rotation | Connectors writes a new immutable version. | The platform replaces the item; Connectors re-pins on `revalidate` and never follows silently. |
| Revocation | The version is retired; its material is retained for in-flight reads. | Connectors stops using the item. The operator must also delete or rotate the Secret. |
| Failure | A locked or absent store is `custody_unavailable`, with no prompt. | A missing or refused item or a writable mount is `custody_unavailable`; any write is `custody_read_only`. Never another binding. |
| Disclosure through Connectors | No values, locators or scope ids in output, logs or errors. | The same. The directory is in the owner configuration only; item paths never appear in results or diagnostics. |

How it differs from the refused plain-text fallback: that design would have used a file or
an environment variable when the keyring was unavailable, so a desktop credential would
silently land on disk and anyone able to plant a file could steer which credential is used.
This binding is used only when the owner configuration names it; Connectors never writes
plain text, so nothing received through `--credential-file`, a prompt or stdin is persisted
outside a protected store; an ordinary writable file is refused even when selected; and the
material's custodian is the platform, which already holds and delivers it.
