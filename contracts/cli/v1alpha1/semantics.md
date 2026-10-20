# Local CLI binding v1alpha1

**Specified; partial production implementation.** This contract owns the local
Linux CLI presentation, configuration and host lifecycle choices selected by
`story:local-cli-binding-semantics`. [ESS values](../../../ess/domains/cli.yaml)
and [fixtures](fixtures/values.json) describe data and expected traces. Neither a
generated parser nor a recording handler proves keyring durability, process
ownership, provider authentication or restart recovery. The existing runtime
continues to support the [documented three-adapter slice](../../../README.md#where-the-project-stands).
Setup, configured inventory and passive connection management have production
handlers, `setup checkpoints-enable` (§3) included. The [private adapter transport](private-adapter.md) has process/TLS
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

The `connector` and `instance` CLI selectors name one configured instance
together, and the deprecated `adapter` selector names a configured entry alias;
each resolves to exactly one
`connectors.declarations.ServiceConfiguration.instance_id`
([selecting an instance](#selecting-an-instance)). Neither is an adapter
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
before contacting an owner. Connect/reconnect inputs are ephemeral local protected
actions: their `credential_document: String` is acquired through the protected
source binding and passed only to trusted completion. Their distinct
`ConnectionCreateRequest` / `ConnectionReconnectRequest` management projections
contain only safe selectors. An ephemeral protected action is never an ordinary
management wire payload or a persistent record.

| Command | Owner | Input → successful result | May start local host / selected adapter |
|---|---|---|---|
| `setup init` | local configuration writer | inputless Context → `SetupInitResult` | no / no |
| `setup check` | local configuration inspector | inputless Context → `SetupCheckResult` | no / no |
| `setup checkpoints-enable` | local metadata store, under the owner lifetime lock | `SetupCheckpointsEnableInput` → `SetupCheckpointsEnableResult` | no / no |
| `setup owner-stop` | the running owner of the state directory, through its shutdown request ([owner transport](owner.md#stopping-an-owner)) | inputless Context → `SetupOwnerStopResult` | no / no |
| `approvals clock-check` | configured clock inspector | `ApprovalClockCheckInput` → `ApprovalClockCheckResult` | no / no |
| `approvals policy-status` | local policy inspector | `ApprovalPolicyStatusInput` → `ApprovalPolicyResult` | no / no |
| `approvals policy-set` | local policy coordinator | `ApprovalPolicySetInput` → `ApprovalPolicyResult` | no / no |
| `approvals prepare` | local subject coordinator | `ApprovalPrepareInput` → `ApprovalPrepareResult` | no / no |
| `approvals issue` | local issuer coordinator | `ApprovalIssueInput` → `ApprovalIssueResult` | no / no |
| `instances list` | configured inventory | `AdapterListInput` → `AdapterListResult` | no / no |
| `instances describe` | configured inventory and selected cached metadata | `AdapterDescribeInput` → `AdapterDescribeResult` | no / no |
| `instances status` | existing lifecycle owner observation | `AdapterStatusInput` → `AdapterStatusResult` | no / no |
| `instances stop` | existing lifecycle owner | `AdapterStopInput` → `AdapterStopResult` | no / no |
| `connections list` | connection metadata authority | `ConnectionListInput` → `ConnectionListResult` | no / no |
| `connections describe` | connection metadata authority | `ConnectionDescribeInput` → `ConnectionDescribeResult` | no / no |
| `connections connect` | acquisition/configuration coordinator | `ConnectionConnectInput` → `ConnectionConnectResult` | yes / yes, after admission |
| `connections reconnect` | exact existing connection's coordinator | `ConnectionReconnectInput` → `ConnectionReconnectResult` | yes / yes, after admission |
| `connections revalidate` | exact existing connection's evidence coordinator | `ConnectionRevalidateInput` → `ConnectionRevalidateResult` | yes / yes, after admission |
| `connections launch` | exact existing connection's read admission and the owner's custody read | `ConnectionLaunchInput` → the consumer's exit status ([consumer launch](consumer-launch.md)) | yes / no |
| `connections status` | safe connection/acquisition observation | `ConnectionStatusInput` → `ConnectionStatusResult` | no / no |
| `connections revoke` | connection metadata authority | `ConnectionRevokeInput` → `ConnectionRevokeResult` | no / no |
| `operations list` | selected cached descriptor projection | `OperationListInput` → `OperationListResult` | only to [launch for a description](#launching-for-a-description) |
| `operations describe` | selected cached descriptor projection | `OperationDescribeInput` → `OperationDescribeResult` | only to [launch for a description](#launching-for-a-description) |
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
The observation carries one `sources` line per configured source. When the quorum
forms no answer it refuses `unavailable` at `stage = observation`, and its `Failure`
names the combined refusal in `clock_refusal` and each source's line in
`clock_sources` ([refusals](../../service/clock.md#refusals)): `below_quorum` with
`check_configuration` if a source is `unverified`, else `retry_explicitly`; `split`
and `too_imprecise` (the combined answer over 8,000 ms; a verified source counts at
any width and is never `too_imprecise` on its own) with `choose_clock_source`. A local timer, continuity or calendar refusal carries neither
field and keeps `check_configuration`. The refusal never carries the observation's
bounds, a source's own interval, the request or the reply. A retired
`roughtime-clock/1` table refuses `invalid_configuration` at
`stage = configuration` with `check_configuration` and `configuration_rule`
`approval_clock_format_retired`, for `clock-check` and for every approval issuance,
execution or recovery that needs the clock; commands that do not use the clock
still load the configuration.

`approvals policy-set` accepts only `--input-file`; its path is acquired by the
production handler using the bounded document reader. `prepare` and `issue` use
one of `--input-json`, `--input-file` or `--input-stdin` for nonsecret business input.
`issue` additionally requires `--approve-subject` and `--proof-output`. Their
authority, publication boundaries and helper limits belong to the
[local mutation binding](../../service/local-mutations.md). Proof bytes never
enter the ordinary successful CLI result.

Inventory and status commands use configured/cached descriptions and safe persisted metadata
or query an already-running owner without provider authentication; `operations describe`
and `operations list` start an owner and the adapter only to
[launch for a description](#launching-for-a-description). Unavailable
authoritative metadata returns `metadata_unavailable`; cached facts are labeled
stale and cannot be used to decide fresh connection viability. Missing cached
operation metadata returns `description_unavailable`, never download,
installation or authentication, and never a launch except as follows.

#### Launching for a description

`operations describe`, `operations list` and `operations invoke` (a read or a
write) start the instance's configured adapter when, and only when, all of these hold: no description is
cached for the instance's configured selection; the started executable is the
configured one, admitted by its pinned SHA-256 as every launch is; and the
instance holds a published connection (the named one, for a command that names
one) that is not revoked or `stale`, whose credential is neither known invalid
nor expired, whose upgrade the configured revision has not refused, and that is
current under the configured entry, as `connections list` reports it with
`stale = false` (its configuration revision is the configured one, or both
record the same credential-side digest). The launch reads no credential, asks
for none and sends nothing to the provider; it only caches the description, and
an operator's `instances stop` is not resumed by it: a stopped instance answers
`description_unavailable`. A revoked or `stale` connection, one whose credential
is known invalid (its identity refused outside an executable-only change), and
one whose credential-side digest moved (a changed provider authority, profile
authentication or trust roots, for an implementation that states the digest)
or that records no digest under another configuration revision are not
current, so the command answers `description_unavailable` as before and starts
nothing. An identity refused under an executable-only change records nothing
and changes nothing, so it does not stop a launch for a description, which
reads no credential; that connection's reads and writes keep refusing as
`identity_mismatch`.

Status distinguishes unavailable owner
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
`instances list` reports configured inventory with `source: configuration` only;
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
`{"ok":false,"error":{"code":...,"data":...}}` envelope plus newline to stderr.
A command selected with the deprecated `--adapter`, or run under a renamed path,
adds a `deprecations` member to either envelope and nothing else
([selecting an instance](#selecting-an-instance), [renamed commands](#renamed-commands)).
Application error data is `Failure`, while parser/source/internal errors have
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
data on stderr, never an argv echo. `COMMAND --help` prints that command's usage,
and exactly `help COMMAND` prints the same usage with the same exit status;
`help COMMAND` with a leading global or followed by anything else stays a parse
refusal. `completions bash` emits the
generated completion function unchanged, followed by a completion for these
three commands built from their own parser and a routing function registered
for `connectors`: a command line whose command word, after any leading
`--output` selections, is one of them completes that command's own options;
any other line is completed by the generated function, with the three command
words added at the root and as the word after a leading `help`.
Help is the generated grouped tree as `connectors::help` presents it
(`apps/connectors/src/help.rs`) with what the `ess-cli/1` binding cannot
declare: each group's summary, a deprecated command path marked `Deprecated` in
its parent's listing and in its own usage line, and `--adapter` marked
deprecated. It parses exactly as the generated tree does, and help prints no
deprecation line on standard error, under an old path or with `--adapter` too.
Root `--help` or `-h`, alone or after leading process globals, prints that
help's short form, one blank line and then this block, which the generated help
does not carry:

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
file refuses the configuration, and `setup init` keeps writing `/2`. A
`connectors-local/4` file is `/3` plus an optional `[custody]` table
(`connectors.cli.LocalCustodyBinding`) that explicitly selects the custody binding:
`binding` (closed: `secret_service`, `macos_keychain`, `windows_credential_manager`,
`mounted_secret`, and one the build supports) and, exactly for `mounted_secret`,
`directory`, an absolute path with no symbolic link. `mounted_secret` beside
`secret_service_socket` refuses the configuration, as does `[custody]` in a `/1` to
`/3` file. Without `[custody]` custody is the platform binding, unchanged. The
mounted directory is an infrastructure binding of the owner configuration, like
`secret_service_socket`, not a per-credential locator
([mounted-secret custody](../../../docs/local-mounted-secret-custody.md)).
`setup init` never writes `[custody]`. It records the
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
bytes, keyring locators or actionable acquisition continuations; a `[custody]`
directory is an infrastructure binding, not a locator. A default adapter
must name exactly one entry, and so must a default instance
([selecting an instance](#selecting-an-instance)); connection defaults, if used by a future binding,
must be explicitly recorded and cannot be inferred from last usage. This first
binding requires an explicit connection for a credentialed operation.

The configuration stage needs an absolute configuration and state path, an
owner-only file and `owner_uid` equal to the effective user, and no other
variable: with both `--config` and `--state-dir` no environment variable is read.
Its `invalid_configuration` refusal names the rule that refused in
`configuration_rule` (`connectors.cli.ConfigurationRule`), never a path or OS
text: `config_path_unresolved` (no `--config`, `XDG_CONFIG_HOME` or `HOME`, or an
inadmissible path), `config_missing` (no file at the resolved path),
`config_not_private` (the file or a directory above it is not owner-only),
`owner_uid_mismatch` or, from `setup migrate` only, `config_too_large` (the
rewritten file would be larger than the 1 MiB a read admits; the refusal
carries its size in `configuration_bytes` and the bound in
`configuration_limit`). `operations invoke --definition` alone also names
`definition_instance_configured` ([invoking a definition file](#invoking-a-definition-file)).

### Selecting an instance

Every command that selects one configured instance takes `--connector <connector>
--instance <instance>`; an instance name is unique within its connector only, so
`--instance` alone selects nothing. `--adapter <alias>` keeps selecting by a
configured entry's alias for a deprecation window whose length the release plan
decides. Exactly one of `--adapter` or `--connector` with `--instance` is given;
both, or either half of the pair alone, is `invalid_input` at `arguments`. With
neither, the configuration's `default_instance` selects, else its
`default_adapter`; with neither configured the command is `invalid_input` at
`arguments`. A pair that names no configured instance is `not_found` at
`admission` with `next_action` `check_configuration`, as an unknown alias is.

An instance's names are those of the `[connectors.<connector>.instances.<instance>]`
table that names its alias. An `[adapters.<alias>]` entry that no instance names
is selected as connector `adapter_id`, instance `<alias>`, until it is migrated. A
configuration in which two entries would answer to one connector and instance
pair refuses the whole file, as does `default_instance` that is not
`"<connector>/<instance>"` naming a configured instance, `default_instance`
outside `connectors-local/5`, or `default_instance` beside `default_adapter`.

Both selectors reach the same entry and the same records: a result is the same
whichever selected it. Every result type that carries `adapter` carries
`connector` and `instance` beside it, all three required. `adapter` is the
instance's legacy alias for the deprecation window. One shape is exempt: the
service descriptor that `adapters describe` returns (the object carrying
`configuration_schema`) keeps its own `adapter`, the implementation it describes
(such as `catalog`), and its own `instance`, the instance id. Neither is a
selector there, and the descriptor carries no `connector`. The surrounding result
carries the selector fields.

`--adapter` says it is deprecated exactly once per command. Under `--output
human` that is one line on standard error, ahead of any failure text the
command writes there:

```text
deprecated: --adapter; select with --connector <connector> --instance <instance>
```

Under `--output json` nothing is added to standard error. The one envelope the
command writes, success or failure, parse refusals included, carries a third
member beside `ok` and `result` or `error`:

```json
{"ok":true,"result":{},"deprecations":[{"option":"--adapter","replacement":["--connector","--instance"]}]}
```

A caller that reads all of standard error as one JSON failure envelope keeps
reading its `error`. Under `--output json`, output that is no envelope (help,
`completions`, a launched consumer's streams) is unchanged.

### Invoking a definition file

`operations invoke --definition <file> --operation <key>`, with the input
sources of every `operations invoke`, runs an operation of a
`connectors-definition/1` file that no configuration names
(architecture-decision-record:provider-definitions-and-runtimes D12 point 3).
`--definition` is a third selector: given beside `--adapter`, `--connector` or
`--instance` the command is `invalid_input` at `arguments`, as are
`--definitions-root` and `--credential-file` without it. `--definitions-root
<dir>` is the tree holding the definition's vendored source and shared auth
files; it defaults to the parent of the directory holding the file.
`--schema` and `--revision` may be left out: each one left out is the one the
instance's description states, as `operations describe` reports it. The bare
`invoke` stays the explicit compatibility command.

The file is lowered with the reference lowerer into an owner-only cache under
the state directory, `definitions/<digest>/`, keyed by the digest of every
file the lowering reads (the definition, its vendored source, its amendments
and the shared auth files its profiles use, each by its path under the
definitions root) and of the lowerer's release. Each file is read once; the
lowerer reads a private copy of exactly those bytes, so an entry holds the
lowering of its key. A run whose inputs are unchanged reuses the entry; a
changed one is lowered into a new entry beside it. A definition the lowerer
refuses, or one that cannot be read, is `invalid_input` at `arguments` and
leaves nothing behind; `connectors-build definition` names the lowerer's
refusal. Every variable the definition's base URL names must have a default:
the instance's endpoint is the one the lowered declaration states, and its
trust roots are the platform's.

The instance is the connector's instance `definition` (the connector id is
the definition's `id`, else its file stem), with the alias and instance id
`<connector>.definition`, or its moved instance `definition-<16 hex>` when that
instance id cannot hold the file's provider authority or profile (below). Its entry pins the installed catalog provider on the
lowered configuration as a derived instance's does, and is held in the
owner-only file `definitions/instances.toml` beside the cache; a later run of a file with the
same connector id replaces that entry, so both files are one instance and its
connection follows the rules below; a run whose entry another run replaced
before it was selected is `revision_conflict` at `publication` with
`retry_explicitly`. At most 64 connector ids are held; another is
`invalid_configuration` at `configuration`. No command forgets an entry:
revoking its connection with `connections revoke` and then removing
`definitions/` from the state directory while no owner runs forgets every
entry and the cache. The connector id, the file stem and the catalog (the stem when the
definition names none) are plain names (ASCII letters, digits, `-` and `_`);
any other is `invalid_input` at `arguments`, before anything is written. An entry names the catalog provider started as
`--local-config` of its own cache entry's `native.json`, compared as paths so
every spelling of the state directory names it; an entries file holding any
other entry is not read. Every command
that selects an instance reads it beside the configuration, so `--connector
<connector> --instance definition` selects it too, but `instances list` and
`setup check` report the configuration alone. The configuration file is never
written. A configuration that already has the connector's instance
`definition`, or an entry under that alias or instance id, is never replaced:
the command is `invalid_configuration` at `configuration` with
`configuration_rule` `definition_instance_configured` and `next_action`
`check_configuration`, and lowers nothing.

Lowering and recording connect nothing: they create no connection, admit no
operation and perform no OAuth exchange (D12 point 4). When the instance holds
no connection the command asks once, on its controlling terminal, whether to
connect; only a `y` or `yes` answer starts the profile's acquisition, exactly
as `connections connect` runs it: the token prompt on the terminal, or the
file `--credential-file` names (an OAuth client file becomes the entry by
consent; under the read-only `mounted_secret` binding the owner reads the item
itself and the file is refused). Any other answer, or a run without a
controlling terminal in the foreground, is `not_found` at `admission` with
`next_action` `create_connection`; nothing is acquired and nothing is sent to
the provider. With one connection the operation runs on it with no question,
and `--connection` names one explicitly; an instance that holds more than one
is `invalid_input` at `arguments`. A change to the bundle, its source or the
selection alone keeps the connection ([one connection per instance](#one-connection-per-instance),
D12 point 2). A change of the provider authority or of the profile's
authentication cannot be followed under the instance id `<connector>.definition`
once a connection of it, revoked ones included, was admitted under the earlier
one (`docs/local-connection-registry.md`, "Configuration upgrades": only a
new instance id helps). The definition then resolves to its moved instance,
`definition-<16 hex>`: the first 16 hexadecimal digits of a digest of the
connector id, the provider authority and the profile's authentication (its
declaration apart from its revision and evidence lifetime, with its id,
purpose, subject and minimum scopes). The name is the same on every run of the
same definition, a lifetime-only change keeps it, and a file moved back to the
earlier authority resolves to `definition` and its connection again. The moved
instance is prepared, cached under its own key and recorded as the connector's
entry exactly as above, so its first run is a first invoke: the one question,
a fresh connect, and no candidate at the earlier authority. The connector keeps
one entry, so the instance id the file left is configured by no entry, and the
connections left under it are not used, changed or forgotten; `connections
revoke` of one under an instance of the same connector whose instance id is
`<connector>.<name>`, such as the moved instance, reaches it (the moved entry);
an instance of another connector does not. A configuration that already has the moved instance is
refused as for `definition` below. Reusing a stored credential for another
authority or profile is not part of this binding.

When the instance holds no connection and another instance of the same
connector holds a connection whose credential a new connection of the
definition's instance can use, the one question lists those connections and
offers to reuse one instead (D12 point 3, story:invoke-definition-reuse-credential).
A candidate is a published connection of another instance of the definition's
connector id, an instance of `[connectors.<connector>]` (a legacy `[adapters]`
entry is no instance of a connector), that is Live and not Stale, holds its
credential, has current validation evidence and is current under its own
configured entry, and is bound to the same adapter, provider authority and
profile authentication (equal, or differing only in the evidence lifetime) as
the definition's instance. Its profile is a static entry: a profile whose entry
an OAuth acquisition refreshes is never copied, since one rotation would end the
other's refresh. The definition's lowered configuration must send the
credential to its HTTPS API base alone: a profile naming a token endpoint or
any other URL, on any origin, is offered nothing, since the binding compares
the provider authority and the profile declaration, a token endpoint is part of
neither, and the source may send its secret to another place than the
definition would. The copy is validated under the platform's trust roots, as
every definition is, whatever trust roots the source's own entry pins. Under the read-only `mounted_secret` binding
nothing is offered.
A revoked, Stale, expired or moved connection, and a connection of another
connector, is never listed. The question lists at most 16 candidates, each by
number with its `<connector>/<instance>`, its provider authority and its
validated external identity (kind and subject), never material, and ends
`[1/c/N]` (`[1-<n>/c/N]` for n candidates). A number reuses that candidate; `c`
or `connect` connects fresh as above; any other answer, or a run without a
controlling terminal in the foreground, is `not_found` at `admission` with
`next_action` `create_connection`, and nothing is read, copied or sent.

On reuse the owner, never the CLI, copies the credential
([custody §4.3](../../auth/custody/v1alpha1/semantics.md#43-copy-to-another-connection)).
It repeats every check above against the current configuration and store,
admits the definition's instance as a connect does (refused while it holds a
connection or another connect of it is in flight), reads the candidate's
current version under a read use of the source, has the definition's adapter
validate it with the profile's own identity and scope probes, and publishes a
new connection of the definition's instance holding the material as its own
new custody version, only when the validation succeeds, the validated
identity equals the source's, and the source is, inside the publishing commit,
still published, Live, not Stale and under the publication fence and version
that were read: a source revoked, made Stale, reconnected or revalidated while
its copy was validated publishes nothing (`revoked`, or `lifecycle_conflict`). The acquisition is recorded as a connect's, and
`connections status --acquisition` follows it. Otherwise nothing is published,
the acquisition is failed, and the command offers to connect fresh with one
`[y/N]` question; declined, the command answers the reuse's own refusal:
`identity_mismatch` at `admission` with `next_action` `create_connection` when
the provider answered another identity, otherwise the validation's or the
owner's failure. A run that cannot know whether the copy published
(`outcome_unknown`, `interrupted`) offers nothing. Either way the source
connection is unchanged (its revision, state, evidence and identity), and
after a copy the two share nothing: revoking or reconnecting either leaves the
other as it was.

From the selection on, the command is the configured instance's: the
description, admission, approval of writes, audit and the result envelope are
those of every `operations invoke`, and a result names connector
`<connector>`, instance `definition` and adapter `<connector>.definition`, or
the moved instance and `<connector>.definition-<16 hex>`.

### Renamed commands

`connections reconnect` replaces `connections repair`, and the `instances` group
replaces `adapters`: `instances list`, `instances describe`, `instances status`
and `instances stop`. The commands address an instance; the process they
observe and stop is still that instance's adapter. Each old path stays an alias
of its new one for the deprecation window: the same arguments run the same
command against the same instance with the same result. These are the
binding's only path aliases.

An old path says it is deprecated exactly once per command, as `--adapter`
does. Under `--output human` that is one line on standard error, ahead of any
other deprecation line and any failure text:

```text
deprecated: connections repair; use connections reconnect
```

Under `--output json` the one envelope's `deprecations` member carries the
entry, ahead of an `--adapter` entry when both apply:

```json
{"ok":true,"result":{},"deprecations":[{"command":"connections repair","replacement":["connections reconnect"]}]}
```

The machine-facing `next_action` value `repair_connection` is unchanged; it
names `connections reconnect`.

### One connection per instance

An instance holds at most one published non-revoked connection
([connection semantics](../../auth/connection/v1alpha1/semantics.md) §4.2).
`connections connect` is refused while the selected instance holds a published
Live or Stale connection, before any credential entry or provider exchange, as
`lifecycle_conflict` at `admission`. Its `next_action` is the
step the held connection's own reconnect admits:

| The instance holds | `next_action` |
|---|---|
| one connection under the configured binding, or one whose binding differs from it only on the operation side (bundle, source or selection, under an equal credential-side digest) | `repair_connection`: `connections reconnect` replaces its credential and keeps its reference |
| one connection saved under an older configuration revision that a revalidation can follow | `revalidate_connection`, after which it is the instance's connection |
| one connection that reconnect refuses as changed (`create_connection` on a read), or more than one | `revoke_connection`: `connections revoke` it, then connect |

A connection with another identity (provider authority or profile
authentication) keeps `check_configuration`, as before. The refusal is the same
under every custody binding: under the read-only `mounted_secret` binding the
second connect of an instance is this `lifecycle_conflict`, not
`custody_read_only`. A connect is also refused while another connect of the
instance is in flight, as `lifecycle_conflict` at `admission` with
`next_action = retry_status`, and the failure data's `acquisition` names that
connect's acquisition, which `connections status --acquisition` follows until
it ends; it is absent when that connect ended before the refusal was read. Two
concurrent connects therefore never publish two connections. Revoked
connections stay as history and do not count, and neither does a connection a
connect allocated and never published: once that connect failed or its
acquisition passed its deadline, the next connect is admitted.

A second account is a second instance. `connections reconnect` keeps the
connection's external identity, so a reconnect whose credential the provider
answers as another account is `identity_mismatch` at `admission` with
`next_action = revoke_connection`, never `repair_connection`: connect names
the reconnect, and reconnecting again with the other account answers the same.
Revoking the connection lets the instance connect the other account; keeping
it, the other account is a connection of another instance.

An instance that held more than one published non-revoked connection before
this rule keeps every one of them, references unchanged, and each stays
usable, reconnectable and revocable. A new connect on it is refused with
`revoke_connection` until all but one are revoked.

`connections describe`, `reconnect`, `revalidate`, `launch` and `revoke` take
`--connection` optionally. Without it they select the instance's one published
non-revoked connection; `--expected-revision`, where the command takes it, is
still required. An instance that holds none refuses as `not_found` at
`admission` with `next_action = create_connection`; one that holds more than
one refuses as `invalid_input` at `arguments`, and `--connection` chooses.
`connections status` keeps requiring `--connection` or `--acquisition`. Under
`--output json` each of these refusals is the one failure envelope on standard
error.

### Migrating adapter entries

`setup migrate --mode dry-run` and `setup migrate --mode apply` move legacy
`[adapters.<alias>]` entries to connector instances without a new consent,
revalidation or approval policy publication. `--dry-run` and `--apply` are the
same two commands. Each entry whose implementation maps to a shipped connector
declaration becomes `[connectors.<connector>.instances.<alias>]` carrying every
pin of the entry, its alias included. The instance name is the alias, so the
instance is selected as `--connector <connector> --instance <alias>`, and
`--adapter <alias>` keeps selecting it for the deprecation window. Its
`instance_id`, `configuration_revision`, executable selection digest (an approval
policy's `executable_selection`) and mounted custody item names
`<alias>/<profile>` stay byte for byte, so its connections, approval policy and
custody items stay valid. Platform custody items are keyed by connection, not by
configuration names. Read and write instance pairs are not merged: merging
changes scopes and so the revision.

Only a catalog entry maps to a connector today: the connector whose shipped
declaration names the `provider` of the entry's native configuration as its
`native_provider` (the declaration file's `catalog_provider` key)
(`--local-config <file>`, admitted as the provider admits it: owner-only, at
most 1 MiB), or the shipped connector its `--connector` argument names. Every
other entry stays where it is, and the result names why in `reason`
(`connectors.cli.MigrateKeptReason`):

| `reason` | The entry |
|---|---|
| `implementation_unmapped` | is not a catalog entry: no declaration maps a credential binary, MCP, SQL or Kubernetes implementation to a connector yet |
| `connector_undeclared` | names a native provider no shipped declaration carries (Slack ships none) |
| `gateway_prefix` | states a request prefix its connector's declaration does not produce: an API gateway in front of the provider's API, such as the Atlassian gateway |
| `native_configuration_unreadable` | names no native configuration, or one that is not owner-only, not readable JSON or names no provider |
| `instance_conflict` | maps to a connector configured disabled, or one that already has an instance named like the alias |
| `selection_changes` | would read back with another value of the field `differing_field` names |

The new file is read back with the reading rules above before anything is
written, its 1 MiB bound included: a rewritten file larger than that is
refused in both modes as `invalid_configuration` naming `config_too_large`,
and nothing is written. An entry that reads back with any other value of any
of its fields is not migrated, naming the first such field. So is one whose
new instance table reads back differently (`instance`, or the first of
`description`, `endpoint` and `profile`) or whose connector's pre-existing
table does (`enabled`, or an existing instance's `description`, `endpoint`
or `profile`, which keeps every entry moving to that connector); a `format`
that reads back other than `connectors-local/5` keeps every entry. Moving to `connectors-local/5`
requires `private_protocol` on every entry, and it enters the executable
selection digest, so no entry of a `connectors-local/1` file is migrated; each
candidate names `private_protocol`. Nothing else of the file changes, but its
comments are not kept.

The result lists every `[adapters.<alias>]` entry of the file it read with its
`disposition` (`migrated` or `kept`), the `connector` and `instance` that select
it afterwards and, `before` and `after`, its `instance_id`,
`configuration_revision`, `executable_selection` and `custody_items`. It carries
the file in `configuration` and its `format_before` and `format_after`. The
format moves to `connectors-local/5` only when at least one entry is migrated.
With nothing to migrate the result is `unchanged` and nothing is written in
either mode. A dry run that would migrate is `planned` and writes nothing. An
applied one is `applied`: it publishes the file it read, byte for byte and
owner-only, as `config.toml.before-migrate-<UTC timestamp>`
(`YYYYMMDDTHHMMSSZ`) beside the configuration without replacing any existing
file, names it in `backup_path`, and then replaces the configuration atomically
and owner-only, only while it still holds the bytes the plan was made from; a
configuration that changed in between is `revision_conflict` at `publication`.
Rollback is restoring the backup and running the previous release. Under
`--output json` a refusal is one failure envelope on standard error, like every
other command's.

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
   store unchanged. The command never signals the owner; the user stops it with
   `connectors setup owner-stop`, as for `owner_build_mismatch`.
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
be detected; it fails to open the store once enabled.

Which open verifies what: the provider's tracked open of a store with
checkpoints verifies the persisted checkpoint and the suffix after it, and on
its own does not detect raw edits of the database file that bypass SQLite while
no handle is open (Entity Runtime 0.29.0). Every fresh open the host makes, in
a command run directly, a second process, the owner when it starts and recovery
alike, is that tracked open followed by a complete read of the store before it
answers anything; Entity Runtime 0.29.0 treats a complete read as a complete
verification, and no open uses the separate `FullVerification` policy. So it
verifies the whole store,
so such an edit is refused as `metadata_unavailable` by the first command or
owner start after it. Only a handle a process already verified and keeps for
later opens in that process reads just what was appended since; the owner keeps
its handles for its whole run. A command run while an owner of its own build
runs reads the store through that owner (its `cached` request and the owner's
admission of `invoke`, [local owner transport](owner.md)) and does not open it;
without such an owner it opens the store directly and verifies it completely.
A write through any SQLite connection is proven or makes the next read verify
completely, as before. Due expiries of transient subjects are recorded in
batches of at most 128 before the batch carrying the command's own change; a
refused expiry batch leaves earlier ones committed and the command's change
unapplied, with its failure unchanged.

### Store writer marker and binaries of different versions

The metadata store records its newest writer in one `StoreWriter` record
(identity `store`): the release version of the newest binary that wrote it and
the digest of the record model that binary persists. The digest is a SHA-256
over the persisted record shapes of the Entity Runtime definitions the store's
records are checked against: for each entity its name, identity field,
lifecycle states, field schema (names, types, required-ness, bounds and
enumerations) and the assertions of its invariants. Invariant names and
messages, which the generator chooses, are not part of it, so two releases that
persist the same record shapes have the same digest even when the generator
that lowered them changed. A release version is a SemVer 2.0.0 version, ordered
by SemVer precedence: a pre-release orders below its release, and build
metadata is ignored. A binary records the marker in the batch of its first
write when the store has none, and raises it in the batch of its first write
when its own release version is newer; an equal or
older binary leaves it as it is, so the marker is never lowered. A store written
before the marker existed has none, is admitted, and gets one on its next write.

Every command that opens the metadata store, `setup check` included, compares
the marker with its own release version and model digest after it opens the
store and before it reads a record. When the digests differ and the marker's
version is newer than the binary's, the command refuses `metadata_schema_newer`
(`kind = operational`, exit 1, `stage = admission`, `next_action = upgrade`),
and the `Failure` names `store_version` (the marker's version) and
`binary_version` (its own); it reads and writes nothing. The same digest under a
newer version, a release that changed no persisted record, is admitted. A marker
whose version this binary cannot order (not a SemVer version, or a pre-release
longer than 40 bytes) under a different digest is refused the same way with
`store_version = "unrecorded"`; under the same digest it is admitted and kept,
since it is never known to be lower. A store whose SQL level (`user_version`) is
above the highest this binary knows was migrated by a newer binary and is
refused the same way with `store_version = "unrecorded"`, before any record is
replayed: its marker is not read through a schema this binary does not know. A
store that fails integrity verification is not a newer store and stays
`metadata_unavailable`. A command an owner of its own build answers reports the
owner's refusal the same way.
Independently of the marker, a write the Entity Runtime refuses because a record
carries a field this binary's model does not declare is `metadata_schema_newer`
too, with `store_version = "unrecorded"` unless a newer marker names the writer;
it is never `metadata_unavailable` or `retry_status`, because running the same
binary again cannot change the record. None of these refusals changes the store.

So once a binary has written a state directory, an older binary may not use it:
binaries of different versions share one state directory only while the oldest
of them is at least the newest that wrote it. Releases before the marker cannot
read it and answer such a store as they always did, with `metadata_unavailable`
(`next_action = retry_status`), which retrying does not change. A consumer pinned
to a version keeps its own state directory with `--state-dir`, initialises it
with `setup init`, and connects there; connections, cursors, approvals and
cached descriptions recorded in another state directory are not visible in it.

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
ordinary local files are never fallback custody. The one exception is not a fallback:
the read-only `mounted_secret` binding, used only when the owner configuration
explicitly selects it (custody §5.2), reads files a platform mounted read-only and
writes nothing; an unusable platform binding still answers `custody_unavailable`. Keyring locations remain private
inside the binding. Secret Service's actual durability/ownership behavior remains
a required backend conformance check before this binding is advertised.

Exactly one protected entry source is selected for connect/repair: hidden terminal,
protected file or stdin, each explicitly selected by its own source flag. Under the
read-only `mounted_secret` binding connect and repair take no source: the owner
reads the item `<adapter alias>/<profile>` itself, and a source flag is
`invalid_input` at `arguments`. Under a platform binding a missing source is
`protected_entry_unavailable` with `next_action = select_protected_source`. Hidden
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

`connections reconnect` binds the same connection identity, immutable profile/owner/target and
expected revision. Same-principal/same-target credential renewal may publish a new
private generation and advance its private publication fence while preserving
both connection reference and public semantic revision. Only separately admitted
effect-relevant configuration/management changes advance that semantic revision
under the shared fixed-binding rules; material renewal or evidence recollection
alone does not. Different
principal, account or provider target returns `identity_mismatch` with
`next_action = revoke_connection`: another account is another connection,
which the instance holds only once its own is revoked
([one connection per instance](#one-connection-per-instance)). A failed or uncertain candidate
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
`operations describe`, in the `instances describe` descriptor and in the
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
| decoded business document the operation schema, depth or byte bound refuses; `approvals prepare` or `approvals issue` business document that does not decode or has a trailing document; unreadable business document file; argument value the handler refuses; on an approval command, `field` names the argument ([below](#the-refused-argument)) | `invalid_input` | 2 |
| TOML syntax, unsupported startup/restart, duplicate identity, bad permissions | `invalid_configuration` | 2 |
| configuration already exists (`setup init`: with a metadata database, or one that does not load) | `configuration_exists` | 1 |
| no safe entry channel (in place of the presentation's `cli_source`) | `protected_entry_unavailable` | 2 |
| `setup checkpoints-enable` without `--confirm one-way`; nothing changed (`stage = arguments`, `next_action = retry_explicitly`) | `confirmation_required` | 2 |
| keyring unavailable or durable storage not acknowledged | `custody_unavailable` | 1 |
| a read-only custody binding refused a write it would need (a refresh-rotating profile, an approval-signing key; stage = custody, next_action = check_configuration) | `custody_read_only` | 1 |
| the read-only binding's item no longer holds the bytes its version pinned (stage = custody, next_action = revalidate_connection) | `custody_unavailable` | 1 |
| metadata authority unavailable | `metadata_unavailable` | 1 |
| store written by a newer binary whose record model differs (its marker names a newer or an unorderable version), a store whose SQL level is above this binary's highest, or a store record carrying a field this binary's model does not declare; nothing read or written (`stage = admission`, `next_action = upgrade`, `store_version` and `binary_version` named) | `metadata_schema_newer` | 1 |
| a non-guarded metadata write the store changed under (a concurrent commit); nothing was written (`stage = publication`, `next_action = retry_explicitly`). A guarded write keeps the guarded-write rule below: its mutation record, `next_action = retry_status` | `revision_conflict` | 1 |
| metadata publication acknowledgement uncertain | `outcome_unknown` | 1 |
| changed principal/account/target during repair (`next_action = revoke_connection`, [one connection per instance](#one-connection-per-instance)) | `identity_mismatch` | 1 |
| stale revision, occupied changed configuration; `setup checkpoints-enable` while an owner runs for the state directory (`stage = admission`, `next_action = stop_owner`: run `connectors setup owner-stop`); `connections connect` on an instance that holds its connection, or while another connect of it is in flight ([one connection per instance](#one-connection-per-instance)) | `lifecycle_conflict` | 1 |
| stale/foreign stop target | `incarnation_mismatch` | 1 |
| wrong readiness identity/version/revision | `readiness_mismatch` | 1 |
| running owner is a different executable build (`next_action = stop_owner`: run `connectors setup owner-stop`) | `owner_build_mismatch` | 1 |
| no selected cached description | `description_unavailable` | 1 |
| schema/descriptor changed | `stale_description` | 1 |
| absent selected operation/connection | `not_found` | 1 |
| stale, expired or foreign list cursor (`stage = observation`, `next_action = retry_explicitly`) | `stale_cursor` | 1 |
| connect whose acquisition failed before publication (`stage = dispatch`, `next_action = retry_explicitly`) | the acquisition owner's code | 1 |
| known revoked connection | `revoked` | 1 |
| current permission missing | `forbidden` (or shared `not_granted`) | 1 |
| unavailable service / budget expired | `unavailable` / `timeout` | 1 |
| `setup owner-stop`: state directory, owner socket or lock file of another user (`stage = stop`, `next_action = check_configuration`) | `forbidden` | 1 |
| `setup owner-stop`: the lifetime lock is held and no owner answers within the bound; nothing is signalled (`stage = stop`, `next_action = retry_explicitly`) | `unavailable` | 1 |
| `setup owner-stop`: the owner accepted the request and still holds the lifetime lock at the bound (`stage = stop`, `next_action = retry_explicitly`) | `timeout` | 1 |
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

### The refused argument

An `invalid_input` that `approvals policy-set`, `approvals prepare` or
`approvals issue` decides names the argument it refused in `Failure.field`, as
the CLI input model names it: `connection`, `operation`, `schema`, `revision`
or `input` (the business document, from whichever of `--input-json`,
`--input-file` or `--input-stdin`) for the target, `approve_subject` and
`proof_output` for `issue`, `input_file` and `expected_revision` for
`policy-set`, and `adapter` for the instance selection (`--connector` with
no `--instance` or the reverse, `--adapter` beside either, or no selector and
no default instance; an unknown instance stays `not_found`). `issue` refuses
`proof_output` unless it is an absolute path of plain names, no component is
a symbolic link (each is opened without following one, so a link to a private
directory is refused too), its directory exists, is the caller's own and
admits no other user, every ancestor is writable only by its owner (the
caller or root), and the directory holds no entry of that name; an existing
file is never replaced. A path relative to the working directory is refused,
not resolved. `field` never carries the caller's value and appears only on
`invalid_input` of those three commands; no other command carries it, an
approved write's `operations invoke` included; a refusal the presentation raises
before the handler runs carries its `ess-cli/1` code and no `field`. As for
`instance_id`, the presence rule is a host obligation: ESS declares no
invariant on a CLI type.

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
succeed, and `connections reconnect` replaces it. A credential refused while a connection is
made (`connections connect`, `connections reconnect`) or revalidated is not this
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

A read on `operations invoke` whose connection's validation evidence expired
while its credential is intact (the connection reports `pending`) recollects
that evidence once before it is admitted, as `connections revalidate` does: the
owner, on the instance's serialized worker, captures the retained custody
version, runs the profile's identity probe on exactly those bytes, publishes
the new baseline under the unchanged generation and revision, and then admits
the read against it. Evidence still current that would lapse before the read
could use it counts as lapsed: a read whose evidence has less than half its
own lifetime left (from its collection to its `valid_until`, which the provider's
identity probe may set below the profile's `evidence_lifetime_ms`, never the
profile's bound), and less than the read's own remaining deadline, recollects it
first, because a read's use and its provider deadline never outlast the
evidence it was admitted on. Evidence fresh from a recollection therefore
admits its read without another one. Evidence that still lapses between the
read's capture and its dispatch is refused as expired evidence (below), never
as the read's own `timeout`. The caller revalidates nothing: a connection connected or
revalidated once keeps serving reads for as long as its credential stays valid,
across any number of evidence lifetimes. The recollection asks for no
credential, writes no custody version and never follows changed material: a
mounted item whose bytes changed still refuses the read with
`code = custody_unavailable` and `next_action = revalidate_connection`, and
only `connections revalidate` re-pins it. A credential the provider refuses in
the recollection is cut off as an explicit revalidation cuts it off, and the
read answers as for a stored credential the provider refuses on a read:
`code = service_failure`, `service_code = unauthorized` with
`next_action = repair_connection`; the business read is not dispatched. A
recollection run only because the evidence would lapse early, while it is
still current, keeps the read admitted on that evidence: when it fails with
anything but such a credential refusal (the provider or custody briefly
unavailable, a timeout), the read is served on the evidence it was admitted
on, for as long as that evidence stays current, because a transient failure
preserves the still-valid baseline. Any other failure of the recollection of
expired evidence is the read's own answer, before dispatch.
Only a read still refused for expired evidence after its one recollection
answers `code = not_granted` at `stage = admission` with
`next_action = revalidate_connection`. A consumer launch
(`connections launch`) recollects nothing and keeps its own refusal
([consumer-launch.md](consumer-launch.md)). A connection whose credential the provider refused
or is known invalid (`stale`), one whose credential expired
(`reauthorization_required`), and a credential below the operation's scopes,
keep `code = not_granted` with `next_action = repair_connection` and recollect
nothing.

A connection whose binding and configured entry both record a credential-side
digest (`credential_revision`, the implementation's digest of what decides
where the credential is sent and how it authenticates; never the bundle, its
source or the selection) is bound to it and to the provider executable, not to
the configuration revision. A change that leaves the digest equal is not a
stale configuration: when only the bundle, source or selection moved the
connection reports `ready` with `stale = false`, and a read, a reconnect, an
approval naming it and a consumer launch admit it with no credential source
and no question; its next revalidation records the new revision. When only the
provider executable changed, it also reports `ready` with `stale = false`; a
read validates its stored credential once under the new executable and is
served when the recorded identity answers, and a revalidation follows it the
same way; another identity is refused as `identity_mismatch` and changes
nothing. Every other change, and every change for a connection whose binding
records no credential-side digest (saved before it was recorded, or under an
entry that states none), follows the configuration-revision rule below. A
changed configuration's description is cached by the first read command that
[launches for it](#launching-for-a-description); a write after an
executable-only change validates the connection once, as a read does.

A connection saved under another configuration revision of its instance also
reports `pending` (`stale = true`), and no status change is pending for it: a
read on `operations invoke`, an approval naming it and a consumer launch refuse
it as `code = lifecycle_conflict` at `stage = admission`, never with
`next_action = retry_status`. The next action is `revalidate_connection` only
when a revalidation can follow the configuration upgrade: its provider
authority and profile declaration are unchanged, its credential is neither
known invalid nor expired, and the new provider has not already refused it
under the configured revision. The connection's identity is the instance,
adapter, provider authority and the profile's authentication, which is the
profile declaration apart from its evidence lifetime. A configuration that
changes only `evidence_lifetime_ms` leaves the identity unchanged, but the
connection stays bound to the declaration it was admitted under, so the next
action is `create_connection`: `connections connect` under the same instance
id names the old connection's revocation while the instance holds it
(`revoke_connection`, [one connection per instance](#one-connection-per-instance)),
then admits the new connection. When the
provider authority or the profile's authentication changed, the next action is
`check_configuration`: no command under the same instance id can help, since
`connections connect` there is refused for the same reason (`lifecycle_conflict`
at `stage = admission`, `next_action = check_configuration`), and only a new
instance id admits a new connection. The connections left under the earlier
instance id stay revocable through the entry, under the same adapter, that
moved: `connections revoke` reaches a connection whose recorded instance id no
entry configures any longer, never one another entry still configures.
Otherwise it is `create_connection`, a saved connection whose profile id the
configured adapter no longer declares included (revalidate and repair refuse it
as a read does, never as `unsupported`); connect under a declared profile
admits the new connection. A
`connections reconnect` of a connection saved under another binding is classified
the same way, never as `identity_mismatch`: reconnect refuses a changed binding. A
`connections revalidate`, `connections connect`, `connections reconnect`, read or
write whose adapter process still runs an earlier adapter entry stops that
process and launches the configured one before it captures anything, so no
command answers `retry_status` for it until `instances stop`. Only a credential
validation captured by the earlier process is still refused as
`lifecycle_conflict`; its `connections connect` starts again with a launch
that replaces the process. A revalidation the new provider refuses (the
credential invalid, below scope or another identity) answers `create_connection`
and records the refusal against the configured revision
(`Connection.refused_configuration_revision`), so the next read names
`create_connection` too; the refusal invalidates nothing under the connection's
own revision, revalidation stays admitted, and a later upgrade that succeeds
or a repair that replaces the credential removes it. An approval reads the
wall clock for this classification only, as the read path does, without
advancing the registry clock. `connections connect` under the configured
revision is admitted once the instance holds no published non-revoked
connection; an instance from before that rule may still hold older ones saved
under their own revisions. The instance's
own recorded configuration revision moves to the configured one only when a
connection is published under it: a revalidation that upgrades one, or a new
connection whose publication succeeds while the instance still records the
revision it had when that connect began. A connect that fails or is abandoned
moves nothing, and a connect begun under an older configuration never moves
the instance back. Until it moves, the instance's approval keys (`approvals
key-status` and the other key commands) and its approval policy commands
refuse as `code = lifecycle_conflict` with `next_action = create_connection`.

A provider's timeout or capacity answer is the provider's too: a request the
provider transport sent whose deadline then passed (a connection probe's
included, and one that passes while the provider's answer is still being
read after its status and headers arrived), a database statement timeout, and a capacity refusal the adapter
marks as the upstream's answer (the SQL adapter's database connection limit)
report `timeout` / `capacity` at `stage = dispatch` with
`next_action = retry_explicitly`. A connection that never opened sent nothing
and is not one of them. The host's own deadline and limits, and an adapter's
own connect or work deadline and result bounds, keep `stage = admission`
(`retry_explicitly`). An `operations invoke` read whose deadline passes after
the owner dispatched it (its read use recorded dispatched) is the provider's
`timeout` at `stage = dispatch`; one whose deadline passes before is the host's
at `stage = admission`, and the owner dispatches nothing for it afterwards. The
owner answers by the deadline; an invoke the CLI sent whose answer it never
receives may have been dispatched and reports `outcome_unknown`.
An HTTP 429 stays `service_failure` with
`service_code = rate_limited`, and with `retry_after_seconds` when the provider
named a delay. `unsupported` is always `admission`: every
source of it is raised before dispatch.

**Revalidation retries.** `connections revalidate` sends its validation (the
adapter's identity probe, a read) again when the attempt failed transiently,
inside the same dispatched revalidation and within its deadline. A transient
failure is the adapter answering `unavailable` (a provider connection refused,
reset or closed, such as a pooled keep-alive connection the provider closed
while it was idle; a name that does not resolve; the catalog provider's
identity or scope probe also on an HTTP 429 or 5xx), a provider timeout, rate
limit, internal error or marked capacity answer. The owner sends at most three
attempts, waiting 200 ms before the second and 1 s before the third, and only
while the adapter process still runs, the owner is not stopping, and the wait,
then an attempt as long as the last one took, then a 500 ms margin, all end
before the deadline. A credential the provider refuses (HTTP 401), a
permission refusal (403), a credential below the profile's scopes, an identity
mismatch and a protocol violation are never sent again. When every attempt
failed, the failure keeps its code, stage and next action, and additionally
carries `cause`, the class of the last attempt
(`connectors.cli.FailureCause`), and `attempts`, the number of validations the
owner sent, when it sent any. `cause` is `adapter_exited` when the adapter
process was no longer running after the failed attempt, and `owner_stopping` when the owner began to
stop during the revalidation. No other command and no other failure carries
either field; a revalidation whose reply never arrived stays `outcome_unknown`
and carries neither.

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
write's preflight is `retry_explicitly`. A write whose selection declares a
conflict status, answered with it (the target changed since it was read, nothing
applied), is `refused` as `service_failure`, `service_code = conflict` and is
never sent again. A preflight declaring `mismatch: conflict` whose check differs
refuses the write unsent with the same `service_failure`, `service_code = conflict`. A write classified `applied` or
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

### macOS platform guarantees

The host builds on seven operating-system guarantees, each fixed by the platform
the binary was built for. `connectors.cli.PlatformGuarantee` names them,
`connectors.cli.PlatformGuaranteeStatement` states each row below as an
invariant, and `connectors.cli.PlatformGuaranteeWeakerForm` names every weaker
form. Linux holds all seven, by the mechanisms the rest of this binding states.
On macOS one holds and six hold only in a named weaker form:

| Guarantee | Linux | macOS | macOS mechanism |
|---|---|---|---|
| `sealed_executable` | holds | weaker: `owner_private_copy` | no memfd: the image is copied into a mode 0500 file in a fresh mode 0700 directory, and the digest and Mach-O header are checked on that copy before exec by its path. Other users cannot change it; a process of the same user can rewrite it between the check and exec. A copy that is refused or never launched is removed with its directory; a launched copy is removed by its launch once the exec it serves no longer needs it. |
| `parent_death_kill` | holds | weaker: `adapter_channel_close` | no `PR_SET_PDEATHSIG`: an adapter exits when its inherited channel on descriptor 3 reaches end of file. A consumer has no channel and outlives a launcher that dies. |
| `exact_process_signal` | holds | weaker: `child_reap_lock` | no pidfd: kqueue `EVFILT_PROC` with `NOTE_EXIT` observes exit, and `kill` signals the child's process id only while holding the lock that reaps it, so the id cannot be reused between the check and the signal. |
| `own_image_build_measure` | holds | weaker: `launch_path_read` | no `/proc/self/exe`: the owner build digest is read from the executable's path. A file replaced there is measured as a new build and refused as `owner_build_mismatch`. |
| `atomic_no_replace_rename` | holds | holds | `renamex_np` with `RENAME_EXCL`, on a filesystem that reports `VOL_CAP_INT_RENAME_EXCL` (APFS). A state directory on any other filesystem is refused. |
| `atomic_close_on_exec` | holds | weaker: `set_after_receive` | no `MSG_CMSG_CLOEXEC`: `FD_CLOEXEC` is set on each received descriptor right after `recvmsg` returns. An exec by another thread of the same process in between inherits it. |
| `peer_credential_snapshot` | holds | weaker: `separate_pid_query` | no `SO_PEERCRED`: `LOCAL_PEERCRED` gives the connect-time uid and gid, and `LOCAL_PEERPID` the pid in a separate query. The uid check is unchanged. The owner and the supervisor use the pid only for its sign; an adapter also requires it to equal its parent's process id, the owner that made the socket pair, as on Linux. A peer that has already closed has no pid to query: the owner and the supervisor admit it by its connect-time uid, as Linux does, and an adapter refuses it. |

Protected stdin (§5) keeps its rule on macOS with other mechanisms, and no
guarantee row changes. A protected regular file is checked as on Linux, its
parent directory found through `F_GETPATH`. macOS names no `pipe:[…]`: an
inherited anonymous pipe is a FIFO owned by the admitted UID for which
`F_GETPATH` returns no path, and a FIFO with a path is a named FIFO and
refuses. The producer check reads the parent's owner through `proc_pidinfo`
in place of `/proc/<parent>`: the parent must not be pid 1, must share this
process's session, and must be owned by the admitted UID.

Exact stop (§4) requires "Linux pidfd or equivalent non-reusable kernel process
ownership"; on macOS the equivalent is `child_reap_lock`, which covers the
owner's own children only, and those are the only processes stop signals. The
runtime acceptance above applies on macOS with each Linux mechanism replaced by
its row here: stop is exercised through the reap lock, and the owner-only
descriptor and socket-peer checks through `LOCAL_PEERCRED`.

Windows has its own rows, in the next section. Every Linux rule above stands
as written. These macOS rows are specified; until the host and CLI suites run on a macOS runner, none of them
is observed.

### Windows platform guarantees

The same seven guarantees apply on Windows, stated by the same invariants of
`connectors.cli.PlatformGuaranteeStatement`. Linux and macOS keep their rows
above. On Windows four hold and three hold only in a named weaker form:

| Guarantee | Linux | macOS | Windows | Windows mechanism |
|---|---|---|---|---|
| `sealed_executable` | holds | weaker: `owner_private_copy` | weaker: `deny_write_copy` | no memfd: the image is copied into a fresh owner-only directory inside the state directory's owner-only `run` directory, never the system's temporary directory, and the copy is opened for reading without `FILE_SHARE_WRITE` or `FILE_SHARE_DELETE` before its digest and PE header are checked on that handle. The handle, and one on its directory without delete sharing, are held until the child is created, so from that open no process can write, rename or delete the copy or rename its directory; after creation the mapped image keeps it unwritable, and the child's current directory, which is the copy's directory, keeps that directory in place. A copy is removed with its directory on every refusal; one left by a process that died is removed when the next owner starts, unless a running process still holds it. A writable mapping that a process of the same user made of the copy before that open is not excluded. |
| `parent_death_kill` | holds | weaker: `adapter_channel_close` | holds | no parent-death signal: every child is created inside a job object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, through `PROC_THREAD_ATTRIBUTE_JOB_LIST`, so it is never outside the job, and its launcher holds the job's only handle. When the launcher dies the handle closes and the job ends every process in it, consumers included. An adapter's job permits no breakaway. A consumer's job permits breakaway on request (`JOB_OBJECT_LIMIT_BREAKAWAY_OK`, never `JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK`), so a process the consumer creates with `CREATE_BREAKAWAY_FROM_JOB`, such as an owner, leaves it; the consumer itself stays in it. |
| `exact_process_signal` | holds | weaker: `child_reap_lock` | holds | no pidfd: the launcher holds the child's process handle, which names exactly that process and keeps its id from reuse while open. Exit is observed by waiting on the handle, and stop terminates through it (`TerminateProcess`); Windows has no signal to send first. |
| `own_image_build_measure` | holds | weaker: `launch_path_read` | weaker: `launch_path_read` | no `/proc/self/exe`: the owner build digest is read from the path `GetModuleFileNameW` reports. A running image cannot be written in place, but it can be renamed and another file put at its path; that file is measured as a new build and refused as `owner_build_mismatch`. |
| `atomic_no_replace_rename` | holds | holds | holds | `MoveFileExW` with `MOVEFILE_WRITE_THROUGH` and without `MOVEFILE_REPLACE_EXISTING` refuses an existing destination in the same call, on NTFS. A state directory on any other filesystem is refused. |
| `atomic_close_on_exec` | holds | weaker: `set_after_receive` | holds | no descriptor passing: a handle reaches the other process through `DuplicateHandle` with inheritance off, so it is not inheritable from the moment it exists there. A child is created with `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`, and only the handles listed there reach it. |
| `peer_credential_snapshot` | holds | weaker: `separate_pid_query` | weaker: `client_token_query` | no Unix socket: the owner listens on a named pipe whose access control list grants the current user's SID alone, so the kernel admits no other user at connect. `GetNamedPipeClientProcessId` gives the client's process id as recorded at connect; the client's SID is then read from that process's token, a separate query, and must be the current user's. If the client exited and its id was reused before the query, the token read is another process's. The token's integrity level must also equal the reader's own, and the token must not be restricted: an elevated and a non-elevated process of one user, or a restricted one, do not admit each other, on either end, and refuse with `lifecycle_conflict`. The owner opens an admitted client's process once from that id; the checks and every handle duplicated into the client go through that one process handle. |

Owner-only files keep their rule (§5) with another mechanism, and no guarantee
row changes. Ownership and permission are checked on the opened handle: the
owner SID is the current user's, the discretionary access control list exists,
and every entry that allows access names the current user, LocalSystem or the
built-in Administrators group, which correspond to what root may read on Linux
whatever the mode bits say. Any entry allowing another SID refuses, as group or
other mode bits do. A file is opened with `FILE_FLAG_OPEN_REPARSE_POINT` and
refused when it, or any directory on its path, is a reparse point (a symbolic
link or junction); it must have one link. A sidecar of the metadata database
that SQLite's last close is deleting keeps its name until every handle to it
closes, and opening it answers access denied meanwhile; one deleted after it
was opened shows no link on its handle. Such a sidecar is probed again, within
the lifecycle lock's wait, until its name is gone, and is then absent; one
still there at the end of the wait refuses, and a present sidecar that is not
being deleted keeps the full rule. Files and directories the host
creates get a protected list naming only the current user. Protected stdin
admits a protected regular file; a pipe on stdin is refused on Windows until a
producer check is specified for it.

Where a rule above fsyncs a directory (`setup init`, the metadata database's
creation and each migration), Windows takes no directory step: a directory is
held open for listing only, and Windows flushes no directory through such a
handle. A file the host publishes is made durable by its own flush and by
`MoveFileExW` with `MOVEFILE_WRITE_THROUGH`, which returns only once the rename
is written; the metadata database's files are flushed by SQLite, which on
Windows syncs files and never their directory.

Hidden terminal entry opens `CONIN$` and `CONOUT$`, clears
`ENABLE_ECHO_INPUT` and restores the saved console mode on success, failure and
interruption. A process with no console has no trusted terminal and refuses as
on Linux.

The clock reading of the [clock binding](../../service/clock.md) uses
`QueryUnbiasedInterruptTimePrecise`, which stops while the machine sleeps, in
place of the suspend-exclusive clock, and `QueryInterruptTimePrecise`, which
counts sleep, in place of the suspend-inclusive one. The non-precise forms move
only at each timer interrupt, coarser than the 1 ms the binding requires.

Exact stop (§4) is held through the process handle on Windows, which is the
"equivalent non-reusable kernel process ownership" §4 requires. The runtime
acceptance above applies on Windows with each Linux mechanism replaced by its
row here and in the [owner](owner.md#windows), [adapter](private-adapter.md#windows)
and [consumer](consumer-launch.md#8-windows) Windows sections. These Windows
rows are specified; until the host and CLI suites run on a Windows runner, none
of them is observed.

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
