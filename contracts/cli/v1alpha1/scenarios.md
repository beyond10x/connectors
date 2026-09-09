# Local CLI conformance expectations

These are authored expectations for the [selected contract](semantics.md), using
fictional adapter aliases, revisions and credentials. **No trace in this file is
a report of production runtime execution.** The [ESS values](../../../ess/domains/cli.yaml)
and [value fixtures](fixtures/values.json) are structurally checkable. Lifecycle,
Linux descriptor, persistence and provider observations require the real binding
and remain explicit runtime obligations.

## C01–C05 journeys

| Case | Arrange and actions | Required observations |
|---|---|---|
| C01 durable reuse | Fresh local configuration; unlocked admitted OS keyring; `forge` on-demand and `database` automatic. Init → protected terminal connect → explicit read invocation → exit CLI → restart local owner → another explicit read on the same connection. | Init starts nothing. First admitted connect starts one host, `database`, and `forge`. One protected capture and one durable credential publication. Both reads succeed on the same connection ref; restart reloads the exact still-valid custody version. No second capture, refresh/exchange, hosted login or fallback credential. The two business calls are explicit independent calls. |
| C02 local selection | A saved hosted login exists and its identity service is unavailable. Select explicit local Context config/state; list adapters/connections, then invoke an admitted local read. | Result identifies local configuration. No hosted-auth request or credential lookup, no routing substitution. Lists start/authenticate nothing. Invocation starts only the permitted local host set. The hosted outage changes no local admission or result. |
| C03 locked custody | Connection metadata and a valid active generation exist; collection is locked. Check → describe → invoke. | Check reports `keyring=locked`; metadata describe remains admitted without credential read. Invocation returns `custody_unavailable`, exit 1, stdout empty. No successful connection, plaintext fallback, new generation or alteration to the active binding. |
| C03 failed write | New connect or same-target repair reaches candidate custody; inject definitely failed or unknown durable write. | No metadata publication and no connected result. An unknown write remains unacknowledged and unusable. A still-valid repair source remains active. No replayed exchange or alternate secret store. |
| C03 crash after custody | Store candidate definitely; crash before metadata publication. Restart owner and inspect existing acquisition/connection. | Candidate is unpublished, never selected by invocation. No false connected acknowledgement. Reconciliation can retain/delete under custody rules; it cannot invent publication. Existing valid active binding remains unchanged. |
| C03 publication reply lost | Commit exact candidate metadata, then lose reply. Repeat status, not connect/exchange. | Original command returns `outcome_unknown` or interruption as appropriate. Authoritative status resolves the original ref and committed revision. One publication and at most one acquisition exchange; no replayed business operation. |
| C03 public acquisition projection | Observe the same owner's Pending, Completing, Completed, Failed and Expired records through acquisition status. | Pending/Completing report `pending` without reason/yielded connection. Completed reports `completed` with its exact yielded connection and no reason. Failed reports `failed` with safe `rejected` or `exchange_unknown` reason; Expired reports `failed` with reason `expired`. Failed observations carry no yielded connection. Protected details and internal lifecycle states are absent; observation grants no repeated exchange. |
| C04 same-target repair | Existing non-revoked connection r1 names fixture-user at target A. Capture candidate authenticated as the same identity/target, validate and publish at expected r1 while the coordinator checks its own private fence. | Safe result keeps the same connection ref and public semantic revision r1. Private generation/publication fence changes do not change effect-relevant connection meaning. Separately admitted configuration changes may advance semantic revision under shared rules. No identity/target transfer. Still-valid reuse works after both CLI and owner restart. |
| C04 changed identity | Repeat repair with another principal/account or target B; alternatively change profile ownership or expected revision before publication. | `identity_mismatch` for changed external semantics or `lifecycle_conflict` for a stale fence. No candidate publication. Valid r1 remains usable; failed candidate evidence does not globally poison it. Creating a new connection is a separate admitted action. |
| C04 revoke races | Begin repair; local revoke commits before repair publication; lose revoke reply and restart owner. | Status reads terminal revocation; repair publication and later dispatch refuse. Configuration reload/static activation cannot resurrect it. Provider outcome remains `not_requested`; no provider-revoke or keyring-delete success is inferred. |
| C05 JSON sources | Describe an admitted `project.get` schema and revision; invoke from business JSON file and separately stdin. | Exactly one decode of the UTF-8 text carrier yields wire JSON input. Handler sees the selected operation/schema/revision. Valid results appear in one stdout success envelope; progress is suppressed in JSON mode. |
| C05 wrong input | Independently try malformed JSON, duplicate key, trailing document, wrong field, wrong type and byte/depth excess. | Distinct safe `invalid_input` usage failure, exit 2; stdout empty; provider dispatch count 0. Carrier structure alone does not imply the embedded provider JSON passes its schema. No input snippets in errors. |
| C05 refusal classes | Independently select absent operation, stale description/schema identity, denied permission and unreachable service. | Respectively `not_found`, `stale_description`, `forbidden`/`not_granted`, `unavailable`; exit 1. Stale selection/permission refuses before business dispatch. No schema substitution, route fallback or automatic replay. |

## Startup and process ownership traces

Each row starts from the same explicit two-entry configuration unless stated
otherwise. Count host starts, child launches and provider-auth calls at the real
ports; counting recording-handler calls alone cannot decide these cases.

| Case | Trace | Required observation |
|---|---|---|
| L01 pure inventory | Host absent → setup check → adapters list/describe/status → operations list/describe → connections list/describe/status. | Host starts 0, child launches 0, provider-auth calls 0. Absent metadata/description is an explicit unavailable result; known cached facts are marked stale. |
| L01 cached connection page | Read a cached connection page containing a formerly ready member, once with a known validity deadline and once without one. | Both pages carry `source: cached, stale: true`, and retain observation time. Any known page deadline bounds every member; absence makes no freshness claim. An all-authoritative page with expired/unknown member validity is also stale. No ready summary substitutes for current viability or admission. |
| L02 startup definition | Host absent → admitted `connections connect --adapter forge`. | One host start. `database` automatic and selected `forge` on-demand each launch once. No other demand entry launches. Automatic work begins at host startup, not at every CLI invocation. |
| L03 concurrent coalescing | Two admitted invoke/connect callers select `forge` with the same configuration while host is starting. | One host and one `forge` incarnation. Both await its outcome using independent original deadlines and current admission. No duplicate launch or inherited grant. |
| L04 autostart isolation | `database` automatic fails readiness; `forge` becomes ready. | `database` records failure. An admitted `forge` request can proceed; a `database` request receives its failure and never routes to `forge`. |
| L05 identity mismatch | Launched `forge` describes wrong instance/adapter/version/configuration revision. | `readiness_mismatch`, no ready publication or business dispatch. Only the newly owned child is stopped. No matching-by-name fallback. |
| L06 exact stop | Read host H1/child P1/config r1; concurrently replace with H2/P2 or r2; submit stop for old coordinates. | `incarnation_mismatch` or configuration `lifecycle_conflict`; signals to P2/foreign processes 0. Numeric PID reuse cannot redirect stop. |
| L07 stop suppression | Explicitly stop automatic `database`; start unrelated `forge` request; restart host. | Database stays suppressed after the unrelated action and owner restart. Automatic sweep never silently resumes it. An explicit admitted connect/repair/invoke targeting `database` is required to resume once. |
| L08 crash policy | `forge` terminates unexpectedly; wait past every default timer. | State failed; background restart count 0 (`restart=never`). A later explicit admitted request may launch exactly one new incarnation. |
| L09 changed config | P1 active at r1; change executable/config to r2 and invoke. | Refuse `lifecycle_conflict`; no simultaneous r2 incarnation. Exact stop/drain must settle P1 before explicit resumed launch. |
| L10 capacity/deadline | 65 entries, five concurrent candidates, or readiness/stop expiry. | 65-entry configuration refuses before launch. At most four concurrent launches; waiting uses bounded original deadlines. Timeout reports truthful failure/remaining observation and never signals a foreign process. |

## Protected entry, output and invalid configuration

The sole secret fixture values are visibly fictional sentinels. Assert that their
bytes, source paths, actionable continuations and private custody locators do not
appear in stdout, stderr, logs or ordinary serialized management requests.

| Case | Input/control | Required result |
|---|---|---|
| P01 terminal | Trusted controlling TTY; hidden capture, then success / validation failure / interruption. | Echo disabled before capture and restored on all paths; no credential argv value; no secret echo or JSON-stream prompt. |
| P02 file | Open owner-only regular file 0400/0600, all path components non-symlink, safe parents; replace pathname after verified open. | Bounded read from the original verified descriptor only. No reopen/TOCTOU substitution. |
| P03 stdin | Protected already-open file or deliberately inherited admitted anonymous pipe. | Bounded private document capture without argv/environment content. Same document validation as file input. |
| P04 unsafe sources | Symlink component, foreign owner, broad mode, extra hard link, named FIFO/socket, terminal under stdin mode, unknown pipe producer boundary. | Refuse before capture/publication/provider work. No fallback source. A failed fstat/open error string is not echoed. |
| P05 source conflict | Select file and stdin, stdin and hidden terminal, or omit every protected source. | Exit 2 for conflict/missing required explicit source; capture count 0. A selected terminal without trusted TTY fails safely, with no fallback. |
| P06 independent channels | Protected credential source and business source contend for stdin in a future combined binding. | Refuse before consuming either channel. Current selected commands separate these actions. |
| P07 protected JSON | Invalid UTF-8/JSON, duplicate/unknown fields, trailing documents, depth >64 or >64 KiB. | Bounded safe refusal; no secret-bearing parse context or buffer serialization. JSON whitespace is permitted, secret string contents are unchanged. |
| P08 parser redaction | Unknown command/flag after a protected path or sentinel-bearing malformed argument; request JSON output before or after subcommand. | Stable JSON error stderr, stdout empty, exit 2, no argv/path/value echo. |
| P09 interruption | Interrupt entry or waiting after possible dispatch/publication. | Exit 130 and safe interruption envelope when interruption is handled; restore TTY. No rollback/retry claim. If OS termination prevents an envelope, only observed process exit is evidence. |
| P10 error typing | Return unknown application error code or wrong typed success/error data from a recording handler. | Generator adapter refuses with a stable internal error; unvalidated handler output never reaches stdout. This is parser/codec evidence only. |
| T01 valid TOML | [Valid fixture](fixtures/config-valid.toml), with omitted forge startup/restart. | Effective forge on-demand/never, database automatic/never. No process is started merely by parsing. |
| T02 invalid TOML | [Invalid fixture](fixtures/config-invalid.toml). | `invalid_configuration`, exit 2, launches 0, fictional credential field/value absent from diagnostic. Each independent invalid field should additionally be isolated in runtime tests. |
| T03 identity/path/keys | Duplicate alias or instance, unknown top-level/entry key, unsupported format, relative artifact, missing digest, unresolved default, reserved management collision. | Reject whole configuration without discovery, build, authentication or partial launch. |

The generated CLI's portable default Sources implementation may prove bounded
UTF-8 acquisition and hidden TTY selection. Its acceptance does not prove P02–P04,
the narrower 64 KiB/depth/profile bounds, or actual Linux keyring/owner behavior;
the selected application Sources implementation must prove those separately.
