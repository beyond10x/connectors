# Consumer launch v1alpha1

- **Status:** implemented for the local Linux CLI (story
  `launch-consumer-with-connection-credential`, design decisions of 2026-10-06 as
  corrected the same day for argument and environment passing).
- **Owner:** the [local CLI binding](semantics.md); custody party in
  [auth.custody](../../auth/custody/v1alpha1/semantics.md) § 4.2.

`connections launch` starts one operator-pinned **consumer** executable with one
connection's protected document on descriptor 3. The operator pins the executable
and its argv prefix; the caller names the consumer and the connection and may add
arguments and pass environment variables the consumer's entry admits.

```sh
connectors connections launch --adapter ALIAS --connection CONNECTION --consumer NAME \
  [--args '["subcommand","--flag","value"]']
```

## 1. Configuration

A consumer is declared in a `connectors-local/3` file. Format `/3` is `/2` plus
`[consumers]`: every adapter entry still selects a `private_protocol`, and `/1` and
`/2` files keep loading unchanged. A `[consumers]` table in a `/1` or `/2` file
refuses the whole configuration. `setup init` still writes `/2`.

```toml
format = "connectors-local/3"

[consumers.store]
pass_env = ["STORE_"]

[consumers.store.executable]
path = "/opt/consumer/bin/store"
sha256 = "<64 lowercase hex digits>"
args = ["--password-file", "/proc/self/fd/3"]

[consumers.store.permissions]
connections = ["warehouse"]
```

| Key | Rule |
|---|---|
| `consumers.<name>` | an admitted selector; at most 64 consumers |
| `executable.path`, `sha256` | as for an adapter: an absolute, owner-admitted regular executable whose content must match the digest at launch |
| `executable.args` | the consumer's pinned argv **prefix** after argv\[0\]; at most 256 arguments, each at most 4096 bytes without NUL. The caller's `--args` follow it. |
| `pass_env` | name prefixes of the caller's environment variables passed to the consumer; at most 64, each 1–256 bytes without `=` or NUL. Omitted or empty passes nothing. |
| `permissions.connections` | the adapter aliases whose connections this consumer may receive; at most 64, each a configured alias. Omitted or empty denies every connection. |

Unknown keys in a consumer entry refuse the configuration.

## 2. Arguments and admission

`--args` is optional and is one option value: a JSON array of strings. Any other
value — not JSON, not an array, a non-string element — or an element holding NUL is
refused before admission as `invalid_input`, stage `arguments` (a usage refusal,
exit 2). An empty array adds nothing.

Admission runs in the CLI before any owner starts, and again in the owner before
the credential is read. Its order decides which refusal a caller sees:

| Step | Refusal (`code`, `stage`) |
|---|---|
| the adapter alias and the consumer are configured | `not_found`, `admission` |
| the consumer lists the adapter in `permissions.connections` | `forbidden`, `admission` |
| the consumer's file matches its pinned `sha256` | `invalid_configuration`, `configuration` |
| the connection is visible to the adapter's instance, not revoked | `not_found` / `revoked`, `admission` |
| the connection's profile is in the adapter's `permissions.profiles` | `forbidden`, `admission` |
| the connection was saved under the configured revision | `lifecycle_conflict`, `admission` |
| the connection's evidence is current | `unavailable`, `readiness` |
| custody is available | `custody_unavailable`, `custody` |

The binding is the saved connection record's: provider authority and profile
declaration as the connection was saved, under the configured instance, adapter
and revision. **No adapter process starts for a launch**, and no provider is
contacted. Lapsed evidence keeps the existing readiness contract (`unavailable`,
not `not_granted`); `connections revalidate` renews it.

## 3. Delivery

The owner process reads the credential; the CLI process does not.

1. The owner captures the consumer image into a sealed snapshot and verifies its
   digest, before reading the credential. A wrong digest is refused before exec.
2. The owner takes a bounded read use of the connection's current generation
   (`capture_read`), reads its material from custody, re-reads the configuration
   (a changed consumer entry, adapter binding or custody socket is
   `lifecycle_conflict`), and opens the use (`dispatch_read`).
3. It writes the material **verbatim and uninterpreted** into a memfd, seals it
   (`F_SEAL_WRITE`, `F_SEAL_GROW`, `F_SEAL_SHRINK`, `F_SEAL_SEAL`) and reopens it
   read-only.
4. It sends the admitted entry's argv prefix and `pass_env`, then the image and the
   credential as two descriptors (`SCM_RIGHTS`) over the owner socket, and releases
   the read use. The sealed copy is the consumer's from then on.
5. The CLI places the credential at descriptor 3 and executes the image through
   its descriptor, without reading either.

## 4. Process rules

- **No shell, no PATH.** The captured image runs through `/proc/self/fd/N`;
  argv\[0\] is that path, followed by the pinned prefix, followed by the `--args`
  elements verbatim. Nothing is expanded, split or quoted.
- **Environment by prefix only.** The consumer's environment holds exactly the
  caller's variables whose names start with one of the entry's `pass_env`
  prefixes; every other variable is dropped. An entry without `pass_env` gets an
  empty environment.
- **The caller's stdio is the consumer's.** stdin, stdout and stderr are inherited.
  The CLI writes nothing to stdout when a consumer ran.
- **Exactly descriptors 0–3.** The consumer starts with stdin, stdout, stderr and
  the credential on 3, and no other descriptor: every descriptor from 4 up is
  closed at exec, including the CLI's own (owner socket, image, bus connections)
  and any the caller left open without close-on-exec, such as a shell's
  `exec 7<file`. Between fork and exec the CLI marks them close-on-exec with
  `close_range(4, ~0, CLOSE_RANGE_CLOEXEC)`, or, before Linux 5.11, one by one up
  to the `RLIMIT_NOFILE` soft limit; marking rather than closing keeps the image's
  descriptor open for the exec that runs it.
- **The exit status is the consumer's**: its exit code, or 128 plus the signal
  number when a signal ended it. While the consumer runs the CLI ignores `SIGINT`
  and `SIGQUIT`, as `system(3)` does; the terminal delivers them to the consumer.
  If the CLI dies first, the consumer receives `SIGTERM`.
- **A refused launch** writes the ordinary `failure` envelope to stderr and exits 1,
  or 2 for a usage or configuration refusal; no consumer runs.

## 5. Boundary

What the operator pins and what the caller supplies are separate:

| Pinned by the operator (configuration) | Supplied by the same-user caller |
|---|---|
| the consumer executable and its digest | `--args`, appended after the prefix |
| its argv prefix | the values of variables matching `pass_env` |
| which adapters' connections it may receive | which consumer and connection to launch |
| which variable name prefixes pass | |

The credential reaches the consumer only on descriptor 3. No CLI argument,
configuration field, owner reply document, CLI response, and no file under the
state directory carries the protected document; the consumer's argv and environment
carry only what the operator pinned and what the caller supplied. The CLI process
holds the sealed descriptor only between receipt and exec and never reads it.

The boundary is the operating-system user, as for every owner request: any process
of the configured `owner_uid` can ask the owner for a launch of a configured
consumer, with any arguments and passed variables, and receives the same
descriptors. A consumer entry is an operator decision about which executable may
receive which adapter's connections, starting with which argv, not a defence
against other processes of the same user. A consumer that must not act on
caller-chosen arguments or variables must refuse them itself.

## 6. Verification

- `crates/connectors-host/src/local/runtime/launch/tests.rs`: sealing, descriptor
  passing, descriptor 3, argv prefix plus caller arguments, environment by prefix,
  exit-code mapping and admission order, in the ordinary lane;
  `disposable_consumer_launch_delivers_fd3_and_refuses_by_code` drives the
  production CLI and owner against a disposable Secret Service (opt-in, family
  `disposable`).
- `apps/connectors/tests/consumer_launch.rs`: configuration and `--args` refusals
  through the production CLI, before any owner starts.
