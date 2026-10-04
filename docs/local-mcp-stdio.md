# Local MCP stdio: admitted read-only tools

The production `connectors server --transport stdio` entry implements a bounded
inbound tools slice for `2026-07-28` and `2025-11-25`. It uses the generated native
launch grammar and the existing protected local owner, registry, credential
custody, audited metadata and bounded read dispatch. Resources, prompts,
subscriptions, mutations and outbound MCP are not implemented by this entry.
The full inbound runtime story remains active.

## Configure and launch

Use the existing local setup and connection workflow first. The selected adapter
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
at least one configured exposure. It supports one active owner request; the
configured concurrency is a ceiling. An operation is advertised only when its
complete declared profile and response fit the configured ceilings:

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
result fields. No operation is admitted before initialization finishes.

## Local lease and cancellation bounds

The supervisor issues and checks its own lease on one correlated local timeline.
A startup wall-time value labels that timeline; it is not independent UTC evidence.
Elapsed time uses Linux `CLOCK_BOOTTIME`; a bracketed `CLOCK_MONOTONIC` comparison
refuses suspend/clock discontinuity before more data is sent. The clock resolution
and sample bracket must each fit 1 ms, with 2 ms observation uncertainty.

Configuration must supply a valid `approval_clock` selection with the existing
explicit deployment rate assumption (at most 10,000 ppm). This local lease does
not acquire a Roughtime sample or change the approval clock's UTC semantics.
A 1,900 ms lease plus 2 ms resolution at the maximum 1% slow rate remains below
2,000 real ms. Renewal re-admits the same owner incarnation and local selection
and checks the old lease first. A stopped process cannot renew after expiry.
Neither input traffic nor an MCP progress notification supplies lease authority.

Each owner request is shortened to the lease available when dispatched; this
initial slice can therefore terminate a session when a slow read exhausts that
short budget, even if the declared profile permits a longer execution. Renewal
keeps idle sessions usable; it does not extend a dispatched read's budget.

Cancellation suppresses the matching reply and does not replay a call. Already
started owner work remains bounded by its original deadline. EOF discards a
partial input frame; loss during partial output terminates the process. Local
worker teardown is bounded to five seconds and unjoined work is not recorded as
released. These guarantees make no claim about remote rollback.

## Verification boundary

`apps/connectors/tests/local_mcp_server.rs` launches the actual production binary,
owner and a pinned Rust provider with disposable GNOME Secret Service. Its explicit
lane checks both revisions, exact payload/audit preservation, zero provider calls
for lists and refusals, withdrawal, malformed/oversized/truncated frames,
cancellation and stopped-process lease expiry:

```console
cargo test --locked --offline -p connectors --test local_mcp_server -- --include-ignored --test-threads=1
```

The three custody-dependent scenarios are classified as disposable by the ignored
suite runner. They are not run implicitly by the default gate. These process
bindings cover the read-tools subset of the AEP acceptance scenarios; they are
not a complete ESS runtime conformance report, independent review or full MCP release.
