---
format: aep.planning-md/3
id: story:mounted-secret-custody
kind: story
status: active
title: Read-only mounted-secret custody, explicitly selected, for headless and container hosts
relations:
- serves: vision:independent-contract-adapters
- depends_on: story:platform-credential-custody
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: inferred
  path: apps/connectors-cli-contract
- confidence: cited
  path: apps/connectors/spec/cli.yaml
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: cited
  path: apps/connectors/src/local/connections.rs
- confidence: cited
  path: apps/connectors/tests/container_env_i.rs
- confidence: cited
  path: apps/connectors/tests/mounted_custody.rs
- confidence: cited
  path: contracts/auth/custody/v1alpha1/semantics.md
- confidence: cited
  path: contracts/cli/v1alpha1/fixtures
- confidence: inferred
  path: contracts/cli/v1alpha1/fixtures/values.json
- confidence: cited
  path: contracts/cli/v1alpha1/scenarios.md
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-build/src/ignored.rs
- confidence: cited
  path: crates/connectors-custody/Cargo.toml
- confidence: cited
  path: crates/connectors-custody/src/contract.rs
- confidence: inferred
  path: crates/connectors-custody/src/keychain.rs
- confidence: cited
  path: crates/connectors-custody/src/lib.rs
- confidence: inferred
  path: crates/connectors-custody/src/memory.rs
- confidence: cited
  path: crates/connectors-custody/src/mounted.rs
- confidence: inferred
  path: crates/connectors-custody/src/secret_service/mod.rs
- confidence: inferred
  path: crates/connectors-custody/src/tests.rs
- confidence: cited
  path: crates/connectors-docs/src/status.rs
- confidence: cited
  path: crates/connectors-host/src/local/approval_keys.rs
- confidence: cited
  path: crates/connectors-host/src/local/config.rs
- confidence: cited
  path: crates/connectors-host/src/local/filesystem.rs
- confidence: cited
  path: crates/connectors-host/src/local/keyring.rs
- confidence: cited
  path: crates/connectors-host/src/local/keyring/custody.rs
- confidence: inferred
  path: crates/connectors-host/src/local/metadata/er.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/supervisor.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/lifecycle.rs
- confidence: cited
  path: crates/connectors-spec/toolchain.json
- confidence: cited
  path: docs/local-container.md
- confidence: cited
  path: docs/local-mounted-secret-custody.md
- confidence: cited
  path: docs/local-secret-service.md
- confidence: cited
  path: ess/components.yaml
- confidence: cited
  path: ess/domains/auth_bindings.yaml
- confidence: cited
  path: ess/domains/cli.yaml
- confidence: inferred
  path: website/data/status.json
- confidence: inferred
  path: website/docs/concepts/local-runtime.md
- confidence: inferred
  path: website/docs/guides/set-up-the-local-cli.md
- confidence: inferred
  path: website/docs/reference
- confidence: inferred
  path: website/docs/status.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T20:03:47Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-14T02:55:05Z", actor: "human:timo", revision: 6}
---
## Problem

A consumer runs `connectors` in a container under a Kubernetes CronJob and reaches its source and
PostgreSQL credentials through `connectors connections`. Custody, the only place a saved
connection's credential lives, binds on Linux to one qualified Secret Service daemon on the
user's session bus. A container has no session bus, no keyring daemon and no unlocked `login`
collection, so it has no usable custody. It also cannot satisfy the configuration stage
without a home directory laid out the way a desktop session lays one out. Under
`env -i HOME=<empty> PATH=/usr/bin:/bin`, both `connections list --adapter tavily` and
`setup check` exit 2 with `invalid_configuration` at stage `configuration`, before custody is
reached. The custody refusal is therefore inferred, not observed. A headless host has no
supported way to hold a credential today.

## Today

- **One binding per platform, fixed at build time, no fallback.**
  `contracts/auth/custody/v1alpha1/semantics.md:132-136`: one binding per platform, reported
  by `setup check` as `custody_backend`. "No binding falls back to another, to a file or to an
  environment variable". Without a usable binding, custody answers `Unavailable` and the CLI
  refuses with `custody_unavailable`. Code: `crates/connectors-custody/src/lib.rs:70-78`
  (`PLATFORM`), `:315-332` (`open`), `:356-364` (`available`).
- **The Linux binding is narrow by design.** `docs/local-secret-service.md:20-26` admits one
  daemon, identified by its digest, on ext4. `:29-41` requires the effective UID's bus at
  `/run/user/<uid>/bus` or an owner-configured `secret_service_socket`, and allows no
  activation, unlock or prompt. `:43-46` selects only the already-unlocked `login`
  collection.
- **Plain text is refused in three places.**
  - `contracts/cli/v1alpha1/semantics.md:468-469`: "plaintext TOML, environment variables or
    ordinary local files are never fallback custody".
  - `ess/domains/cli.yaml:302-308`: `connectors.cli.CustodyBackend` is
    `[secret_service, macos_keychain, windows_credential_manager]`, with "There is no
    plain-text variant". The same is said at `crates/connectors-custody/src/lib.rs:41-48`.
  - `.engineering/planning/story/platform-credential-custody.md:19-22,79` (implemented): "no
    plain-text fallback" and "No path stores credential material in a file or environment
    variable on any platform".
- **The binding selection in configuration is Linux-socket-only.** `ess/domains/cli.yaml:142-147`
  (`connectors.cli.LocalCustodyBinding { secret_service_socket }`).
  `crates/connectors-host/src/local/config.rs:12-28` (`Config`, `deny_unknown_fields`
  envelope at `:33-45`). `contracts/cli/v1alpha1/semantics.md:396-403`. Configuration
  "contains no credential bytes, keyring locators or actionable acquisition continuations"
  (`contracts/cli/v1alpha1/semantics.md:310-311`).
- **Writes go through two call sites.**
  - Connection publication: `crates/connectors-host/src/local/registry/lifecycle.rs:183-209`
    (`store_and_publish` → `write_new_guarded`), reached from
    `crates/connectors-host/src/local/owner/transport.rs:1703`.
  - Approval signing keys: `crates/connectors-host/src/local/approval_keys.rs:199`.
  - The backend trait requires `write_new`, `confirm` and `delete`
    (`crates/connectors-custody/src/lib.rs:270-312`).
- **CustodyVersion lifecycle ends in deletion.** `ess/domains/auth_bindings.yaml:218-251`:
  Candidate → Stored → Retiring → Deleted. The opaque `store_version: String` is the
  binding-private coordinate. Store layout is "UNMAPPED, binding-private"
  (`contracts/auth/custody/v1alpha1/semantics.md:188`).
- **A read-only file binding already exists, but not for local custody.** It is the service
  host's `CredentialRef::File` / `DirectorySecrets` (`crates/connectors-host/src/credentials.rs:17-28,99-149`).
  It opens with `O_NOFOLLOW` and refuses a file that is not regular, is group- or
  other-accessible, or belongs to another UID (`:101-123`). Custody semantics keep it as the
  `read_only` profile (`contracts/auth/custody/v1alpha1/semantics.md:24`, `:167`). It is not
  a backend of the local CLI's versioned custody.
- **Why the `env -i` run stops at configuration.**
  - `Paths::resolve` (`crates/connectors-host/src/local/config.rs:202-228`) takes
    `--config`/`--state-dir`, otherwise `$XDG_CONFIG_HOME`/`$XDG_STATE_HOME`, otherwise
    `$HOME/.config/connectors/config.toml` and `$HOME/.local/state/connectors`. With no
    `HOME` and no flags this is `invalid_configuration` (`:209`).
  - `Config::read` (`:240-251`) opens the file through `private_file`
    (`crates/connectors-host/src/local/filesystem.rs:196-207`). That walks every component
    with `O_NOFOLLOW` (`:40-52,95-127`), admits a directory only if it is owned by the
    effective UID or root and is not group- or other-writable (`:61-71`), and admits the file
    only if it is regular, owned by the effective UID, has no group or other bits and has one
    link (`:188-194`).
  - `owner_uid` must equal the effective UID (`config.rs:267`).
  - An empty `HOME` has no `config.toml`, so the run refuses there. It never reaches custody:
    `setup check` reads the configuration before inspecting the keyring
    (`apps/connectors/src/local.rs:325-361`).
  - The spawned local owner gets its paths as arguments, with a cleared environment
    (`crates/connectors-host/src/local/owner/transport.rs:886-891`). After path resolution,
    nothing needs an environment variable.
- **A Kubernetes Secret volume fails today's file rules on several counts.** This is inferred
  from documented Kubernetes behaviour and must be confirmed by the acceptance run below.
  - Each key is a symlink `key -> ..data/key`, and `..data` is a symlink to a timestamped
    directory, so `O_NOFOLLOW` refuses both the key and `..data`. The protected
    `--credential-file` source refuses them for the same reason
    (`contracts/cli/v1alpha1/semantics.md:486-490`).
  - Files are root-owned, so they fail the effective-UID owner check for a non-root
    container.
  - With `fsGroup` set, the kubelet adds group read: a `defaultMode: 0400` file presents as
    `0440 root:<fsGroup>`, which fails the no-group-bits check.
  - The volume root is a world-writable sticky tmpfs directory, so it fails the ancestor
    check.
  - A ConfigMap mounted as `config.toml` fails in the same ways.

## Outcome

An operator can choose, in the owner configuration, a second Linux custody binding,
`mounted_secret`. It reads credentials from a read-only mounted directory, such as a Kubernetes
Secret volume or a read-only bind mount. It writes, rotates and deletes nothing. It is never
chosen automatically: with no explicit selection, custody is the platform binding exactly as
today, and a missing or unusable platform binding still refuses with `custody_unavailable`. A
container that sets only the documented flags or variables can run `setup check`,
`connections connect`, `connections list` and a credentialed `operations invoke` or
`connections launch` under `env -i`. A documented container setup with a CronJob example shows
how. `setup check` reports `custody_backend = mounted_secret`. Every write path either does
nothing to the mounted material or refuses by name.

## Design

### Selection: explicit, owner-configured, never a fallback

- A new configuration format, `connectors-local/4`, is `/3` plus an optional `[custody]` table:

  ```toml
  format = "connectors-local/4"
  owner_uid = 10001
  [custody]
  binding = "mounted_secret"
  directory = "/var/run/connectors/custody"
  ```

  `binding` is required in the table and closed (`secret_service`, `macos_keychain`,
  `windows_credential_manager`, `mounted_secret`). It must be one the build supports;
  `mounted_secret` is Linux-only in this story. `directory` is required for
  `mounted_secret` and refused for every other binding. `secret_service_socket` and
  `binding = "mounted_secret"` together refuse the configuration. A `[custody]` table in a
  `/1`–`/3` file refuses the whole configuration, as `[consumers]` does in `/1`–`/2`. The
  table is absent from configurations `setup init` writes, and `setup init` keeps writing
  `/2`.
- Absence of `[custody]` means the platform binding, unchanged. There is no CLI flag and no
  environment variable that selects a binding. A request can never select a binding, a
  directory or an item. That keeps custody §2's "no request can select a credential source,
  store, … item, or fallback" (`contracts/auth/custody/v1alpha1/semantics.md:22`). The
  directory is an infrastructure binding of the owner configuration, like
  `secret_service_socket`, not a per-credential locator.
- A selected binding that is unusable answers `Unavailable` → `custody_unavailable`. A
  missing directory, an item that fails a check, or a mount that is not read-only all answer
  this way. Nothing ever retries against another binding.

### Directory layout and per-credential naming

- **Item naming.** An item is named by public, owner-configured coordinates only:
  `<adapter alias>/<profile>` for a connection credential, for example
  `tavily/tavily.api-key` or `warehouse/postgres.password`. Both segments are already
  `valid_id` selectors (`[A-Za-z0-9._-]{1,128}`, `crates/connectors-core/src/lib.rs:360-366`)
  and never `.` or `..`. The item holds exactly the JSON credential document that the profile's
  protected file source accepts (`contracts/cli/v1alpha1/semantics.md:499-500`), at most
  64 KiB.
- **One connection per profile.** There is one connection per `(alias, profile)` under this
  binding. A second `connections connect` for the same pair refuses `custody_read_only`.
- **Two admitted layouts.** The binding detects which one is present and never mixes them.
  - *Projected* (Kubernetes Secret, ConfigMap-style atomic writer):
    `<directory>/..data -> ..<timestamp>`, with items at
    `<directory>/..<timestamp>/<alias>/<profile>`. The Secret keys stay flat (`tavily-key`).
    The pod spec maps each key to its path with `items: [{key: tavily-key, path:
    tavily/tavily.api-key}]`. The user-visible top-level key symlinks are never opened.
  - *Flat* (read-only bind mount): `<directory>/<alias>/<profile>` as regular files, with no
    `..data` entry.

### Permission and integrity checks

All opens are `openat` from an already-admitted descriptor, with
`O_NOFOLLOW|O_CLOEXEC|O_NONBLOCK`. Each check below runs on the opened descriptor, not the path.

1. **Path to `directory`.** The existing ancestor walk is kept (`filesystem.rs:95-127`) for
   every component above `directory`.
2. **`directory` itself.** It is owned by root or the effective UID. It is either not group- or
   other-writable, or it is the root of a read-only mount (check 6). In the projected layout,
   a world-writable sticky root is admitted only because nothing under it is trusted except
   through `..data`.
3. **`..data` (projected only).** `fstatat(AT_SYMLINK_NOFOLLOW)` shows a symlink owned by root
   or the effective UID. `readlinkat` returns a single component that starts with `..`, has no
   `/` and is not `..` itself. That sibling is opened with `O_NOFOLLOW|O_DIRECTORY` and must
   be owned by root or the effective UID and not group- or other-writable. No other symlink is
   resolved, and no target outside `directory` is ever reached.
4. **`<alias>` directory.** It is a real directory, not a symlink, and passes the same
   owner and writability rule.
5. **Item.** It is a regular file (`S_ISREG`) with `st_nlink == 1` and at most 64 KiB.
   - It has no write bit for anyone (`mode & 0o222 == 0`), so it is read-only even for its
     owner.
   - It has no other bits (`mode & 0o007 == 0`).
   - Its owner is the effective UID, or root.
   - Group read is admitted only if `st_gid` is the process's effective GID or one of its
     supplementary GIDs (a Kubernetes `fsGroup`). This is the relaxation operator question Q1
     decides. Without it, the rule is owner-only: `0400`, owned by the effective UID.
6. **Read-only mount.** `fstatvfs` on the item descriptor has `ST_RDONLY` set. A writable
   filesystem is refused even when every mode bit is right. This is what separates a mounted
   secret from "a file in a directory".
7. **Read.** The item is read once from the checked descriptor and bounded before parsing; it
   is never reopened. Bytes are held in `Zeroizing` buffers, and diagnostics carry neither the
   path, the layout nor the bytes.

### Version identity

- **Rule.** Custody versions are immutable (`contracts/auth/custody/v1alpha1/semantics.md:58`),
  but a mounted item can be replaced underneath connectors. So the binding never claims a
  version it cannot pin.
- **At connect, repair or revalidate.** The host allocates the `CustodyVersion` as today. Its
  binding-private `store_version` records the item name and a fingerprint of the bytes that
  were validated. The fingerprint is defined by Q3.
- **On read.** The binding re-reads the item and returns its bytes only if the fingerprint
  matches. Otherwise it answers `Missing`, the connection reports `revalidate_connection`, and
  the next `connections revalidate` (or `repair`) re-reads the item, re-validates it with the
  provider and publishes a new `CustodyVersion`.
- **Rotation.** Rotating a credential therefore means updating the Secret, then running
  `revalidate`. A rotated item is never silently used under evidence gathered for the old
  bytes.

### Read-only behaviour of every write path

The brief's `add/revalidate/remove` are, in this CLI, `connect`, `repair`, `revalidate` and
`revoke` (`contracts/cli/v1alpha1/semantics.md:85-93`).

| Command / path | Under `mounted_secret` |
|---|---|
| `connections connect --adapter A --profile P` | Takes **no** protected source flag; a source flag refuses `invalid_input` at `arguments`. Reads item `A/P` under the checks above and validates it with the adapter. Records and publishes a `CustodyVersion` *without* a write (`acknowledge` from the read, not from `write_new`). A missing or refused item is `custody_unavailable` at stage `custody`. |
| `connections repair` | Same as connect for the existing connection: re-reads the item; never writes. |
| `connections revalidate` | Re-reads the item. If the fingerprint changed, it publishes a new `CustodyVersion`; otherwise it refreshes evidence as today. |
| `connections revoke` | Metadata only, as today. The mounted item is not touched. Retirement ends the version in a new terminal state, `Released` ("not owned; nothing deleted"), never `Deleted`. |
| Reclamation (`delete`) of superseded versions | The binding declares that it provides no delete (custody §4, "a binding declares the guarantees it does not provide", `semantics.md:67`). Retiring → Released. |
| Refresh-rotating profiles (OAuth refresh that writes a new version) | Refused at connect with `custody_read_only` at stage `custody`, before any provider exchange. |
| `approvals` key creation / rotation (`approval_keys.rs:199`) | Refused with `custody_read_only`. Mounted approval-signing seeds are not in scope. |
| `Backend::write_new` / `confirm` / `delete` | The trait splits into a read capability and a write capability. `mounted_secret` implements only the read capability, so the compiler, not a runtime branch, rules out a write. |

`custody_read_only` is a new closed failure code (exit 1, `next_action = check_configuration`).
It is distinct from `custody_unavailable`, so an operator can tell "this binding cannot write"
from "the store is down".

### Kubernetes Secret volume specifics (documented container setup)

The setup page is `docs/local-container.md`, linked from the website guide. Its example
(UIDs are illustrative):

```yaml
securityContext: {runAsUser: 10001, runAsGroup: 10001, runAsNonRoot: true, fsGroup: 10001}
containers:
  - name: job
    image: <image with /srv/connectors owned 10001:10001, mode 0700>
    args: ["connectors", "--config", "/srv/connectors/config.toml",
           "--state-dir", "/srv/connectors/state", "connections", "list", "--adapter", "tavily"]
    securityContext: {readOnlyRootFilesystem: false, allowPrivilegeEscalation: false}
    volumeMounts: [{name: custody, mountPath: /var/run/connectors/custody, readOnly: true}]
volumes:
  - name: custody
    secret:
      secretName: connectors-custody
      defaultMode: 0400
      items: [{key: tavily-key, path: tavily/tavily.api-key}]
```

- **Ownership.** Secret files are root-owned. A non-root container reads them only through
  group read: `fsGroup` makes the group the pod's group, and the kubelet adds group read, so
  `defaultMode: 0400` presents as `0440 root:10001`. Without `fsGroup`, a `0400` root file is
  unreadable by UID 10001 and `setup check` reports the custody prerequisite failed. Running as
  UID 0 to make `0400` root-owned files readable is refused by the documented setup
  (`runAsNonRoot`); the binding itself does not forbid UID 0. The admitted outcome depends on Q1.
- **Layout.** The volume is read-only (the kubelet mounts Secret volumes read-only), so check 6
  holds. Kubelet's atomic update replaces `..data`. A CronJob pod reads the volume as it was
  when the pod started, and the next pod sees the update.
- **Configuration.** A ConfigMap cannot hold `config.toml`: it is root-owned, symlinked and
  group-readable. The configuration is baked into the image, owned by the run UID, mode
  `0600`, in a `0700` directory whose ancestors are root-owned and not group- or
  other-writable, with `owner_uid` equal to the run UID.
- **State.** `--state-dir` names a directory the run UID owns in the container's writable
  layer. `setup init` creates it `0700` on first run. An `emptyDir` or a PVC root with `fsGroup`
  is group-writable and is refused by today's ancestor check. Q2 decides whether that changes.
  With ephemeral state, each run does `setup init`, `connections connect` (one provider
  validation call), then the work; the connection reference comes from the connect result's
  JSON.

### The `env -i` configuration failure

- **What the configuration stage needs.** It needs a resolvable absolute config path and state
  path (`config.rs:202-228`), an admitted owner-only config file (`filesystem.rs:188-207`) and
  `owner_uid` equal to the effective UID. It needs no other variable.
- **What a container must set.** Either both `--config <abs>` and `--state-dir <abs>` (no
  variable needed), or `HOME` pointing at a directory that meets the ancestor rule and holds
  `.config/connectors/config.toml`. `XDG_CONFIG_HOME`/`XDG_STATE_HOME` are optional
  alternatives. `PATH` is not consulted: executables are absolute and digest-pinned
  (`contracts/cli/v1alpha1/semantics.md:315-319`).
- **Refusal detail.** The story keeps the refusal code. The configuration-stage refusal also
  names which rule failed (`config_path_unresolved`, `config_missing`, `config_not_private`,
  `owner_uid_mismatch`). It does this through the existing `Refusal` carrier
  (`config.rs:68-89`), with no path or OS text. The consumer's report shows that an
  undifferentiated `invalid_configuration` cost a diagnosis round.

## Threat model

The two bindings are shown side by side. The adversary is anyone other than the configured
owner UID, plus a careless operator.

| Concern | `secret_service` (today) | `mounted_secret` (this story) |
|---|---|---|
| Who supplies the bytes | The owner, through a protected entry source; connectors writes them. | The platform that mounts the volume (a Kubernetes API server and kubelet, or the host doing the bind mount); connectors never writes. |
| At rest | Encrypted GNOME Keyring collection; the textual empty-password form is refused (`docs/local-secret-service.md:53-59`). | **Plain bytes** in a tmpfs (Kubernetes Secret volume) or on the mounted filesystem. Confidentiality at rest is the platform's (etcd encryption, node access controls), not connectors'. This is the material weakening, and why the binding is never automatic. |
| Who else on the host can read | Any same-UID process that can talk to the unlocked collection over the session bus; root. | Root on the node; anyone who can read the Secret through the Kubernetes API (RBAC) or exec into the pod; with group read admitted (Q1), any process in the pod holding the `fsGroup`. Not other UIDs on the node. |
| Same-UID process | Can call Secret Service for the same items (same-UID compromise is outside the promise, `contracts/cli/v1alpha1/semantics.md:497-498`). | Can read the file directly. Same promise boundary. |
| Tampering / substitution | Item attributes are scoped and verified; the writer refuses an existing version. | Read-only mount (`ST_RDONLY`), no write bits, root-or-owner ownership, and `..data` resolved only to a root/owner-owned sibling. A substituted item is caught by the fingerprint only after a `revalidate` re-pins it. Until then, reads of the old version answer `Missing` rather than serving new bytes. |
| Symlink / path redirection | Every storage component is admitted without symlinks (`docs/local-secret-service.md:49-51`). | `O_NOFOLLOW` everywhere. The single admitted indirection is `..data` → one same-directory `..`-prefixed sibling. |
| Rotation | Connectors writes a new immutable version. | The platform replaces the item; connectors re-pins on `revalidate` and never follows silently. |
| Deletion / revocation | Connectors deletes the item after retention. | Connectors cannot delete. Revocation removes connectors' use of the item, and the operator must also delete or rotate the Secret. The documentation says so in the revoke section. |
| Failure mode | Locked/absent store → `custody_unavailable`, no prompt. | Missing or failed item, writable mount → `custody_unavailable`; any write → `custody_read_only`. Never another binding. |
| Disclosure through connectors | No values, locators or scope ids in output, logs or errors. | Same. The directory is in owner configuration only, and item paths never appear in results or diagnostics. |

**How this differs from the refused plain-text fallback.** The platform story refused a plain-text
*fallback*: a file or environment variable that custody would use when the keyring is
unavailable. In that design a desktop user's credential silently ends up on disk, and anyone
who can plant a file can steer which credential is used. `mounted_secret` differs on every one of
those points:

1. **Explicit selection, never automatic.** It is used only when the owner configuration names
   it. A locked or absent keyring still answers `custody_unavailable`, exactly as today.
2. **Connectors never writes plain text.** There is no write path, so no credential connectors
   receives through `--credential-file`, a prompt or stdin is ever persisted outside a protected
   store.
3. **Read-only and owner-bound checks** (regular file, single link, no write bits, owner or
   root, `ST_RDONLY` mount, no symlink except `..data`). An ordinary writable file in a home
   directory is refused even if selected.
4. **The material's custodian is the platform.** It is a secret the platform already holds and
   delivers. Connectors does not create a new copy.

## Spec first

Model, validate with the newest `ess`, regenerate, then implement against the generated code.
The pin today is `crates/connectors-spec/toolchain.json` = `0.59.0`; move it to the newest
release first. Changes:

- `ess/domains/cli.yaml`
  - `connectors.cli.CustodyBackend` (`:302-308`) gains `mounted_secret`. The comment changes
    from "no plain-text variant" to "no fallback variant; `mounted_secret` is explicit and
    read-only".
  - `connectors.cli.LocalCustodyBinding` (`:142-147`) gains `binding:
    Optional<connectors.cli.CustodyBackend>` and `directory: Optional<String>`. The invariants
    are: `directory` is defined iff `binding == mounted_secret`, and `secret_service_socket`
    is not defined when `binding == mounted_secret`.
  - `connectors.cli.ConfigurationFormat` (`:517-519`) gains `connectors-local/4`.
  - `connectors.cli.FailureCode` (`:508-510`) gains `custody_read_only`.
  - `connectors.cli.Prerequisite` names gain `mounted_custody`, used in `SetupCheckResult`
    (`:680-686`).
  - The configuration-refusal rule names belong where `Refusal` is modelled. If ESS cannot
    express a closed rule-name set on a refusal, stop and report.
- `ess/domains/auth_bindings.yaml`
  - `CustodyVersion` lifecycle (`:240-249`) gains a terminal `Released` state and a
    `release` transition `Retiring → Released`.
  - `RetireCustodyVersion` and `AcknowledgeCustodyDeletion` (`:528-553`) gain the `released`
    branch.
  - `store_version` stays opaque (binding-private).
- `ess/components.yaml` (`:33-37`) lists the new command if `release` is its own command.
- `contracts/auth/custody/v1alpha1/semantics.md`
  - §5.1 (`:130-152`) becomes "one platform binding per build, plus the explicitly selected
    read-only `mounted_secret`", with a table row and the checks.
  - §4 gains "a read-only binding provides no `write_new`/`delete`; retirement is `Released`".
  - §6 gains the mounted conformance scenarios.
  - §9 maps `Released`.
  - §10 records that the "Second binding after file" decision is taken for read-only.
- `contracts/cli/v1alpha1/semantics.md`
  - §2 (`:59-93`): connect/repair under a read-only binding take no source.
  - §3 (`:264-311`): `connectors-local/4` and `[custody]`; keep "no keyring locators" and add
    "the mounted directory is an infrastructure binding".
  - §5 (`:463-471`): keep "never fallback custody" and add the explicit-selection carve-out.
  - §6 failure table: `custody_read_only`.
- `contracts/cli/v1alpha1/scenarios.md` gains new rows `C10 mounted custody` (or the next free
  id): selection, refusals, read-only write paths, rotation, `env -i` container run.
- `contracts/cli/v1alpha1/fixtures/` gains `config-valid-mounted.toml` and refusal fixtures.
- `apps/connectors/spec/cli.yaml` gets no new global. If the connect source flags are modelled
  there as required-one-of, record that none is accepted under a read-only binding.
- Regenerate with `cargo run --locked -p connectors-docs -- generate` (CLI reference,
  contracts and model reference, status page). The status item names its holding test
  (`crates/connectors-docs/src/status.rs`).

## Acceptance

Each item is a named test; the conformance scenarios are listed in `contracts/cli/v1alpha1/scenarios.md`.

- `custody::mounted::contract_suite_read_only`: the generic contract suite
  (`crates/connectors-custody/src/contract.rs`) runs its read scenarios against
  `mounted_secret` over a tmpfs fixture remounted read-only. Write, confirm and delete do not
  compile against the read capability, which a `trybuild` compile-fail case shows.
- `custody::mounted::refuses_each_check`, one case each: symlinked item; `..data` pointing at
  `/`, at `../x` and at a non-`..` name; hard-linked item; FIFO; item with a write bit; item
  with other-read; item owned by another UID; group-read with a group the process does not
  hold; writable mount (`ST_RDONLY` clear); over 64 KiB. Each answers `Unavailable`, and its
  error and log text contain neither the bytes nor the path.
- `custody::mounted::projected_layout_reads_through_data`: a fixture laid out as a kubelet
  atomic writer lays it out (`..data -> ..2026_10_13_…`, flat key symlinks present) reads
  `alias/profile` through `..data` and never opens the key symlinks.
- `local_cli::mounted_selection_is_explicit`:
  - With no `[custody]`, an unavailable Secret Service still answers `custody_unavailable`
    even when a valid mounted directory exists at the documented path.
  - `[custody]` in a `/3` file refuses the configuration.
  - `binding = "mounted_secret"` with `secret_service_socket` refuses.
  - `setup check` reports `custody_backend = mounted_secret`.
- `local_cli::mounted_connect_reads_never_writes`: `connect` takes no source flag (a source
  flag is `invalid_input`). It publishes a connection whose read returns the item. A second
  connect for the same pair is `custody_read_only`. The mount and the platform store are
  byte-for-byte unchanged afterwards.
- `local_cli::mounted_rotation_requires_revalidate`: after the item's bytes change, invoke is
  refused with `revalidate_connection` and the new bytes are never sent. `revalidate` re-pins,
  and the next invoke uses the new bytes.
- `local_cli::mounted_revoke_releases`: revoke and reclamation leave the item in place and end
  the version `Released`, never `Deleted`.
- `local_cli::mounted_refuses_refresh_and_approval_keys`: a refresh-rotating profile and
  approval key creation each answer `custody_read_only` before any provider exchange or
  write.
- `local_cli::configuration_refusal_names_rule`: under `env -i` with no flags, the refusal is
  `invalid_configuration` and names `config_path_unresolved`. With an empty `HOME` it names
  `config_missing`. A group-readable config names `config_not_private`.
- `container::env_i_mounted_custody_round_trip`: a container-like run under
  `env -i` with **only** `--config` and `--state-dir` (no `HOME`, no `PATH`). The fixture
  adapter, a read-only tmpfs in projected layout, group read through a supplementary GID and
  a non-root UID run `setup init`, `setup check`, `connections connect`, `connections list`
  and a credentialed `operations invoke`, all succeeding.
- `container::kind_secret_volume` (CI only; ignored locally and listed in the ignored
  registry, `crates/connectors-build/src/ignored.rs`): the documented manifest runs in a kind
  cluster. It confirms the Kubernetes facts this design infers (`0440 root:<fsGroup>`,
  `ST_RDONLY`, `..data` layout) and the same round trip.
- `task check`: ESS validation, generation drift, the conformance suite and the docs drift
  check are green. The status page lists `mounted_secret` as shipped, held by
  `container::env_i_mounted_custody_round_trip`.

## Scope

- `crates/connectors-custody`
  - New `src/mounted.rs` (Linux).
  - `src/lib.rs`: split `Backend` into read and write capabilities, a `CustodyBackend::MountedSecret`
    variant, and an `open`/`inspect`/`available` that take the configured selection.
  - `src/contract.rs`: a read-only suite.
  - `Cargo.toml` description.
- `crates/connectors-host`
  - `src/local/config.rs`: `/4`, `[custody]`, named refusal rules.
  - `src/local/keyring/custody.rs`: the coordinator over the selected binding; read-only
    `acknowledge`.
  - `src/local/keyring.rs`.
  - `src/local/registry/lifecycle.rs`: publish without write, `Released`.
  - `src/local/approval_keys.rs`: refusal.
  - `src/local/owner.rs`, `owner/transport.rs` and `owner/supervisor.rs`: replace the
    `custody::available_at(secret_service_socket)` call sites with the selection.
  - `src/local/filesystem.rs`: the `..data` resolver and `fstatvfs`.
- `apps/connectors`
  - `src/local.rs`: `setup check` prerequisite and backend.
  - `src/local/connections.rs`: connect without a source under a read-only binding.
  - New tests: `tests/mounted_custody.rs` and `tests/container_env_i.rs`.
- Spec and contract files as listed under Spec first, plus regenerated outputs.
- Documentation
  - New `docs/local-mounted-secret-custody.md`: binding design and threat model.
  - New `docs/local-container.md`: container and CronJob setup.
  - `docs/local-secret-service.md`: a threat model section, and a link to the sibling binding.
  - Website guide.
  - `CHANGELOG.md`.

## Not in scope

- A plain-text, environment-variable or automatic fallback of any kind, on any platform.
- A writable file custody, an encrypted-file custody, Vault, a cloud secret manager, or reading
  a Kubernetes Secret through the API (CSI Secrets Store, or a `kubernetes.secret.read`
  operation, which is `story:parity-kubernetes-secret-read`).
- `mounted_secret` on macOS or Windows.
- Mounted approval-signing seeds, and refresh-rotating OAuth profiles under a read-only binding.
- More than one connection per `(alias, profile)` under `mounted_secret`.
- Admitting a ConfigMap-mounted `config.toml`, or relaxing the configuration file's owner-only
  rule.
- A container image, Helm chart or registry publication. The documented setup is an example
  manifest.

## Questions for the operator

Decided 2026-10-13 (decision-blocker:mounted-secret-custody-trust, cleared), within the approved headless-custody design:

- **Q1 (a).** A secret file readable by group is admitted only when the group is one the process
  holds, the file is owned by root, it has no write bits and no other bits, and it sits on a
  read-only mount. This is how Kubernetes mounts a Secret for a non-root pod. Cost, stated in the
  design: every process in the pod that holds that group can read the file.
- **Q2 (c).** Today's ancestor rule stays: the documented container setup gives the state
  directory a writable image layer. Admitting a sticky or setgid mount ancestor when the final
  directory is `0700` and owned by the process is a separate story,
  `story:state-directory-on-mounted-volume`.
- **Q3 (a).** A mounted version is pinned as an HMAC-SHA-256 of its bytes under a random key held
  in an owner-only file in the state directory.
