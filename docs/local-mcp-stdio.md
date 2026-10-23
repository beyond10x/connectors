# Local MCP stdio: admitted read-only tools

The production `connectors server --transport stdio` entry implements a bounded
inbound tools slice for `2026-07-28` and `2025-11-25`. It uses the generated native
launch grammar and the existing protected local owner, registry, credential
custody, audited metadata and bounded read dispatch. Resources, prompts,
subscriptions, mutations and outbound MCP are not implemented by this entry.
The full inbound runtime story remains active.

The entry runs on Linux only: its local lease is measured on `CLOCK_BOOTTIME`
(below). On macOS and Windows a valid launch exits 1 with the stderr diagnostic
`mcp_unavailable` before any protocol byte; `--help` still prints the launch
grammar.

## Configure and launch

Use the existing local setup and connection workflow first. The selected instance
must have a pinned executable, a cached admitted bootstrap, explicit
`connectors-private/3`, operation/profile grants, and receiver-owned operation
curation. Curation lives beside the config as `CONFIG.operation-curation.json`,
with format `connectors-operation-curation/1`; its adapter entry binds the exact
executable selection and bootstrap digest. See the
[metadata contract](../contracts/service/local-operation-curation.md) for declarations.

Place the native projection at `CONFIG.mcp-stdio.json`, protected like the local
config (owner-only regular file under admitted directories). For example:

```json
{
  "format": "connectors-mcp-local/1",
  "limits": {
    "frame_octets": 1048576,
    "response_octets": 33554432,
    "concurrent_requests": 1,
    "request_milliseconds": 120000
  },
  "exposures": [{
    "adapter_alias": "selected-adapter",
    "operation_ref": "selected-read",
    "connection_ref": "existing-connection-reference",
    "families": ["tools"],
    "enabled": true
  }]
}
```

This slice accepts one adapter alias and Connection coordinate per process, with
at least one configured exposure. It has one request slot for `tools/list` and
`tools/call`; the configured concurrency is a ceiling. One that arrives while the
slot is taken is refused with the Connectors error code `capacity`. An operation
is advertised only when its complete declared profile and response fit the
configured ceilings:

| Realization / profile | Execution / provider ceiling | Input / result bytes |
|---|---|---|
| `implemented` / `resource` | 20,000 / 15,000 ms | 65,536 / 4,194,304 |
| `generic` / `generic-http` or `generic-http-page` | 40,000 / 30,000 ms | 262,144 / 4,194,304 |

Input and output schemas must be inline, without schema references, identifiers or
anchors; this slice omits those that require reference rebasing.

Both require the declared 5,000 ms connect ceiling, read effect, no approval,
`none` or `natural` idempotency, no semantic effects and only optional `network`
effects. Missing or incompatible declarations, disabled exposures and ungranted
operations are omitted. Metadata-policy failure refuses the list. Listing never
probes a credential or invokes the provider.

```console
connectors --config /absolute/config.toml --state-dir /absolute/state server --transport stdio
```

The parent owns stdin/stdout. Newline-delimited MCP is the only stdout data;
launch and terminal failures use a payload-free stderr diagnostic. `--output`,
credential-source flags and non-stdio transports are refused.

## List, then invoke

For the primary revision, each request carries
`params._meta["io.modelcontextprotocol/protocolVersion"] = "2026-07-28"` and an
object `params._meta["io.modelcontextprotocol/clientCapabilities"]`.
`server/discover` advertises tools only. `tools/list` returns canonical names,
wrapped schemas, safe metadata and the projection revision. List pages contain
at most 32 tools; cursors are bound to the current owner, selection and revision.

Call the returned name with `params.arguments.input` and copy its
`_meta["io.beyond10x.connectors/revision"]` into the call's `params._meta`.
The configured exposure selects the Connection; MCP input cannot select another.
The result's `structuredContent` is the complete owner service response, including
request identity, audit reference/status, business result or error. The text
content represents that same response. Large numbers remain exact. The advertised
output schema describes this service envelope, with the operation schema under
`result`. `resultType: "complete"` describes MCP completion, not dataset completeness.

For `2025-11-25`, first send `initialize` with that version, capabilities and
clientInfo, then `notifications/initialized`. Subsequent calls use the same names,
input wrapper and projection revision but omit the primary-only metadata and
result fields. No operation is admitted before initialization finishes; between
the `initialize` response and `notifications/initialized` only `ping` is answered.

## Local lease and cancellation bounds

The supervisor issues and checks its own lease on one correlated local timeline.
A startup wall-time value labels that timeline; it is not independent UTC evidence.
Elapsed time uses Linux `CLOCK_BOOTTIME`; a bracketed `CLOCK_MONOTONIC` comparison
refuses suspend/clock discontinuity before more data is sent. The clock resolution
and sample bracket must each fit 1 ms, with 2 ms observation uncertainty.

Configuration must supply a valid `roughtime-quorum/1` `approval_clock` table with
the existing explicit deployment rate assumption (at most 10,000 ppm); the retired
`roughtime-clock/1` shape is refused ([local clock](local-clock.md)). This local lease does
not acquire a Roughtime sample or change the approval clock's UTC semantics.
A 1,900 ms lease plus 2 ms resolution at the maximum 1% slow rate remains below
2,000 real ms. Renewal re-admits the same owner incarnation and local selection
and checks the old lease first. A stopped process cannot renew after expiry.
Neither input traffic nor an MCP progress notification supplies lease authority.

A request and the writing of its reply are bounded by the request's own budget:
the configured `request_milliseconds` from its first byte, and for a call the
operation's declared execution and provider budgets. The lease left at dispatch
does not shorten them. The supervisor renews the lease while the session is
ready, so a read that outlasts one lease, or a reply the client drains slowly, is
still answered. An unrenewed lease still ends the session at its expiry.

Cancellation suppresses the matching reply, does not replay a call and frees the
request slot at once, so the next list or call is admitted. A reply already partly
written when its cancellation arrives is finished rather than cut off, and the
session stays usable. The owner has no cancellation of its own: owner work that
a cancelled call already started runs out within its original deadline. While two
cancelled calls' owner work is still running, a new list or call is refused with
`capacity`.

Closing standard input ends the session (`remote_hangup`), and local admission
stops: no request reaches the owner after it. A partial input frame is discarded
and nothing more is read. A request already read that has not yet reached the
owner is answered with the error code `unavailable`. Replies to requests already
at the owner are still written, each within its own deadline and the lease live
at the close, which is not renewed; the process then exits successfully. A reply the
lease cannot carry is discarded and the exit is unsuccessful. Loss during partial
output terminates the process. Local worker teardown, cancelled calls included,
finishes within five seconds of the terminal, and unjoined work is not recorded
as released. Owner work still running then is not cancelled either: it runs out
at the owner within its own deadline, after the process has exited. These
guarantees make no claim about remote rollback.

## Verification boundary

`apps/connectors/tests/local_mcp_server.rs` launches the actual production binary,
its owner and a pinned Rust adapter child inside a jail against the read-only
`mounted_secret` custody binding (`apps/connectors/tests/support/mounted.rs`); the
child reads through the host's HTTP transport from a loopback provider the test
serves and counts. It checks both revisions, exact payload preservation including
numbers no `f64` holds, zero provider requests for lists and refusals, disabled,
hidden, approval-required and ungranted operations, stdout carrying only JSON-RPC
frames, payload-free launch refusals, withdrawal, malformed, oversized and
truncated frames, cancellation, a reply still written after standard input
closes, a call refused because standard input closed before it reached the
owner, and stopped-process lease expiry:

```console
cargo test --locked --offline -p connectors --test local_mcp_server
```

No scenario is ignored; the file compiles on Linux only. These process bindings
cover the read-tools subset of the AEP acceptance scenarios; they are not a
complete ESS runtime conformance report, independent review or full MCP release.
