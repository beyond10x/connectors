# Private local adapter binding

This binding implements the local [CLI lifecycle and custody requirements](semantics.md)
between the host supervisor and an exact configured adapter executable. It is
separate from the public service protocol and existing `--config` service mode.
The values below use existing service, profile, connection and generation owners;
they introduce no database, credential registry or independent lifecycle entity.

## Admission and bootstrap

The supervisor admits the executable file descriptor, captures its bounded bytes
into a private memfd, verifies the configured SHA-256 and ELF format against that
capture, and seals it against writes, growth and truncation before exec. Path
replacement and in-place writes to the original cannot change the admitted
snapshot. A kernel policy that supplies non-executable memfds is a refusal; this
binding does not override that policy. It launches
without a shell, PATH search, inherited credential environment or diagnostic
forwarding. The only private channel is an inherited Unix socket. Its peer UID
must match the current owner. Socketpair peer credentials identify its creator,
not a later child: the supervisor's Child/pidfd ownership establishes the exact
spawned process. A fresh challenge and child incarnation bind the bootstrap to
this channel. No PID or socket path supplied by a caller grants process authority.

The child independently reads its admitted nonsecret native configuration and
computes its canonical configuration revision. The supervisor compares returned
instance, adapter, protocol and configuration revision with the configured
selection, and validates bounded descriptor/profile/operation declarations before
any protected entry. Merely echoing a supplied configuration revision is insufficient.
The executable digest is the reviewed source of these declarations, not discovered
metadata that can expand authority. Host policy still selects profiles and enabled
operations; a child declaration alone grants neither.

At most 64 profiles and 256 operations are described. A static-entry profile
includes its immutable id/revision, purpose/subject, scheme/capability, minimum
scopes and evidence lifetime. Native terminal fields are at most 16 distinct
string fields, each with a safe label and bounded byte length. Native code owns
all field interpretation, strict document decoding, required provider validation
and permission checks. The first generic registry admits only its already
supported credential-bearing static-entry subset.

Operation declarations bind exact descriptor ids to one selected profile, required
native scopes and an authored effect classification. Unknown or writing operations
refuse until their approval/audit/dispatch binding is available. Host policy,
profile minimums, operation scope requirements and native target permission all
remain separate checks. Business adapters receive a fixed authenticated
capability; only the trusted composition reads the protected document.

## Private transfer

Frames contain a 12-byte big-endian header with the lengths of three following
sections: safe typed JSON control (at most 1 MiB), protected bytes (at most 64 KiB),
and operation-local JSON document bytes (at most 8 MiB, with requests capped at
1 MiB). Lengths and the current original deadline are checked before allocation.
Truncation, unexpected protected sections, duplicate/unknown control fields,
wrong request identity or invalid response branch closes the channel. There is
no automatic replay. Control values never contain protected bytes or a keyring
locator. Protected buffers clear on drop; they never implement Debug or Serialize.

Validation uses the one admitted candidate document and returns only baseline
identity/grants/expiry/time observations. The host retains the original document
as opaque material only after native validation and definite custody acknowledgement.
The child receives no database handle, arbitrary credential resolver or publication
permit. Invocation receives the exact retained document for the captured use and
an opaque cursor partition; that partition grants no credential lookup authority.
Responses contain one safe result or closed failure; raw exceptions and child
stdout/stderr are never forwarded.

Control objects have a required `kind` discriminator and no unknown fields. The
request/response identity is one fresh opaque id per exchange; it is not an
idempotency key or authority to repeat provider work.

| Kind | Additional control fields | Protected / document section |
|---|---|---|
| `hello` | `version`, `nonce`, `child_incarnation` | both empty |
| `ready` | same challenge fields, `bootstrap` | both empty |
| `validate` | `request_id`, `profile`, `deadline_ms` | candidate document / empty |
| `validated` | `request_id`, `baseline` | both empty |
| `invoke` | `request_id`, `operation`, `revision`, `partition`, `deadline_ms` | exact retained material / decoded provider-input carrier |
| `success` | `request_id` | empty / provider-result carrier |
| `stop` / `stopped` | `request_id` | both empty |
| `failed` | `request_id`, closed `code` | both empty |

Version is `connectors-private/1`. Startup challenges and child incarnations are
UUIDs. Exchange identities and cursor partitions use the shared bounded identifier
grammar. Deadlines are absolute Unix milliseconds with at most 120 seconds
remaining; native validation further caps execution at 30 seconds. Malformed
framing closes the channel. Native refusal is a safe failed response. A remote
timeout/interruption also terminates the owned channel. If the peer closes at its
deadline before the caller's socket timeout, the caller observes unavailability;
neither observation permits transport replay.

The required CLI capture integration remains unwired. It uses the generated parser's Sources hook after host admission. The
production hook retains protected bytes in its own clearing buffer and returns a
fixed nonsecret marker through the generated String carrier. The paired production
handler consumes the private buffer for that one call, checking the marker and
original admission. This binding keeps secrets out of the parser's ordinary
`serde_json::Value` allocations; the marker is neither an identity nor a resumable
capability. Recording/demo handlers cannot obtain the protected buffer.

## Process loss and deadlines

Startup is bounded by ten seconds; each exchange retains its original deadline.
A timeout or malformed response invalidates the channel and terminates only the
owned child. The host's durable acquisition/dispatch state determines any unknown
outcome; a new channel never authorizes repeating an exchange. The adapter sets a
parent-death signal before exec and checks that the owner still exists. Spawning
must occur on a thread retained for the owner's lifetime because Linux binds that
signal to the parent thread. Stop and cleanup use the exact owned pidfd/Child;
they never signal a PID recovered from metadata.

Linux mechanics are grounded in the upstream [Unix socket manual](https://man7.org/linux/man-pages/man7/unix.7.html),
[parent-death signal manual](https://man7.org/linux/man-pages/man2/PR_SET_PDEATHSIG.2const.html),
[pidfd signaling manual](https://man7.org/linux/man-pages/man2/pidfd_send_signal.2.html)
and [execveat manual](https://man7.org/linux/man-pages/man2/execveat.2.html).
Immutable executable capture uses the Linux [memfd and sealing interface](https://man7.org/linux/man-pages/man2/memfd_create.2.html).
Process restart tests do not establish power-loss durability or provider acceptance.
