# Local owner transport

This implements the existing [local CLI](semantics.md) owners. SQLite remains the
only metadata authority, and Secret Service remains the only credential custody.
The owner is a local `connectors` process with an owner-only socket and a retained
kernel file lock in the selected private state directory. It is bound to one
admitted absolute configuration path and the database authority UUID.

Passive commands contact only an existing socket or read recognized metadata.
Admitted connect, repair, revalidate and invoke may start the owner, and so may
`operations describe` and `operations list`, only to
[launch for a description](semantics.md#launching-for-a-description). Competing startups acquire
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

A Unix owner also exits once nothing can reach it or its state is gone: when the
socket it listens on is no longer in its state directory (it was removed, or
another file or socket now holds its name), or the state directory's path no
longer names the directory the owner opened (it was removed or replaced, or an
ancestor was, so that the path names nothing or can no longer be searched). The
socket it listens on is the one it bound last: the socket idle retirement binds
again when a client arrived or work began during retirement is that socket, not a
replacement.
A look at either path that fails for any other reason proves nothing, and the
owner keeps serving. The owner checks both at least every 250 ms. It then accepts
no further connection, lets the clients already connected finish for at most 5
seconds, and stops recovery and its supervisor, idle children included, as a
shutdown request does; a client whose exchange is still in flight then loses its
connection without a reply, and an in-flight mutation's outcome is unknown to it,
resolved only through status. It leaves the socket path alone, which another
directory may now hold.
A Windows owner keeps its state directory open, so that directory cannot be
removed beneath it, and its pipe is not a file; it has neither check.

Only a test build ties an owner to a process. A CLI built with debug assertions
whose environment sets `CONNECTORS_TEST_OWNER_PARENT` to a process id passes that
variable, alone, to an owner it starts on Unix, and that owner exits as above once
the named process has ended (checked at least every 250 ms; an owner whose named
process is already gone exits after answering its startup). A journey harness sets
it to its own process, so an owner it started does not outlive it, however the
harness ends. A release build reads no such variable: its owner stops only on a
shutdown request, at idle retirement or by the checks above.

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
without a reply. No command replaces that owner by itself: the user runs
`connectors setup owner-stop` ([Stopping an owner](#stopping-an-owner)), and
the next admitted command starts one from the current build. The private
shutdown request stays accepted across builds so that command, and a lifecycle
harness, can stop it.
The actual socket peer must be the configured UID. No credential, source path,
keyring tuple or continuation is part of ordinary control JSON. A capture admission
returns only the profile, acquisition reference and original 300-second expiry.
Its private acquisition handle stays with this live socket; reconnecting with a
public acquisition reference cannot complete it. Completion consumes it once,
validates the separately transferred document through the exact adapter, and then
uses the existing guarded custody/publication coordinator. Lost acknowledgement
is unknown and can be resolved only through status, without replay.

A `reuse` request (`connectors.cli.LocalOwnerReuseRequest`,
`{"kind":"reuse","adapter":…,"source":…,"connection":…,"expected_revision":…,"profile":…}`,
story:invoke-definition-reuse-credential) carries no document and no material: it
names the instance to connect, another instance of the same connector, the source
connection with the revision it was offered under, and the profile. The owner
answers it with the published connection, as a completed capture does, or with one
refusal. It repeats every check of the [reuse answer](semantics.md#invoking-a-definition-file)
itself: the target an invoked definition's instance whose lowered
configuration names no URL in its profile and sends the credential to its API base alone, both entries
configured, the same connector id (a legacy `[adapters]` entry is none), another instance and the same adapter (`forbidden`), the profile
granted (`forbidden`), custody available (`custody_unavailable`) and writable
(`custody_read_only`), a static-entry profile (`unsupported`), the instance's
connect admitted (`lifecycle_conflict`), the source a candidate under its own
configured entry (`not_found`, `revoked`, `lifecycle_conflict` for another
revision). It then holds the source's current version under a read use, reads it
from custody, dispatches the use, validates the bytes through the target's
adapter, releases the use, compares the validated identity with the source's
(`identity_mismatch`, `next_action` `create_connection`), re-reads the
configuration, and writes and publishes the bytes as the new connection's own
version through the guarded coordinator, whose publishing commit also requires
the source to be published, Live, not Stale and under the fence and version it
read (`revoked`, `lifecycle_conflict`). A refused dispatch cancels the read use. Every refusal after the allocation fails
that acquisition; a lost acknowledgement is `outcome_unknown` with the acquisition
to follow. The CLI never receives, prints or logs the material, and the source
connection's record is not written by any step but its read use.

A command run while an owner of its own build runs for the state directory reads
the store through that owner instead of opening it. The owner answers a `cached`
request (`connectors.cli.LocalOwnerCachedRequest`, `{"kind":"cached","adapter":…}`)
with `connectors.cli.LocalOwnerCached`, the adapter's cached bootstrap read from
the owner's held metadata handle, and refuses with the codes a direct read gives
(`not_found`, `description_unavailable`, `metadata_unavailable`). With
`"launch": true` (and the named `connection`, when there is one) the owner, finding
no description cached, checks the [launch condition](semantics.md#launching-for-a-description)
again and starts the configured adapter, which caches it. `operations
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
original deadline (below: when it starts) includes queueing, startup, custody and
provider work.

Successful same-identity evidence replaces the one baseline under the unchanged
generation and current binding/fence. Publication advances the private fence,
cutting pending reads, repairs and competing recollections; the public semantic
revision and custody version stay unchanged. Known invalidity, expiry, retirement
or revocation cannot be cleared by recollection. Positive invalidity or a proved
identity mismatch in this retained material cuts off that material; transient
provider/custody failures preserve any still-valid baseline. No credential is
written and no new acquisition is created. A lost publication acknowledgement is
outcome_unknown; connection status observes the resulting baseline and no
request is replayed.

A read on `operations invoke` whose connection's evidence expired while its
credential is intact, or would expire before the read could use it
([semantics.md](semantics.md)), runs this same recollection once, as the first step of its
own job on the instance's serialized worker, within the read's deadline and at
most 30 seconds, before it captures its read use ([semantics.md](semantics.md)).
It never re-pins changed mounted material and never follows a configuration
upgrade: those stay with explicit `connections revalidate`.

A running owner of the CLI's own build admits `connections revalidate` from its
held metadata handle; only without one does the CLI admit it directly, which
verifies the whole store, before it starts an owner, as for `operations invoke`.
The command's 30-second deadline is set by the CLI as it sends the request to the
owner: after its own direct admission when it admitted the revalidation itself,
and before the owner's admission from its held handle otherwise, so that
admission counts against the deadline. Either way the store's size never
consumes it: the direct admission precedes it, and the owner's admission from
its held handle does not verify the whole store.

A revalidation that arrives while one of the same connection, at the same
expected semantic revision, is queued or running on the owner joins it instead
of queueing behind it: it runs no identity probe of its own, and answers the
in-flight revalidation's own answer (its connection summary, or its refusal)
as soon as that one answers. A joined revalidation's own deadline still bounds
its wait; at that deadline it is `outcome_unknown`, as an unanswered
revalidation is, and an in-flight revalidation whose own wait ended
`outcome_unknown` answers its joiners the same. One exception: a read's
recollection never re-pins changed mounted material, so when the one in flight
is a read's recollection that ends on changed mounted material, or ends without
an answer for its joiners, each joined revalidation runs its own, identity
probe included, within its own deadline. A revalidation at another expected
revision, or of another connection, never joins.

## Stopping an owner

`setup owner-stop` (`connectors.cli.SetupOwnerStopResult`) asks the running
owner of the selected state directory, of any build, to drain and exit. It is
the command the `stop_owner` next action names. It reads no configuration file,
starts no owner and creates nothing, and in this order:

1. A state directory that does not exist has no owner: the result is
   `disposition = no_owner`, exit 0.
2. A state directory, owner socket or `owner.lock` of another user is refused
   as `forbidden` (`stage = stop`, `next_action = check_configuration`).
3. A state directory with no `owner.lock`, or whose lifetime lock it can take
   (and releases at once), has no owner either: `disposition = no_owner`, exit 0.
4. Otherwise it greets the owner on its socket as any command does, the
   configuration path and the store's authority included, and sends the private
   shutdown request naming the incarnation that greeting returned. The owner
   stops its recovery and its supervisor, idle children included, answers and
   exits; the lifetime lock is released at process exit.
5. It waits for that lock to be free, and answers `disposition = stopped` with
   that `host_incarnation`.

The command is bounded by 30 seconds. A lock that stays held while
no owner answers on the socket within the bound is `unavailable`
(`stage = stop`, `next_action = retry_explicitly`); an owner that accepted the
request and still holds the lock at the bound is `timeout` (`stage = stop`,
`next_action = retry_explicitly`). Every other refusal keeps its own code, as
`readiness_mismatch` for an owner serving another configuration path. The
command never signals a process: the owner that exits is the one that proved,
over the owner-only socket of this state directory and with the user's own peer
credentials, that it serves this store.

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
  name is derived from the current user's SID, the state directory's
  canonical path and a random salt of 32 bytes kept, in hexadecimal, in the
  owner-only file `owner.pipe` in the state directory, so another local user
  cannot compute the name and create the pipe first. The first CLI or owner
  that finds no salt publishes one without replacing an existing file, and
  every later one reads it; a salt file that is not owner-only or not 64
  hexadecimal digits refuses as `invalid_configuration`. The pipe is created with `FILE_FLAG_FIRST_PIPE_INSTANCE` and an access
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
  once, and requires its token's SID to be the current user's, its integrity
  level to equal the owner's own and the token not to be restricted; the
  process handle opened then is the one every later handle duplication into
  that client goes through. A client of the current user that fails the
  integrity or restriction check is answered with a `lifecycle_conflict`
  failure before the owner closes the stream, never dropped without a reply.
  The CLI applies the same checks to the pipe's server process: an owner at
  another integrity level than the CLI, or with a restricted token, refuses as
  `lifecycle_conflict` (stage `admission`) before anything is sent, and no
  second owner is started.
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
  Adapter children are created inside a job object with
  `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` and no breakaway, through
  `PROC_THREAD_ATTRIBUTE_JOB_LIST`, whose only handle the owner holds, so
  they end when the owner does; no retained worker thread is needed for that.
  Exact stop terminates the owned child through its process handle, and waiting
  on that handle reports its exit.
- **Detaching from the CLI.** The CLI starts the owner with
  `CREATE_BREAKAWAY_FROM_JOB` and `DETACHED_PROCESS`, so the owner outlives the
  launching CLI. A CLI that runs inside a job that does not permit breakaway
  starts an owner that lives only as long as that job. The startup channel and
  the lifetime lock reach the starting owner as inherited handles, the only ones
  besides its standard handles in its `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`, in place of descriptors 3 and 4.
  Its environment holds the variable naming those handles, `SystemRoot`, and
  `TMP` and `TEMP` set to the state directory's `run` directory.
- **Scratch.** The owner's working files, the image copies and the
  credential's transient file, are made in the owner-only directory `run` in
  the state directory, never in the system's temporary directory, which for a
  process started with this environment would otherwise be the Windows
  directory. `TMP` and `TEMP` name it too, so the store's own temporary files
  are made there. The CLI creates it owner-only before it starts an owner, and
  the owner refuses one that is not owner-only. When an owner starts, it
  removes every image copy directory left in `run` whose copy it can delete,
  so one that a running process still holds stays.
