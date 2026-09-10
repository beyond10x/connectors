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

Private requests reuse the bounded three-section framing of the
[adapter binding](private-adapter.md), with distinct `connectors-owner/1` greetings.
The actual socket peer must be the configured UID. No credential, source path,
keyring tuple or continuation is part of ordinary control JSON. A capture admission
returns only the profile, acquisition reference and original 300-second expiry.
Its private acquisition handle stays with this live socket; reconnecting with a
public acquisition reference cannot complete it. Completion consumes it once,
validates the separately transferred document through the exact adapter, and then
uses the existing guarded custody/publication coordinator. Lost acknowledgement
is unknown and can be resolved only through status, without replay.

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
