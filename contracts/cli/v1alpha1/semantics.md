# Local CLI binding v1alpha1

**Specified; partial production implementation.** This contract owns the local
Linux CLI presentation, configuration and host lifecycle choices selected by
`story:local-cli-binding-semantics`. [ESS values](../../../ess/domains/cli.yaml)
and [fixtures](fixtures/values.json) describe data and expected traces. Neither a
generated parser nor a recording handler proves keyring durability, process
ownership, provider authentication or restart recovery. The existing runtime
continues to support the [documented three-adapter slice](../../../README.md#where-the-project-stands).
Setup, configured inventory and passive connection management have production
handlers; `setup checkpoints-enable` (§3) is specified ahead of its handler. The [private adapter transport](private-adapter.md) has process/TLS
fixtures. Protected GitLab acquisition and restart reuse pass disposable runtime
journeys; dedicated provider sandbox acceptance and the other adapters' persistent
lifecycle bindings remain open. [Approval-key management](../../service/approval-issuers.md)
adds the `approvals key-init`, `key-status`, `key-rotate`, `key-recover`, `key-revoke`
and `key-retire` commands. It does not issue approval proofs or enable provider writes.
The [local approval coordinator](../../service/local-mutations.md) adds policy
publication, exact subject preparation and protected issuance. Provider write
dispatch remains unfinished.

## 1. Owners and identities

The CLI parses, selects explicit local configuration, presents safe results and
passes admitted requests. Host configuration owns adapter selection and launch;
the lifecycle supervisor owns child incarnations; ConnectionAuthorityPort and
AcquisitionCoordinatorPort own connection metadata and publication. Linux OS
keyring custody owns sensitive versions. Adapter implementations own provider
profile/schema interpretation and credential validation. These are replaceable
bindings under the existing [management](../../auth/management.md),
[connection](../../auth/connection/v1alpha1/semantics.md),
[acquisition](../../auth/acquisition/v1alpha1/semantics.md) and
[custody](../../auth/custody/v1alpha1/semantics.md) contracts.

The `adapter` CLI selector is a configured entry alias, resolved to exactly one
`connectors.declarations.ServiceConfiguration.instance_id`; it is not an adapter
implementation identifier or executable path. `connection` is the existing
owner-qualified `connectors.auth_bindings.Connection.connection_ref`, and
`acquisition` names its existing acquisition owner/ref. A profile selects the
existing immutable AuthProfile under that adapter. Revisions retain the owning
record's meaning. No CLI entity, persistence ledger, grant, tenant, fake view or
domain command is introduced, except two owner-held records:
`connectors.cli.LocalRuntimeRecord` (stop suppression and the cached bootstrap,
§4) and `connectors.cli.ConnectionListCursor` (the opaque stale-page fence behind
`connections list` cursors, `registry_cursors` in `docs/local-er-metadata.md`).
Neither grants provider, connection or dispatch authority. Value observations
cannot establish authority.

The first local binding admits the current effective Linux UID as the configured
owner, verified again at a protected Unix-domain socket using kernel peer
credentials. Missing/mismatched owner policy refuses before protected entry or
provider work. File ownership alone is insufficient to grant business invocation:
configured operation policy and the current shared connection/evidence/grant
checks still apply. Local management admits only this owner and configured scope.
There is no cloud identity request, implicit tenant selector or saved hosted-login
fallback. Writes requiring an unavailable approval/idempotency binding refuse
before dispatch; this specification does not silently mark them read-only or
select a new write-approval format.

## 2. Command inventory and process contract

Every type below is in `connectors.cli`. The presentation binding is authored
separately as `ess-cli/1`. Process globals `config`, `state-dir` and `output` live
in its separate Context; none becomes a command payload field. Paths are resolved
before contacting an owner. Connect/repair inputs are ephemeral local protected
actions: their `credential_document: String` is acquired through the protected
source binding and passed only to trusted completion. Their distinct
`ConnectionCreateRequest` / `ConnectionRepairRequest` management projections
contain only safe selectors. An ephemeral protected action is never an ordinary
management wire payload or a persistent record.

| Command | Owner | Input → successful result | May start local host / selected adapter |
|---|---|---|---|
| `setup init` | local configuration writer | inputless Context → `SetupInitResult` | no / no |
| `setup check` | local configuration inspector | inputless Context → `SetupCheckResult` | no / no |
| `setup checkpoints-enable` | local metadata store, under the owner lifetime lock | `SetupCheckpointsEnableInput` → `SetupCheckpointsEnableResult` | no / no |
| `approvals clock-check` | configured clock inspector | `ApprovalClockCheckInput` → `ApprovalClockCheckResult` | no / no |
| `approvals policy-status` | local policy inspector | `ApprovalPolicyStatusInput` → `ApprovalPolicyResult` | no / no |
| `approvals policy-set` | local policy coordinator | `ApprovalPolicySetInput` → `ApprovalPolicyResult` | no / no |
| `approvals prepare` | local subject coordinator | `ApprovalPrepareInput` → `ApprovalPrepareResult` | no / no |
| `approvals issue` | local issuer coordinator | `ApprovalIssueInput` → `ApprovalIssueResult` | no / no |
| `adapters list` | configured inventory | `AdapterListInput` → `AdapterListResult` | no / no |
| `adapters describe` | configured inventory and selected cached metadata | `AdapterDescribeInput` → `AdapterDescribeResult` | no / no |
| `adapters status` | existing lifecycle owner observation | `AdapterStatusInput` → `AdapterStatusResult` | no / no |
| `adapters stop` | existing lifecycle owner | `AdapterStopInput` → `AdapterStopResult` | no / no |
| `connections list` | connection metadata authority | `ConnectionListInput` → `ConnectionListResult` | no / no |
| `connections describe` | connection metadata authority | `ConnectionDescribeInput` → `ConnectionDescribeResult` | no / no |
| `connections connect` | acquisition/configuration coordinator | `ConnectionConnectInput` → `ConnectionConnectResult` | yes / yes, after admission |
| `connections repair` | exact existing connection's coordinator | `ConnectionRepairInput` → `ConnectionRepairResult` | yes / yes, after admission |
| `connections revalidate` | exact existing connection's evidence coordinator | `ConnectionRevalidateInput` → `ConnectionRevalidateResult` | yes / yes, after admission |
| `connections launch` | exact existing connection's read admission and the owner's custody read | `ConnectionLaunchInput` → the consumer's exit status ([consumer launch](consumer-launch.md)) | yes / no |
| `connections status` | safe connection/acquisition observation | `ConnectionStatusInput` → `ConnectionStatusResult` | no / no |
| `connections revoke` | connection metadata authority | `ConnectionRevokeInput` → `ConnectionRevokeResult` | no / no |
| `operations list` | selected cached descriptor projection | `OperationListInput` → `OperationListResult` | no / no |
| `operations describe` | selected cached descriptor projection | `OperationDescribeInput` → `OperationDescribeResult` | no / no |
| `operations invoke` | admitted service request | `OperationInvokeInput` → `OperationInvokeResult` | yes / yes, after admission |

### Consumer launch

`connections launch` is the one command whose successful outcome is not a result
envelope. It starts an operator-pinned consumer from a `connectors-local/3`
`[consumers]` entry with the selected connection's protected document on
descriptor 3, passes the caller's stdin, stdout and stderr through, and exits with
the consumer's exit status. `ConnectionLaunchResult` is the handler seam's typed
value only; it is never written, because stdout belongs to the consumer. A refused
launch writes the ordinary `failure` envelope to stderr and starts no consumer. Its
argv is the entry's pinned prefix, then the optional `--args` JSON array verbatim;
the entry's `pass_env` prefixes select the caller's variables it gets. No adapter
starts; the owner alone reads the credential and hands the consumer a sealed memfd.
Admission order, delivery, process rules and the same-user boundary are in
[consumer launch](consumer-launch.md).

`approvals clock-check` performs the bounded authenticated exchange specified in
[the clock binding](../../service/clock.md). Its public observation cannot be
reused as time evidence; it starts no service and reads no secret or metadata.

`approvals policy-set` accepts only `--input-file`; its path is acquired by the
production handler using the bounded document reader. `prepare` and `issue` use
one of `--input-json`, `--input-file` or `--input-stdin` for nonsecret business input.
`issue` additionally requires `--approve-subject` and `--proof-output`. Their
authority, publication boundaries and helper limits belong to the
[local mutation binding](../../service/local-mutations.md). Proof bytes never
enter the ordinary successful CLI result.

Inventory and status commands use configured/cached descriptions and safe persisted metadata
or query an already-running owner without provider authentication. Unavailable
authoritative metadata returns `metadata_unavailable`; cached facts are labeled
stale and cannot be used to decide fresh connection viability. Missing cached
operation metadata returns `description_unavailable`, never implicit launch,
download, installation or authentication. Status distinguishes unavailable owner
from a known stopped process. An owner need not be running to commit local revoke
if the admitted metadata implementation supports the same serialized authority;
otherwise revoke returns `metadata_unavailable` and starts nothing.

`ConnectionListResult` carries page-level `source`, `stale`, `observed_at_ms`
and optional `valid_until_ms`, covering every returned summary. Any cached member
makes the page `source: cached, stale: true`; caching never preserves a fresh
label, even before a known validity deadline. An all-authoritative page may be
fresh only while its owner's observation remains valid for every member. Its
deadline is no later than the earliest member deadline. An absent or expired
deadline requires `stale: true`, including for an empty page without a known
page deadline. Configuration alone cannot establish connection state. These
labels preserve observation provenance; they grant no invocation or publication
authority. The same cached-is-stale rule covers adapter descriptions, operation
lists/descriptions and cached connection descriptions.
`adapters list` reports configured inventory with `source: configuration` only;
it carries no cached lifecycle or readiness claim. Adapter status and acquisition
status read their existing owner's observations or report owner unavailability;
they have no cached-result fallback. Connection status can carry a cached
`ConnectionDescription` only with its explicit cached/stale labels.

`setup init` and `setup check` explicitly bind `input: null` and an empty argument list in
`ess-cli/1`; the handler receives `{}` plus Context. ESS has no empty struct type,
so no dummy payload field or fictitious type is introduced for these actions.

`connections status` selects exactly one connection or acquisition. Connection
status uses a safe connection observation; acquisition status uses the acquisition
owner and stored repair relation. Both are metadata-only. Repair/revoke select
one exact connection and expected public semantic revision; that caller-visible
revision is not the private auth publication fence or material generation. The
coordinator captures and compares its own current private publication fence when
beginning and completing repair and serializes it with revoke/final dispatch. A
repeated body selector at
any management-envelope boundary must agree with the route selector. No implicit
business connection fills an instance-scoped create/list/acquisition-status input.

The public `AcquisitionObservation.state` is exactly `pending`, `completed` or
`failed`, following [acquisition §4.3](../../auth/acquisition/v1alpha1/semantics.md#43-acquisition-owner-consumption-and-terminal-records).
Internal Pending and Completing both project as `pending`; Completed projects as
`completed`; Failed and Expired project as `failed`. Failed observations carry
the safe `reason` from `connectors.auth_bindings.AcquisitionFailure`: `rejected`,
`exchange_unknown` or `expired`. Known expiry specifically carries `expired`.
Reason is present exactly when failed, and the yielded `connection` reference is
present exactly when completed. Pending/completed observations carry no failure
reason. These cross-field requirements are runtime obligations; Optional fields
and structural schema validation alone do not enforce them. Observation of a
consumed or failed acquisition never permits repeating its exchange.

`--output json` emits exactly one UTF-8 `{"ok":true,"result":...}` success envelope
plus newline to stdout. Failure leaves stdout empty and emits one
`{"ok":false,"error":{"code":...,"data":...}}` envelope plus newline to stderr;
application error data is `Failure`, while parser/source/internal errors have
stable codes and empty data as defined by `ess-cli/1`. Human mode identifies a
safe next action through the typed result/error. Progress belongs on stderr; JSON
mode suppresses ordinary progress so stderr is also machine-readable. Diagnostics
are fixed safe messages, never raw argv, file contents, keyring locators, provider
responses, actionable URLs or exception debug dumps. Secret-entry paths are
redacted too. Output framing is a codec obligation beyond the ESS value shape.
`completions SHELL` is outside this envelope: it prints the raw completion script
to stdout with exit 0 whatever `--output` selects. The `ess-cli/1` generator owns
that behaviour; this binding adds nothing to it.

| Exit | Meaning | Failure kind |
|---|---|---|
| 0 | successful command, including a truthful negative status observation | none |
| 2 | malformed arguments, conflicting sources, invalid config or business JSON/schema input | `usage` |
| 1 | admitted operation failed, unavailable dependency, stale target, refusal or unknown outcome | `operational` |
| 130 | interrupted command; no implicit retry | `interrupted` |

Parse errors use this same selected output contract, including unknown commands
and help/argument collisions. Failures the `ess-cli/1` presentation raises
before the handler runs carry its fixed codes with empty data, exit 2 and empty
stdout, never an application `Failure`:

| Condition | `ess-cli/1` code |
|---|---|
| unknown command or flag, missing or malformed argument, two sources for one field | `cli_parse` |
| an argument value that does not decode to its field type or the command's input shape | `cli_input` |
| a selected source the presentation cannot read, when the owner recorded no refusal of its own | `cli_source` |
| an `operations invoke` business JSON document that does not decode as exactly one JSON value (malformed, a duplicate object key, trailing document, 128 or more nesting levels counting the outermost value: the decoder's recursion limit, reached before the owner's depth bound) | `cli_dynamic_input` |

When the owner's reader refused the selected source, the owner's `Failure`
takes the place of `cli_source` (an unreadable business document file is
`invalid_input`; a missing safe entry channel is the table row below). A
decoded business document that the selected operation schema, its depth bound
(64) or its byte bound refuses is the owner's `invalid_input` `Failure`
(exit 2). `approvals prepare` and `approvals issue` hand their document to the
owner undecoded, so there an undecodable or trailing document is the owner's
`invalid_input` too, not `cli_dynamic_input`. Interruption stops waiting and protected capture,
restores terminal settings and erases transient capture buffers. It does not
claim rollback after a durable publication or possible provider dispatch. A
publication whose acknowledgement is lost is `outcome_unknown`; a later status
read may resolve it without replaying begin/exchange or the business request.

Compatibility `describe --endpoint --token-file [--allow-plaintext]` retains the
complete service descriptor, including configuration schema, instance, adapter,
version, revision and operations. It is not an alias for one operation's describe.
`LegacyDescribeInput` → `LegacyDescribeResult` models that complete result.
Compatibility `invoke --endpoint --token-file [--allow-plaintext] --operation
--input FILE` retains existing argument meanings and raw successful provider JSON
output (`LegacyInvokeInput` → `LegacyInvokeResult`). The carrier's internal field
is unwrapped when printing; it is not a new public wrapper. These legacy explicit
network routes are outside the no-launch/no-auth guarantee for local inspection.
The current `serve --config` runtime stays a separate federation command; this
contract does not claim its configuration starts the new local supervisor.

These three compatibility commands accept the process global `--output` before
or after the command word; it selects only the parse-refusal presentation, and
successful results stay raw JSON. `--config` and `--state-dir` are not accepted
before them (`serve --config` is that command's own argument). Their parse
errors, including unknown, missing or malformed arguments, follow the output
contract above: exit 2, empty stdout, and the fixed `cli_parse` code with empty
data on stderr, never an argv echo. `COMMAND --help` prints that command's usage.
Root `--help` or `-h`, alone or after leading process globals, prints the
generated grouped help (the short form of the long help
in `apps/connectors-cli-contract/help.txt`), one blank line and then this block,
which the generated help does not carry:

```text
Explicit service commands (use COMMAND --help for options):
  describe  Read a complete service descriptor
  invoke    Invoke an explicit service operation
  serve     Run the configured federation service
```

## 3. Explicit local TOML configuration

Selection order is explicit Context `--config`, then
`$XDG_CONFIG_HOME/connectors/config.toml` when that absolute, owner-controlled
directory is configured, otherwise `$HOME/.config/connectors/config.toml`. No
working-directory search, hosted profile, shell expansion inside TOML or
environment credential fallback occurs. Explicit Context `--state-dir` selects
the metadata directory; state otherwise defaults analogously below
`$XDG_STATE_HOME/connectors` or `$HOME/.local/state/connectors`. The configuration
path in safe results identifies the selection; it is not a credential locator.

`setup init` exclusively creates an owner-only file and directories with no
symlink traversal, writes/fsyncs a temporary file, publishes without replacing an
existing file, and fsyncs its directory before returning `created`. An existing
configuration is never overwritten. When the selected state directory holds no
metadata database and the existing configuration loads, `setup init` creates the
owner-only state directory and initialises its metadata database, leaves the
configuration byte-for-byte unchanged and returns `state_initialized`. An
existing configuration with an existing metadata database, or one that does not
load, returns `configuration_exists`; an existing database is never opened,
migrated or recreated by `setup init`. The metadata database `setup init`
creates, on `created` and `state_initialized` alike, has Entity Runtime durable
open checkpoints enabled before `setup init` returns, so its opens start from a
persisted checkpoint instead of verifying the whole history, and connectors
0.32.0 and earlier cannot open it. `setup init` never enables a database that
already exists; only `setup checkpoints-enable` does (below). The file uses
the current `connectors-local/2` format, whose adapter entries each select a
`private_protocol` ([private mutation extension](private-mutations.md)); an
existing `connectors-local/1` file keeps loading unchanged. A
`connectors-local/3` file is `/2` plus an optional `[consumers]` table of pinned
[consumer launch](consumer-launch.md) executables; `[consumers]` in a `/1` or `/2`
file refuses the configuration, and `setup init` keeps writing `/2`. It records the
local owner policy and OS keyring requirement; it does not write credentials,
authenticate, install artifacts or start processes. `setup check` checks syntax,
permissions, configured artifacts and non-interactive keyring availability. A
missing/locked keyring produces a failed prerequisite in its typed result; no
unlock prompt, write or successful connection is implied.

[The valid example](fixtures/config-valid.toml) chooses per-entry
`startup = "on-demand" | "automatic"`; omission means `on-demand`.
`restart = "never"` is the only first-stage policy. Configuration points at an
explicit absolute local executable with SHA-256 digest, expected service instance,
adapter identity and protocol version. Launch uses an argv vector, never a shell.
The executable must be an owner-admitted regular file and match the pinned digest
at launch. No discovered URL/path, PATH lookup, moving branch, download/build or
container selection is accepted by this first binding. Discovery is information.
Any future remote endpoint or installer is a separately admitted binding.

Duplicate aliases/instance IDs, unknown keys, relative paths, invalid modes,
reserved management-name collisions, bad digests and unresolved default selections
refuse the entire configuration before host launch. TOML contains no credential
bytes, keyring locators or actionable acquisition continuations. A default adapter
must name exactly one entry; connection defaults, if used by a future binding,
must be explicitly recorded and cannot be inferred from last usage. This first
binding requires an explicit connection for a credentialed operation.

### Durable open checkpoints on an existing store

`setup checkpoints-enable --confirm one-way` enables Entity Runtime durable open
checkpoints on the metadata database in the selected state directory. It is the
only way an existing store gets them: no other command, open or upgrade enables
them. Enabling installs Eventlog's triggers and continuity tables, and is one-way
for older releases: Entity Runtime 0.28.0 and earlier, and so connectors 0.32.0
and earlier, refuse to open the store afterwards. This binding has no command
that removes them. Every `SetupCheckpointsEnableResult` states this:
`change = one-way` and `newest_incompatible_release = "0.32.0"`. The command
starts no local host or adapter and runs in this order:

1. Without `--confirm one-way` it refuses `confirmation_required` (`kind = usage`,
   exit 2, `stage = arguments`, `next_action = retry_explicitly`) before it takes
   a lock or opens the store, and changes nothing. `--confirm` admits only the
   value `one-way`; any other value is the presentation's `cli_input`.
   `ess-cli/1` has no valueless switch, so the confirmation carries its value.
2. It takes the owner lifetime lock of [local owner transport](owner.md) without
   waiting and holds it until it returns, so no owner starts meanwhile. A lock
   another process holds, a running owner for this state directory, refuses
   `lifecycle_conflict` (`stage = admission`, `next_action = stop_owner`) with the
   store unchanged. The command never signals the owner; the user stops the
   running `__connectors-owner` process, as for `owner_build_mismatch`.
3. It opens the store as an admitted mutating open under `metadata.lock`, within
   the usual 30-second wait. As every mutating open does, it first migrates a
   level 1–8 database to level 9 ([local metadata](../../../docs/local-er-metadata.md#compatible-migration-from-levels-18)).
   It never creates a database. A missing, unreadable, foreign-owned,
   unrecognised or integrity-refused store, a `metadata.lock` not released within
   the wait, and Eventlog's refusal to enable (a trigger or continuity table the
   provider did not create) are `metadata_unavailable` (`stage = observation`,
   `next_action = retry_status`), with the store unchanged.
4. A store that already carries checkpoints answers `disposition = already_enabled`,
   a success with exit 0, and changes nothing. A second confirmed run answers it,
   and so does a run on a store `setup init` created.
5. Otherwise it enables them in one transaction, persists the open checkpoint at
   the head it verified, so the next open starts from that checkpoint, and answers
   `disposition = enabled`. A lost acknowledgement is `outcome_unknown`
   (`stage = publication`) with `next_action = retry_explicitly`: enabling an
   enabled store changes nothing, so running the command again answers
   `already_enabled` or enables.

The command sees only running owners. A second installed `connectors` binary or
another tool built on Entity Runtime 0.28.0 or earlier that is not running cannot
be detected; it fails to open the store once enabled. A tracked open from a
checkpoint does not detect raw edits of the database file that bypass SQLite
while no handle is open; a write through any SQLite connection still makes the
next open verify completely, and a read of an edited blob refuses it (Entity
Runtime 0.29.0).

## 4. Lifecycle and bounded startup

The executable owner binding is specified in [local owner transport](owner.md).
Each configured adapter has an optional `permissions` table with `profiles` and
`operations` lists of exact reviewed identifiers (at most 64 and 256 respectively).
Omission means an empty allowlist, never all discovered capabilities. These select
local-owner permission; profile/grant evidence, operation effects and native target
allowlists remain additional checks. The first binding admits reads only.

Optional top-level `secret_service_socket` selects an absolute, owner-controlled
local Unix socket instead of `/run/user/<uid>/bus`. It is an explicit infrastructure
binding, not a credential locator or business input. Parent directories, socket
ownership and kernel peer UID must be admitted; TCP and environment bus addresses
are never alternatives. The exact native service, encrypted storage and durable
acknowledgement qualification remain mandatory on either transport. This permits
dedicated local sessions and disposable acceptance infrastructure without weakening
credential custody or selecting another collection after a failure.

`automatic` means start when this local host starts. Pure list, configuration
inspection and status never start that host or its adapters. After local policy
and selected action admission, connect/repair/revalidate/invoke may start the host; its
automatic set and the selected on-demand entry become launch candidates. Provider
dispatch remains fenced on actual readiness and current connection admission.
Preflight admission is not a reusable grant across startup.

The host coalesces simultaneous startup by configured instance identity and exact
configuration revision. One process incarnation owns that launch; other callers
wait for the same result within their own original deadline. They do not spawn
duplicates, reset budgets or inherit another caller's authority. At most one
configured incarnation is live; changing artifact/configuration while one is
active refuses `lifecycle_conflict` until exact stop/drain completes.

Readiness requires descriptor/bootstrap agreement with the configured instance,
adapter identity, protocol and expected configuration revision. A mismatch is
`readiness_mismatch`, never ready. The supervisor terminates only the child it
just launched, records a safe failure and isolates it. Failed automatic entry A
does not prevent independent ready entry B from serving. A request for A receives
A's failure; it cannot silently choose B. Failure of the host itself fails all
waiting requests without a second competing host launch.

Stop requires configured alias, current host incarnation and current child
incarnation, plus the expected configuration revision. The host compares all
coordinates atomically against its owned process handle; a stale target returns
`incarnation_mismatch` without signaling anything. Linux pidfd or equivalent
non-reusable kernel process ownership is required; a PID/path from caller input
or persisted cache alone is not sufficient. Graceful drain is bounded, then the
owner may terminate only that exact owned child and its registered owned children.
A remote, foreign or already-replaced process cannot be killed by this command.

An explicit stop sets a durable suppression marker for the configured instance
in the existing configuration owner's state; neither a later automatic sweep,
crash nor host restart clears it. An admitted later connect/repair/revalidate/invoke directed
explicitly at that entry is the selected resume action, clears suppression under
current authority and requests one fresh launch. An unrelated command never
resumes it. `restart = "never"` also forbids background crash-restart loops;
observed unexpected termination is `failed`, and a later explicit admitted request
may attempt one new launch. Config changes do not silently clear stop suppression.
The suppression marker is the `suppressed` field of the configuration owner's
`local_runtime_instances` row, modeled as `connectors.cli.LocalRuntimeRecord`
(`ess/domains/cli.yaml`, mapped in `docs/local-er-metadata.md`); the resume and
restart rules above remain owner obligations.

Bounds for this first binding are explicit, selected limits, not measured runtime
performance: 64 configured entries; four concurrent launches; one live launch per
instance; 10 seconds for readiness and 5 seconds for graceful stop; 1 MiB TOML;
100 default/500 maximum list items per page; 128 UTF-8 bytes per selector; 4 KiB
per path; 64 KiB protected credential document; 1 MiB business input; 8 MiB result;
JSON depth 64; 30-second default/120-second maximum business deadline. Provider and
descriptor limits may only lower these bounds. List cursors are opaque, admitted,
bound to the selection/revision and expire after 300 seconds; stale cursors refuse.
No automatic paging or unbounded retry occurs. Entry capture expires after 300
seconds. OAuth continuations/registration and browser callback ingress remain
separately specified; they are not secretly implemented by this terminal profile.

## 5. Protected entry and durable publication

The Linux OS keyring binding selects the user's admitted Secret Service collection
through authenticated local session transport. It must satisfy custody
`write_new/read/delete` with immutable version identity, scoped reads and definite
durable acknowledgement. A transient session cache or a successful method return
without that durability guarantee is insufficient. Collection lock/absence/write
failure returns `custody_unavailable`; plaintext TOML, environment variables or
ordinary local files are never fallback custody. Keyring locations remain private
inside the binding. Secret Service's actual durability/ownership behavior remains
a required backend conformance check before this binding is advertised.

Exactly one protected entry source is selected for connect/repair: hidden terminal,
protected file or stdin, each explicitly selected by its own source flag. Hidden
terminal requires a trusted controlling TTY; unavailable or conflicting sources
refuse as a usage/source error and explain safe source selection. No password/token
argv flag, environment
source, generic business input, or automatic read from a different descriptor is
accepted. These inputs select a reviewed static-entry profile; unsupported managed
flows refuse before acquiring a usable flow. `static_config` uses its existing
configuration activation path, never an invented auth.begin exchange.

Terminal capture uses the controlling terminal, verifies the local owner's session,
disables echo before reading, restores settings on success/failure/interruption,
and bounds capture before allocation. Prompts go to that protected terminal,
never JSON stdout/stderr. File capture opens an absolute regular file without
following symlinks in any component, verifies owner UID, single link and no group/
other permission bits on the opened descriptor, and enforces safe parent traversal.
The source is read once from that descriptor, bounded before parsing; it is not
reopened after verification. Read-only 0400 and read/write 0600 are permitted.

Protected stdin accepts either such an already-open protected regular file or an
inherited anonymous pipe held within the admitted local UID/session boundary.
Sockets, named FIFOs, terminals on the stdin mode, unknown ownership, unexpected
descriptor types and broad permissions refuse. A pipe's writer must be supplied
deliberately by that trusted launcher/session; the CLI does not claim fstat alone
authenticates arbitrary producer behavior. Same-UID debugger/ptrace compromise is
outside this process-isolation promise. Named file/stdin credentials use one UTF-8
JSON document with exactly the reviewed profile's fields; duplicate keys, trailing
documents, invalid UTF-8, depth/size excess and unknown fields refuse. A final JSON
whitespace newline is allowed; secret string contents are never trimmed. Terminal
prompts construct the same protected profile document privately. The CLI input
model carries bytes only in the explicitly ephemeral local protected-action field;
its safe management projection omits that field. Protected bytes bypass ordinary
recording evidence, public serialization, tracing, telemetry, history and error
reports. Fictional sentinel capture in a bounded parser test is test evidence only.

Business file/stdin JSON is a different channel. If both channels claim stdin in
one invocation, source admission fails before either is consumed. File paths,
parse snippets and secret sentinel values must be absent from every ordinary
success/failure/progress/help output, including early parser failures. Clear
transient buffers promptly; do not promise impossible erasure of external keyring,
OS or caller buffers.

Success requires, in order: current management admission and exact target; private
profile validation and baseline external identity evidence; a definite durable
keyring write of an immutable candidate; current identity/revision/non-revocation
checks; atomic durable metadata publication linking that exact stored version;
then a safe successful connection reference. An unknown keyring write cannot
become acknowledged custody. A crash after custody and before publication leaves
an unpublished candidate for guarded cleanup/reconciliation, never a usable
connection. An unknown metadata commit yields `outcome_unknown`; status resolves
the existing acquisition/connection under the original owner without recapturing
or repeating a potentially consumed exchange. The shared acquisition/refresh
fencing and retention rules remain authoritative.

Once published, a still-valid generation is reusable after both CLI and local
owner restart without re-entry, hosted login or an unnecessary provider exchange.
The restarted owner loads authoritative metadata and reads the exact retained
custody version, then checks current expiry/revocation/evidence at invocation.
Missing material or a locked collection is an explicit failure, never another
generation or environment credential. Persisted metadata without valid custody
is not a successful connection.

Repair binds the same connection identity, immutable profile/owner/target and
expected revision. Same-principal/same-target credential renewal may publish a new
private generation and advance its private publication fence while preserving
both connection reference and public semantic revision. Only separately admitted
effect-relevant configuration/management changes advance that semantic revision
under the shared fixed-binding rules; material renewal or evidence recollection
alone does not. Different
principal, account or provider target returns `identity_mismatch`; creating a new
connection is the explicit replacement action. A failed or uncertain candidate
repair does not replace or globally invalidate a still-valid active generation.
Expired/revoked old generations do not become valid merely because repair failed.

Local revoke is terminal. It serializes with publication and final dispatch,
definitely commits the cutoff before reporting success and survives CLI/owner
restart, configuration reload and static reactivation. Pending repairs cannot
publish across it. The returned provider outcome is `not_requested`; this first
binding exposes no provider-revoke flag. Keyring deletion is separately guarded
retirement, not a prerequisite for local cutoff and not implied by it. Repeated
revoke observes the terminal record without repeating provider work. Previously
dispatched effects are not undone. Lost acknowledgement remains unknown until an
authoritative read; no observation of missing sockets permits resurrection.

## 6. Dynamic operation input and safe failure mapping

`OperationInvokeInput.input` is a String carrying one bounded UTF-8 JSON document.
It is the exception needed for runtime-selected provider schemas, not a
universal input/result property bag. A file/stdin presentation binding reads text
once; the provider JSON decoder then rejects malformed/duplicate/trailing input and
validates the decoded JSON value against the exact selected operation schema.
Carrier validation and provider-schema validation establish separate facts. When
serializing the service request, `input` is that decoded JSON value, never the
quoted carrier string or a double-encoded document. Answers are not carriers:
`OperationInvokeResult.result` is the provider result as a JSON value, validated
against the selected operation's output schema, and `OperationDescription`
answers `input_schema` and `output_schema` as JSON schema objects in
`operations describe`, in the `adapters describe` descriptor and in the
compatibility `describe`. A caller decodes the answer once; none of these fields
is a string holding JSON. The schema digest (`schema`), not the schema document,
remains the identity an invocation selects. `ServiceDescriptor.configuration_schema`
remains a JSON text carrier. Structured command metadata uses specific types, not
a catch-all string.

Invocation selects adapter, operation, exact schema identity (`schema`), description
revision (`revision`) and explicit
connection where required. Refreshed readiness/descriptor information that does
not match the selected revision returns `stale_description` before provider
dispatch; it never silently substitutes a new schema or retries the request.
Operation absence, malformed field/type, missing permission and unreachable
service remain separate errors. An operation id the adapter does not expose
answers `not_found` before any grant check, so absence is never reported as
`forbidden`. Bound input before parsing, validate current
authority immediately before dispatch, preserve shared mutation uncertainty and
never replay a possible write after interruption, timeout or lost response.

| Condition | `Failure.code` | Exit |
|---|---|---|
| decoded business document the operation schema, depth or byte bound refuses; `approvals prepare` or `approvals issue` business document that does not decode or has a trailing document; unreadable business document file; argument value the handler refuses | `invalid_input` | 2 |
| TOML syntax, unsupported startup/restart, duplicate identity, bad permissions | `invalid_configuration` | 2 |
| configuration already exists (`setup init`: with a metadata database, or one that does not load) | `configuration_exists` | 1 |
| no safe entry channel (in place of the presentation's `cli_source`) | `protected_entry_unavailable` | 2 |
| `setup checkpoints-enable` without `--confirm one-way`; nothing changed (`stage = arguments`, `next_action = retry_explicitly`) | `confirmation_required` | 2 |
| keyring unavailable or durable storage not acknowledged | `custody_unavailable` | 1 |
| metadata authority unavailable | `metadata_unavailable` | 1 |
| a non-guarded metadata write the store changed under (a concurrent commit); nothing was written (`stage = publication`, `next_action = retry_explicitly`). A guarded write keeps the guarded-write rule below: its mutation record, `next_action = retry_status` | `revision_conflict` | 1 |
| metadata publication acknowledgement uncertain | `outcome_unknown` | 1 |
| changed principal/account/target during repair | `identity_mismatch` | 1 |
| stale revision, occupied changed configuration; `setup checkpoints-enable` while an owner runs for the state directory (`stage = admission`, `next_action = stop_owner`) | `lifecycle_conflict` | 1 |
| stale/foreign stop target | `incarnation_mismatch` | 1 |
| wrong readiness identity/version/revision | `readiness_mismatch` | 1 |
| running owner is a different executable build (`next_action = stop_owner`) | `owner_build_mismatch` | 1 |
| no selected cached description | `description_unavailable` | 1 |
| schema/descriptor changed | `stale_description` | 1 |
| absent selected operation/connection | `not_found` | 1 |
| stale, expired or foreign list cursor (`stage = observation`, `next_action = retry_explicitly`) | `stale_cursor` | 1 |
| connect whose acquisition failed before publication (`stage = dispatch`, `next_action = retry_explicitly`) | the acquisition owner's code | 1 |
| known revoked connection | `revoked` | 1 |
| current permission missing | `forbidden` (or shared `not_granted`) | 1 |
| unavailable service / budget expired | `unavailable` / `timeout` | 1 |
| user interruption | `interrupted` | 130 |

A configuration whose adapter entry contradicts its format — a
`connectors-local/1` entry that carries `private_protocol`, or a
`connectors-local/2` or `/3` entry that lacks it — is `invalid_configuration` with
`next_action = check_configuration`, and its `Failure` additionally carries
`configuration_format` (the file's `format` value) and `instance_id` (the
entry's `instance_id`). They are named only after every entry has passed every
other check, so the refusal never depends on alias order: the first mismatched
entry in alias order is named, and a file with any other defect names nothing.
`configuration_format` is the closed enum `ConfigurationFormat`; `instance_id`
is an admitted selector (at most 128 bytes of `[A-Za-z0-9._-]`). Every CLI
command except `setup init` refuses such a file this way, before acquiring any
protected source or dispatching. The selector shape and the rule that both
fields appear only on `invalid_configuration` are host obligations: ESS 0.40
declares no invariant on a CLI type. No parser, OS or database text is ever
carried.

Other admitted provider/service failures preserve their existing shared error code
inside `Failure.service_code`; `code = service_failure` identifies that route.
Provider strings never bypass safe message projection. Failure has a safe stage
and next-action enum, plus optional opaque correlation refs under current result
access; it never contains raw provider evidence or custody references.

`Failure.service_reason` is the one provider text a failure may carry, and only
in this admitted form. It appears on `operations invoke` of a read, at
`stage = dispatch`, when the provider refused the dispatched request: beside
`service_code`, or on the provider's own `forbidden`. It is absent otherwise,
and absent unless every rule below holds:

- **Source.** The provider adapter read it from the refusal's response body, and
  only from the first of the top-level `message`, `error_description` and
  `error` members of a JSON object body that holds a string. A header, a nested
  member (`error.message`), an array, any other member and a body that is not a
  JSON object are never read. Today only the catalog provider supplies one.
- **Shape.** Whitespace runs are folded to one space and the ends trimmed. The
  reason is withheld when it is empty, or when its first 4096 bytes hold a
  control character (Unicode Cc), any format character (Unicode Cf, as of
  Unicode 16.0: U+00AD, U+0600–U+0605, U+061C, U+06DD, U+070F, U+0890–U+0891,
  U+08E2, U+180E, U+200B–U+200F, U+202A–U+202E, U+2060–U+2064, U+2066–U+206F,
  U+FEFF, U+FFF9–U+FFFB, U+110BD, U+110CD, U+13430–U+1343F, U+1BCA0–U+1BCA3,
  U+1D173–U+1D17A and the TAG block U+E0000–U+E007F), or another character that
  renders as nothing and can carry hidden data (U+034F, U+115F–U+1160, U+3164,
  U+FFA0, and the variation selectors U+FE00–U+FE0F and U+E0100–U+E01EF).
- **No personal data or secret shape.** The reason is withheld whole, never
  partly masked, when its first 4096 bytes hold an `@` (or U+FF20, U+FE6B), so
  no email address is ever carried; or, without regard to ASCII case, a header-
  or credential-like marker (`authorization`, `bearer`, `basic `, `password`,
  `passwd`, `secret`, `cookie`, `api_key`, `apikey`, `api-key`, `private_key`,
  `private key`, `-----begin`, `token=`, `token:`, `access_token`,
  `refresh_token`, `id_token`, `client_assertion`, `signature=`, `sig=`,
  `session=`); or a token-like run: a maximal run of ASCII letters, digits and
  `-_+/=.~` that is at least 32 bytes long, or at least 20 bytes long and holds
  both a letter and a digit; or a token-like word: a space-separated word of at
  least 16 bytes that holds a digit, or whose ASCII letters change case at
  least once per four letters. UUIDs, long numbers and request identifiers are
  withheld by these rules too.
- **Not the request's credential.** The adapter child withholds a reason that
  holds any piece of the credential value it derived and sent (an HTTP basic
  header value, an OAuth access token), and the host admits the reason again
  after the child returns it, withholding it when it holds any piece of any
  string value of the protected document the call carried (the whole document,
  when it is not JSON or holds no string). A piece is eight consecutive bytes
  compared without regard to ASCII case, or eight consecutive ASCII letters and
  digits compared lowercased with every other character removed from both
  sides, so an echo with changed separators matches; a value shorter than
  eight counts whole. The reason is also read with its `%XX` escapes decoded.
- **Bound.** It is at most 256 bytes of UTF-8. A longer reason is cut on a
  character boundary and then back to its last whole space-separated word; one
  whose first word does not fit is withheld.

The reason is text for a person reading why the provider refused, such as the
missing token scope behind an `unauthorized`. It never decides `code`, `stage`
or `next_action`, and a client must not parse it. A guarded write's failure, a
connection probe's answer and every other route carry none.

`Failure.retry_after_seconds` is the delay a provider named before a
rate-limited read may be sent again, so the caller can wait. It appears on
`operations invoke` of a read, at `stage = dispatch`, only beside
`service_code = rate_limited`, and only when the provider's `429` carried a
`Retry-After` the adapter could read: delta-seconds, or an HTTP-date counted
from when the answer arrived and rounded up to a whole second (a date already
past is `0`). It is a whole number of seconds from 0 to 4294967295; a value
outside that range, or in any other form, is not read: the adapter does not
read it, and the host drops one an adapter reports, keeping the failure. Two
`Retry-After` field lines on one answer that disagree name no delay; identical
repeats count as one. Its absence beside `rate_limited` states that the
provider named no delay the adapter could read.
Before refusing, the catalog provider sends such a read once more after the
named delay, never more than once, and only when the wait, then a second
request as long as the first one took, then a 500 ms margin, all end before
the invocation's deadline. The second request is bounded by the deadline less
that margin; if it does not finish, the first answer's refusal and delay
stand. A failure carrying `retry_after_seconds` therefore names a delay that
did not leave such time, a delay whose second request did not finish, or the
delay the second `429` named. A guarded write is never sent again and carries
none. The field never decides `code`, `stage` or
`next_action`.

The stage names who refused, because the same code can come from either side.
A refusal made before any provider request — by the host's admission (an
unknown adapter alias included), or by an adapter from its own configured
scope (a namespace, resource kind or operation the adapter entry does not
allow) — reports `stage = admission` (`forbidden` with
`next_action = request_permission`, `not_found` with `check_configuration`).
`operations invoke`, `approvals prepare`, `approvals issue` and
`approvals policy-set` read their business document before the adapter alias is
admitted, so an unreadable or undecodable document is refused first (the
document refusals above) and an unknown alias is `not_found` only once the
document has been read. Every other command answers an unknown alias with
`not_found` at admission.
Only the provider's own answer to a dispatched call
reports `stage = dispatch`: an upstream forbidden answer (HTTP 403) keeps
`code = forbidden` with `next_action = request_permission`, and an upstream
not-found answer (HTTP 404; the catalog provider also 410) is
`code = service_failure`, `service_code = not_found` with
`next_action = none`. Neither sends the operator to connectors configuration.
A stored credential the provider refuses on `operations invoke` of a read on an
existing connection — an HTTP 401, or an OAuth token endpoint's `invalid_grant` or
`invalid_client` for a refresh profile — is `code = service_failure`,
`service_code = unauthorized` at `stage = dispatch` with
`next_action = repair_connection`: a retry with the same protected entry cannot
succeed, and a repair replaces it. A credential refused while a connection is
made (`connections connect`, `connections repair`) or revalidated is not this
case and keeps `next_action = retry_explicitly`.
A guarded write refused for its stored credential, at its preflight or its
commit, is not this case either: it reports its `mutation` record with
`next_action = retry_status` until
`story:guarded-write-credential-refusal-says-repair` extends this rule to writes.
A connection probe's answer (the catalog provider's identity and scope probes,
the Kubernetes token review) is classified separately and not by this rule: a
403 reads as `forbidden` at `admission`, and a 404 is `service_failure` with
`service_code = upstream_protocol` at `dispatch` and
`next_action = retry_explicitly`.

A read on `operations invoke` refused before dispatch only because the
connection's validation evidence expired, while its credential is intact (the
connection reports `pending`), is `code = not_granted` at `stage = admission`
with `next_action = revalidate_connection`. `connections revalidate`
recollects the evidence from the credential already in custody, with no
re-entry, and the same invoke is then admitted. The invoke never revalidates
on its own: a business read runs no implicit identity probe. A connection whose
credential expired or is known invalid (`reauthorization_required`), and a
credential below the operation's scopes, keep `code = not_granted` with
`next_action = repair_connection`.

A provider's timeout or capacity answer is the provider's too: a request the
provider transport sent whose deadline then passed (a connection probe's
included, and one that passes while the provider's answer is still being
read after its status and headers arrived), a database statement timeout, and a capacity refusal the adapter
marks as the upstream's answer (the SQL adapter's database connection limit)
report `timeout` / `capacity` at `stage = dispatch` with
`next_action = retry_explicitly`. A connection that never opened sent nothing
and is not one of them. The host's own deadline and limits, and an adapter's
own connect or work deadline and result bounds, keep `stage = admission`
(`retry_explicitly`). An HTTP 429 stays `service_failure` with
`service_code = rate_limited`, and with `retry_after_seconds` when the provider
named a delay. `unsupported` is always `admission`: every
source of it is raised before dispatch.

A guarded write's failure carries its `mutation` record, and its stage follows
who refused and whether the write was attempted. A refusal the host made
before anything was sent (`mutation.classification = not_attempted` — the
host's approval refusals and its own deadline included) reports
`stage = admission`. The provider's answer, and a host failure once the write
was sent or may have been (`refused`, `applied`, `unknown`), report
`stage = dispatch`. The provider's own refusal of the write names its next
action as a read's does: an upstream 403 (the catalog provider also 405 and
415) is `forbidden` with `next_action = request_permission`, an upstream 404 or
410 is `service_failure`, `service_code = not_found` with
`next_action = none`, and a provider timeout or capacity answer to the
write's preflight is `retry_explicitly`. A write classified `applied` or
`unknown` is never offered `retry_explicitly`: it may have taken effect.
Every other write failure — the host's approval refusals included — reports
`next_action = retry_status`. A write whose effect is unknown after its
request was sent, including by a timeout, stays `outcome_unknown`. The settled attempt stores which side refused, so a replay
of the same idempotency key reports the same stage and next action as the
original reply; an attempt stored before the origin was recorded replays as
the host's (`retry_status`).

## 7. Verification and remaining obligations

[Scenarios](scenarios.md) enumerate C01–C05, startup races and failure boundaries.
[Value fixtures](fixtures/values.json) carry explicit valid/invalid expectations
against generated schemas. Their structural validation proves required fields,
types, enums and closed objects, not cross-field bounds or any process trace.
[Cached fixture expectations](fixtures/cached-expectations.json) retain the two
specific cached operation regressions. The fixture gate must additionally inspect
every object in every structurally valid example, require `stale: true` whenever
`source: cached`, and check the public acquisition state/reason/connection
combinations and connection-list page freshness labels. Selections must be
nonempty, fixture ids unique, and each named expectation must match exactly one
fixture. These are consistency checks on authored examples, not evidence that a
runtime enforces the same rules.
TOML fixtures and trace expectations are reviewed specification inputs; they are
not a pretend running backend. The dependent CLI surface story owns generated
parser/process/source fixtures and the integrated gate.

Runtime acceptance must exercise real Linux ownership/no-symlink descriptor checks,
echo restoration and output redaction before parsing failures; keyring lock/write/
durability and crash recovery; metadata publication/revoke/dispatch atomicity;
same-target repair and revision races; child launch coalescing and pidfd stop;
durable stop suppression; actual readiness/timeout/isolation; still-valid
credential reuse across both process restarts; and durable open checkpoints: a
store `setup init` creates carries them, an existing store `setup checkpoints-enable`
enabled then opens from its checkpoint, and an unconfirmed run is refused with
the store unchanged. New ESS and deterministic output
must be revalidated using the selected exact current-source ESS toolchain. An
older release binary is provisional authoring evidence only.

## Operations by family

`operations list --family CONTRACT` answers only the listed operations whose
descriptor `contract` is exactly `CONTRACT`, each with its `id`, `contract` and
native `profile`. It is how a consumer finds a shared family on a saved connection
without knowledge of the provider: a connection is selected by the `--adapter`
alias it was made under (§1), and the operations it can be invoked with are that
adapter's cached description, so the consumer lists them with the same alias, then
describes and invokes the family's fixed operation ids. For
[datasource.feed/v1alpha1](../../datasources/feed/v1alpha1/semantics.md) these
are `feed.containers` and `feed.items` under one profile. An adapter added or
upgraded after the consumer was built is found the same way once its description
is cached.

The filter narrows the same selection `operations list` already answers: an
operation the configuration does not grant stays hidden, and the cursor is bound
to the family as well as the adapter, revision and granted operations, so a
cursor issued under one filter refuses as `stale_cursor` under another. A family
no listed operation binds answers an empty page, a truthful negative observation,
not `not_found`. The value is matched exactly; one that is empty, longer than 128
bytes or holds a character other than ASCII letters, digits, `-`, `_`, `.` and
`/` refuses as `invalid_input` before the cached description is read.
