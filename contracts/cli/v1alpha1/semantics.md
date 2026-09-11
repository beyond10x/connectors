# Local CLI binding v1alpha1

**Specified; partial production implementation.** This contract owns the local
Linux CLI presentation, configuration and host lifecycle choices selected by
`story:local-cli-binding-semantics`. [ESS values](../../../ess/domains/cli.yaml)
and [fixtures](fixtures/values.json) describe data and expected traces. Neither a
generated parser nor a recording handler proves keyring durability, process
ownership, provider authentication or restart recovery. The existing runtime
continues to support the [documented three-adapter slice](../../../README.md#where-the-project-stands).
Setup, configured inventory and passive connection management have production
handlers. The [private adapter transport](private-adapter.md) has process/TLS
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
domain command is introduced. Value observations cannot establish authority.

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
| `connections status` | safe connection/acquisition observation | `ConnectionStatusInput` → `ConnectionStatusResult` | no / no |
| `connections revoke` | connection metadata authority | `ConnectionRevokeInput` → `ConnectionRevokeResult` | no / no |
| `operations list` | selected cached descriptor projection | `OperationListInput` → `OperationListResult` | no / no |
| `operations describe` | selected cached descriptor projection | `OperationDescribeInput` → `OperationDescribeResult` | no / no |
| `operations invoke` | admitted service request | `OperationInvokeInput` → `OperationInvokeResult` | yes / yes, after admission |

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

Both setup actions explicitly bind `input: null` and an empty argument list in
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

| Exit | Meaning | Failure kind |
|---|---|---|
| 0 | successful command, including a truthful negative status observation | none |
| 2 | malformed arguments, conflicting sources, invalid config or business JSON/schema input | `usage` |
| 1 | admitted operation failed, unavailable dependency, stale target, refusal or unknown outcome | `operational` |
| 130 | interrupted command; no implicit retry | `interrupted` |

Parse errors use this same selected output contract, including unknown commands
and help/argument collisions. Interruption stops waiting and protected capture,
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
existing file, and fsyncs its directory before returning `created`. Existing
configuration returns `configuration_exists`, never overwrites it. It records the
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
These are owner obligations, not a new CLI persistence entity.

Bounds for this first binding are explicit, selected limits, not measured runtime
performance: 64 configured entries; four concurrent launches; one live launch per
instance; 10 seconds for readiness and 5 seconds for graceful stop; 1 MiB TOML;
100 default/500 maximum list items per page; 256 UTF-8 bytes per selector; 4 KiB
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
quoted carrier string or a double-encoded document. Results and schema documents
use the same explicit carrier distinction. Structured command metadata uses
specific types, not a catch-all string.

Invocation selects adapter, operation, exact schema identity (`schema`), description
revision (`revision`) and explicit
connection where required. Refreshed readiness/descriptor information that does
not match the selected revision returns `stale_description` before provider
dispatch; it never silently substitutes a new schema or retries the request.
Operation absence, malformed field/type, missing permission and unreachable
service remain separate errors. Bound input before parsing, validate current
authority immediately before dispatch, preserve shared mutation uncertainty and
never replay a possible write after interruption, timeout or lost response.

| Condition | `Failure.code` | Exit |
|---|---|---|
| unknown flag/command, wrong field/type, secret source collision | `invalid_input` | 2 |
| TOML syntax, unsupported startup/restart, duplicate identity, bad permissions | `invalid_configuration` | 2 |
| configuration already exists | `configuration_exists` | 1 |
| no safe entry channel | safe source error (`protected_entry_unavailable` in application data) | 2 |
| keyring unavailable or durable storage not acknowledged | `custody_unavailable` | 1 |
| metadata authority unavailable | `metadata_unavailable` | 1 |
| metadata publication acknowledgement uncertain | `outcome_unknown` | 1 |
| changed principal/account/target during repair | `identity_mismatch` | 1 |
| stale revision, occupied changed configuration | `lifecycle_conflict` | 1 |
| stale/foreign stop target | `incarnation_mismatch` | 1 |
| wrong readiness identity/version/revision | `readiness_mismatch` | 1 |
| no selected cached description | `description_unavailable` | 1 |
| schema/descriptor changed | `stale_description` | 1 |
| absent selected operation/connection | `not_found` | 1 |
| known revoked connection | `revoked` | 1 |
| current permission missing | `forbidden` (or shared `not_granted`) | 1 |
| unavailable service / budget expired | `unavailable` / `timeout` | 1 |
| user interruption | `interrupted` | 130 |

Other admitted provider/service failures preserve their existing shared error code
inside `Failure.service_code`; `code = service_failure` identifies that route.
Provider strings never bypass safe message projection. Failure has a safe stage
and next-action enum, plus optional opaque correlation refs under current result
access; it never contains raw provider evidence or custody references.

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
durable stop suppression; actual readiness/timeout/isolation; and still-valid
credential reuse across both process restarts. New ESS and deterministic output
must be revalidated using the selected exact current-source ESS toolchain. An
older release binary is provisional authoring evidence only.
