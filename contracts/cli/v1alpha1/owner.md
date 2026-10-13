# Local owner transport

This implements the existing [local CLI](semantics.md) owners. SQLite remains the
only metadata authority, and Secret Service remains the only credential custody.
The owner is a local `connectors` process with an owner-only socket and a retained
kernel file lock in the selected private state directory. It is bound to one
admitted absolute configuration path and the database authority UUID.

Passive commands contact only an existing socket or read recognized metadata.
Admitted connect, repair, revalidate and invoke may start the owner. Competing startups acquire
the same lifetime lock before spawning; failure to acquire it never authorizes a
replacement process. A startup socket carries a fresh challenge, owner incarnation
and metadata authority before the parent releases startup ownership. The daemon
keeps the inherited lock and owns its children after the launching CLI exits.
An owner that refuses to start or to keep serving exits with status 1 and writes
only its refusal code to standard error (`connectors owner: "<code>"`), which
the launching CLI discards.
Once it starts admitting requests, that lock is retained until kernel process
exit even on early return or unwinding. Recovery runs between jobs on a retained
worker, or under the supervisor's worker-creation lock with no live worker for
that instance. A busy/idle observation alone never owns the interval used for the
[local mutation recovery binding](../../service/local-mutations.md).

An owner exits after 10 minutes with no connected client and no work in flight:
no queued or in-flight request (to an adapter child or otherwise) and no adapter
child being started. A live adapter child with nothing in flight is not work,
and neither is an `automatic` adapter by itself. Every accepted connection and
every such piece of work restarts the interval. Recovery restarts it only when
it settles an attempt: a pass that finds nothing pending, or that settles
nothing (such as a Prepared attempt with no qualified clock), is not work, so
such an attempt cannot keep an owner alive. It stays pending in the store and
the next owner resumes it. A recovery pass still running when the interval ends
delays the exit until it finishes. The owner first removes its socket, so no
later connect reaches it, and then serves any connection that reached the socket before the removal; if
one did, it listens on a new socket and stays. Otherwise it stops recovery and
its supervisor, idle children included, as a shutdown request does, and the
lifetime lock is released at process exit. A command arriving meanwhile finds no
socket, or has its stream closed after the socket it reached was removed: one
that may start an owner treats that as no owner, greets a newer owner already
listening or waits for the lock and starts one, and a passive command reports
no owner. Only a test build can shorten the 10 minutes.

Private requests reuse the bounded three-section framing of the
[adapter binding](private-adapter.md), with distinct `connectors-owner/1` greetings.

A greeting carries `build`, the SHA-256 of the caller's own running executable
image, and the owner's reply returns its own. A caller that sends no `build` gets
none back, so earlier callers still read the reply. The CLI sends no admission,
capture, invocation, revalidation, status or stop request to an owner whose
`build` differs, and fails with `owner_build_mismatch`, stage `readiness`,
next action `stop_owner`. An owner built before this field closes the stream
without a reply on the unknown field. A silent close alone proves nothing, so
the CLI then greets without `build` and, on that established stream, sends a
`build` request. An owner that answers it knows the handshake and is greeted
again with `build`; only an owner that accepts the greeting and then closes on
the unknown request is identified as an earlier build and refused the same way,
rather than reported unavailable. Any reply, including a refusal, keeps its own
code. An owner at its client limit answers `capacity` rather than closing
without a reply. The CLI never signals, stops or replaces that owner: the
user stops the running `__connectors-owner` process for the state directory, and
the next admitted command starts one from the current build. The private
shutdown request stays accepted across builds so a lifecycle harness can stop it.
The actual socket peer must be the configured UID. No credential, source path,
keyring tuple or continuation is part of ordinary control JSON. A capture admission
returns only the profile, acquisition reference and original 300-second expiry.
Its private acquisition handle stays with this live socket; reconnecting with a
public acquisition reference cannot complete it. Completion consumes it once,
validates the separately transferred document through the exact adapter, and then
uses the existing guarded custody/publication coordinator. Lost acknowledgement
is unknown and can be resolved only through status, without replay.

A command run while an owner of its own build runs for the state directory reads
the store through that owner instead of opening it. The owner answers a `cached`
request (`connectors.cli.LocalOwnerCachedRequest`, `{"kind":"cached","adapter":…}`)
with `connectors.cli.LocalOwnerCached`, the adapter's cached bootstrap read from
the owner's held metadata handle, and refuses with the codes a direct read gives
(`not_found`, `description_unavailable`, `metadata_unavailable`). `operations
invoke` with such an owner running sends no admission of its own: the owner's
admission of the `invoke` request decides it, before dispatch, from the same held
handle. The authority a greeting names is read from the database's authority row
without replaying the store. Without a reachable owner of the same build a
command reads the store directly, which verifies it completely, and `operations
invoke` admits directly before it starts an owner. A `cached` exchange that fails
after the greeting, an owner's refusal included, is answered by that direct
read, so a refusal carries the direct read's own code. The owner itself never routes
its reads through an owner.

The generated parser's protected Sources hook retains one clearing buffer and
one admitted live owner channel, and supplies only a nonsecret marker to the
generated String/JSON carrier. The paired handler checks the original selectors
and marker before completing this one capture. The native adapter alone interprets
the document. Ordinary business JSON has separate bounded sources and is validated
against the exact cached operation/schema before dispatch. Current owner policy,
bootstrap, registry generation/fence, evidence and native permission remain required.

SQLite migration three adds the configuration owner's cached bootstrap and durable
stop suppression, keyed by the existing configured instance identity. A cache is
always stale information. Updating a snapshot never clears suppression. Exact stop
commits suppression before signalling the owned incarnation; explicit admitted
connect/repair/revalidate/invoke is the only resume action. No PID in a cache grants process
ownership. Old/future or incomplete migrations remain explicit refusals.

Configuration is reloaded for admission and immediately before protected completion
or business dispatch. A changed active artifact/configuration cannot replace its
owned child implicitly. Profile and operation allowlists only narrow its declarations.
Each child lives on a retained worker thread for the Linux parent-death signal;
startup is coalesced per instance and at most four launches run concurrently.
Per-owner connections and work queues are bounded, and original request deadlines
are never reset by queueing, startup, capture or provider work.

CLI reply reads check cancellation while waiting, including an incomplete frame;
the bounded polling interval never resets the original deadline or replays a
request. A received terminal success wins a concurrent interruption. Disconnecting
after protected completion may leave publication unresolved to that client; its
acquisition reference remains the recovery route. Already dispatched read work may
finish within its original deadline after the CLI disconnects. No business write
or automatic repeat is admitted by this binding.

Explicit `connections revalidate` admits the selected public connection and its
semantic revision under the configured profile policy before startup. It captures
the exact retained generation/version and private publication fence for at most
30 seconds, independent of stale baseline evidence, then resolves that version
through qualified custody. The existing bounded transient-use records retain the
version during validation; they create no new generation or custody owner. The
capture is consumed once before the declared native identity/grant read. The
original deadline includes queueing, startup, custody and provider work.

Successful same-identity evidence replaces the one baseline under the unchanged
generation and current binding/fence. Publication advances the private fence,
cutting pending reads, repairs and competing recollections; the public semantic
revision and custody version stay unchanged. Known invalidity, expiry, retirement
or revocation cannot be cleared by recollection. Positive invalidity or a proved
identity mismatch in this retained material cuts off that material; transient
provider/custody failures preserve any still-valid baseline. No credential is
written, no new acquisition is created, and business invocation never performs
implicit revalidation. A lost publication acknowledgement is outcome_unknown;
connection status observes the resulting baseline and no request is replayed.

## macOS

The rules above are the Linux transport and stand unchanged. On macOS the
owner keeps its socket, lifetime lock, handshake and refusals; four guarantees
hold only in the weaker form the [platform guarantee table](semantics.md#macos-platform-guarantees)
names:

- **Build** (`own_image_build_measure`: `launch_path_read`). There is no
  `/proc/self/exe`. `build` is the SHA-256 of the file at the running
  executable's path, read once. An owner whose file was replaced after it
  started is therefore measured as a new build, and a caller of the replacing
  build is refused with `owner_build_mismatch` exactly as against another build.
  A CLI that starts an owner runs it from its private launch copy and keeps
  that copy until the owner has answered the startup greeting, since the owner
  measures its build there before it answers.
- **Socket peer** (`peer_credential_snapshot`: `separate_pid_query`). The peer
  UID comes from `LOCAL_PEERCRED` and must be the configured UID, as on Linux;
  the pid is a separate `LOCAL_PEERPID` query used only for its sign. A peer
  that has already closed, such as a startup channel nobody greets, has no pid
  to query; it is admitted by its connect-time UID as on Linux, and its stream
  reads what was sent before the close and then end of file.
- **Children** (`parent_death_kill`: `adapter_channel_close`;
  `exact_process_signal`: `child_reap_lock`). There is no parent-death signal,
  so no retained worker thread is needed for it: an adapter child exits when its
  channel closes. Exact stop signals the owned child by process id only under
  the lock that reaps it; kqueue `EVFILT_PROC` with `NOTE_EXIT` reports its exit.
- **Socket path.** There is no `/proc/self/fd`: the owner socket is bound and
  reached by its full path, which must fit the 104-byte `sun_path`. A state
  directory whose socket path is longer is refused before an owner starts.

## Windows

The rules above are the Linux transport and stand unchanged. On Windows the
owner keeps its lifetime lock, handshake and refusals; the transport and the
guarantees change as the [platform guarantee table](semantics.md#windows-platform-guarantees)
states:

- **Pipe.** There is no Unix socket. The owner listens on a named pipe whose
  name is derived from the current user's SID and the state directory's
  canonical path, created with `FILE_FLAG_FIRST_PIPE_INSTANCE` and an access
  control list granting the current user's SID alone. An owner that cannot
  create the first instance does not serve. Before sending anything, the CLI
  checks that the pipe's server process (`GetNamedPipeServerProcessId`) runs as
  the current user; a pipe held by another user refuses as no owner, never as an
  owner of another build. Removing the socket at idle exit is closing every
  instance of the pipe, after which no later connect reaches the owner.
- **Lifetime lock.** The kernel file lock is an exclusive `LockFileEx` on the
  lock file in the state directory, released at process exit.
- **Pipe peer** (`peer_credential_snapshot`: `client_token_query`). The pipe's
  access control list admits only the current user at connect. The owner reads
  the client's process id with `GetNamedPipeClientProcessId`, opens that process
  once, and requires its token's SID to be the current user's; the process
  handle opened then is the one every later handle duplication into that client
  goes through.
- **Handles in place of descriptors** (`atomic_close_on_exec`: holds). Where
  the Linux transport sends descriptors with `SCM_RIGHTS`, the owner duplicates
  each handle into the client process with `DuplicateHandle`, inheritance off and
  no more access than the receiver needs, and sends the handle's value in the
  control section. A handle the client does not take over is closed by the
  owner with `DUPLICATE_CLOSE_SOURCE` from the same process handle.
- **Build** (`own_image_build_measure`: `launch_path_read`). There is no
  `/proc/self/exe`. `build` is the SHA-256 of the file at the path
  `GetModuleFileNameW` reports, read once. A running image cannot be written in
  place but can be renamed; a file put at its path is measured as a new build,
  and a caller of that build is refused with `owner_build_mismatch`.
- **Children** (`parent_death_kill`: holds; `exact_process_signal`: holds).
  Adapter children are created in a job object with
  `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` whose only handle the owner holds, so
  they end when the owner does; no retained worker thread is needed for that.
  Exact stop terminates the owned child through its process handle, and waiting
  on that handle reports its exit.
- **Detaching from the CLI.** The CLI starts the owner with
  `CREATE_BREAKAWAY_FROM_JOB` and `DETACHED_PROCESS`, so the owner outlives the
  launching CLI. A CLI that runs inside a job that does not permit breakaway
  starts an owner that lives only as long as that job. The startup channel and
  the lifetime lock reach the starting owner as inherited handles, the only ones
  besides its standard handles in its `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`, in place of descriptors 3 and 4.
