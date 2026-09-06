# `connectors` personal-local commands and daemon

This nested workspace builds a thin product frontend without feature-unifying the runtime closure
into the deterministic catalog compiler. Reusable wire behavior lives in `connectors-client` and
daemon assembly lives in `connectors-runtime`; this package owns only top-level clap parsing and
dispatch. Guided input and presentation live in `connectors-console`.

```text
clean-room client
  -> owner-only Unix socket
  -> generic search / describe / invoke / session methods
  -> catalog + current owner snapshot + Connection + Grant + approval
  -> admitted sip.dial plan
  -> supervised sipx telephony <-> neutral VoiceSession <-> RTVBP application
```

`connectors serve` needs no config and safely returns an empty operation search result. A strict
deployment file enables the platform's development-gated `sip.dial` member with Asterisk as the
configured example peer:

```sh
cargo run --manifest-path crates/connectors-cli/Cargo.toml --locked -- \
  serve --config crates/connectors-config/examples/asterisk-dev.example.toml
```

The state root defaults to `$XDG_STATE_HOME/b10x/connectors`, or
`$HOME/.local/state/b10x/connectors`. It must be absolute, outside the current working tree,
real, owner-owned, and mode `0700`. The socket is mode `0600`. The daemon writes only opaque
lifecycle facts to `connector-audit.jsonl`; caller input, SIP identities, credentials, and media are
absent. A held state lock prevents a second daemon from unlinking a live socket; after a crash, a
stale socket is removed only when it is an owner-owned Unix socket.

The deployment file must be a real owner-owned file and cannot be writable by group or other. Its
size and field set are bounded. It carries references and routes but no credential value.

## Run a bounded command without a daemon

`operation search`, `operation describe`, bounded `operation invoke`, and Connection metadata reads
compose the configured personal runtime for one command when `connectors.sock` is absent. The normal
config and state defaults work, as do explicit `--config` and `--state-root` paths. A saved hosted login
does not change this local target; `--target hosted` selects the hosted service explicitly.

Describe an operation, then use its returned `description_ref` and selected `connection_ref` in a
separate invocation. Both commands use the same catalog, owner, credential custody and Grant checks.
They publish no control socket and leave no daemon or background child behind. Concurrent commands
refuse while another process owns the state root; rerun after that command finishes, or start
`connectors serve local` for concurrent clients.

An existing daemon is used directly. An unsafe or stale socket, invalid response, reset connection,
or uncertain invocation outcome never causes a local retry. Restart a stale daemon explicitly with
`connectors serve local --config <file> --state-root <dir>`.

All event commands, SIP/browser session operations, Connection activation, materialization, and
Connect Session flows require that persistent daemon. Kubernetes activated Connections and workload
pagination cursors also belong to its process generation. These commands refuse with the daemon
command instead of returning a handle that dies on exit. `connectors inspect doctor` names this
distinction.

## Repair authentication for an intended operation

Operation commands default to `b10x.connector-operation.v0alpha3`. An admitted authentication need is a
nonzero-exit `authentication_required` refusal with `attempt: not_attempted` and
`next_action: start_trusted_remediation`. It does not retry the operation or renew an Identity
login. Structured output retains the closed authentication need, attempt and next action;
daemon messages, labels, references and private instructions are absent from this projection.
Existing v2 rate-limit details and `outcome_unknown` remain distinct refusals. For interoperability with a v2 daemon, select `connectors operation --protocol-version v2 …`;
`--protocol-version v3` explicitly selects the default. The selection applies to operation
commands, including one-shot calls, and does not change Connection remediation v2. The CLI
sends exactly the selected contract without negotiation, fallback or resend. Unknown versions
are refused before input or socket access.

For a configured personal OAuth Connection, select the intended operation and Connection together:

```sh
connectors setup connect \
  --operation '<operation-ref>' --connection '<connection-ref>' \
  --input-file ./intended-input.json \
  --config /absolute/path/connectors.toml --state-root /absolute/owner-only/state/root
```

This mode requires a persistent personal daemon. Provider, label, profile, credential, instance,
route and enrollment overrides are not accepted with the bound selectors. Input may instead use
`--input-json` or `--input -`; each source is bounded to 64 KiB. Hosted bound acquisition remains
unsupported. The hosted operation client still uses its existing invoke scope and renews only for
Identity HTTP 401, never for a provider authentication HTTP 409.

Human URLs and device codes go only to the controlling terminal, or to a new owner-only file
selected with `--instruction-file` in a private directory. They never enter command output.
The private file is cleared on success, refusal, deadline or client cancellation; stopping the
client does not send a protocol cancellation, and the daemon owns bounded session cleanup.

The client polls the bound session, acknowledges readiness once, obtains fresh Connection and
operation descriptions, validates the original input against the current schema without external
schema retrieval, and stops. Its public success is `ready: true`, `attempt: not_attempted`, and
`next_action: explicit_invoke`. A separate explicit invocation still requires a current description
and ordinary grant admission. An acknowledgement response loss or changed schema never triggers
an automatic invoke or a second acknowledgement.

## Administer a hosted Integration

Hosted GitLab, Slack, and Jira registrations are activated by value-free deployment configuration.
Inspect their exact credential requirements, then supply a missing value directly to the running
service:

```sh
connectors admin integrations status --endpoint https://connectors.example/api/connectors/v1
connectors admin credentials set gitlab oauth_client_secret \
  --endpoint https://connectors.example/api/connectors/v1 \
  --secret-stdin
```

The CLI uses Identity browser PKCE by default and requests only the short-lived
`connectors.integrations.manage` access token. Use `--no-browser` to print the URL, or provide an
already-issued token through `--access-token-stdin` or an owner-only `--access-token-file`. Secret
values are accepted only by hidden prompt, stdin, or an owner-only file; they are never argv.

## Connect a local Kubernetes Service

The end-user path lives in Zwirn because selecting a Service also creates the separate Harness
Endpoint Grant:

```sh
zwirn connect kubernetes
zwirn connect kubernetes --context dev-cluster
zwirn connect kubernetes --context dev-cluster --service monitoring/prometheus
```

The first command reads context labels only and contacts no cluster. Explicit context activation
may authenticate and run an admitted kubeconfig helper; explicit Service selection materializes
only a recognized Prometheus, Loki, or Alertmanager target with an independent Connector Grant.
Invocation uses the API server's exact `services/proxy` route after fresh, resource-specific
`services` and `services/proxy` RBAC checks and a Service UID/port/provider revalidation. Zwirn
stores only absolute local Connector paths and opaque Connection references, then
compiles a session-scoped Endpoint Grant on startup. It never reads kubeconfig or a provider
credential. `zwirn connect reset` revokes those local selections without deleting Connector-owned
Connections.

The lower-level `connectors connect kubernetes` and generic `connection` commands remain available
for Connector diagnostics and contract clients. See the
[Kubernetes guide](../../docs/guides/connect-kubernetes.md).

## Query monitoring through Grafana

The same daemon can connect an HTTPS Grafana instance, discover its configured Prometheus, Loki,
and Alertmanager data sources, and expose their target Provider operations through Grafana's fixed
data-source proxy route. The guided action is:

```sh
connectors connect grafana \
  --config /absolute/path/connectors.toml \
  --state-root /absolute/owner-only/state/root
```

It asks for the Grafana service-account token through hidden terminal input, verifies the instance,
and materializes only recognized targets with configured independent Grants. The token is kept in
an injected in-memory store until durable monitoring metadata exists, so restart requires the
guided action again; it never enters configuration or metadata.
See the [Grafana guide](../../docs/guides/connect-grafana.md) for CLI invocation and Harness use.

## Receive Slack messages

The normal user journey is one action: **Add Slack**. The personal-local CLI presents the same
journey as `connectors connect slack`; Connect Session references, completion endpoints, and raw
protocol responses are internal and never shown.

An operator prepares the Slack app once: enable Socket Mode; generate an app-level token with
`connections:write`; subscribe to `app_mention` and, if desired, `message.channels`; add
`app_mentions:read` and the corresponding history scope; install the app; and add it to the
channels it should observe. Slack's
[Socket Mode guide](https://docs.slack.dev/apis/events-api/using-socket-mode/) is the vendor source.

The deployed product keeps the Connector service running. From this checkout, start that service
with the value-free development policy:

```sh
cargo run --manifest-path crates/connectors-cli/Cargo.toml --locked -- \
  serve --config crates/connectors-config/examples/slack-socket-mode.example.toml
```

Then run the guided action in an operator terminal:

```sh
cargo run --manifest-path crates/connectors-cli/Cargo.toml --locked -- \
  connect slack \
  --config crates/connectors-config/examples/slack-socket-mode.example.toml \
  --state-root /absolute/owner-only/state/root \
  --label "Development Slack"
```

The command asks one hidden-input question and waits until Slack is reachable. The token goes from
that terminal directly to the one-use Connector receiver. It is never accepted as an argument,
config value, environment variable, harness callback, or model result. On an installed personal
deployment, the default config and state locations reduce the command to:

```sh
connectors connect slack
```

Successful output is intentionally human and value-free:

```text
Connect Slack
Input is hidden and sent only to the local Connector.
Slack app token: [hidden]

Slack is connected and ready to receive messages.
Connection: Development Slack
Events: app_mention, message.channels
```

Connect Session endpoints remain protocol surfaces for reusable clients; the product CLI does not
expose raw session verbs as an onboarding flow. See the
[public-facing guide](../../docs/guides/connect-slack.md) for the product wording.

After connection, the product delivers admitted messages to the harness automatically. Until that
read-only harness adapter lands, developers can inspect the generic Event contract with `event
search`, `event receive`, and `event replay`; those protocol verbs are not end-user setup steps.

The channel supervisor resolves `slack.app_token` from the owner-only Connector credential store,
calls Slack's fixed `apps.connections.open` endpoint, validates the returned `wss://*.slack.com`
ticket destination, and reconnects when Slack refreshes the socket. It stores the normalized inner
event durably before acknowledging the Slack envelope. The outer Socket Mode envelope, its legacy
`token`, the app token, and the secret-bearing WebSocket ticket URL never enter the event store or a
client result. Message events carrying `bot_id` or `subtype` are acknowledged and dropped as the
declared loop guard.

The first personal-local event store is bounded at 10,000 events and 64 MiB. Reaching the bound
stops acknowledgement and forces Slack redelivery; it does not discard an admitted message while
claiming delivery. Retention/compaction and hosted posture remain M4 work.

## Voice development path

The authority key file is deployment-owned, absolute, non-symlink, owner-owned, and inaccessible to
group/other. It contains exactly 32 raw bytes or 64 lowercase hex digits. Startup prints the
corresponding verifying key; the RTVBP application endpoint must trust that exact issuer/key before
a dial can establish. The file can be created outside a checkout with an operating-system random
source, for example:

```sh
umask 077
openssl rand 32 > /absolute/owner-only/state/voice-authority.key
```

The caller never supplies a SIP URI, host, port, transport, bind address, application endpoint,
tenant, Grant, or credential. It supplies only the configured alias:

```json
{
  "protocol": "b10x.connector-operation.v0alpha1",
  "request_id": "request-3",
  "context": {
    "tenant_id": "tenant-development",
    "agent_id": "harness-development",
    "agent_revision": 1,
    "authority_snapshot_id": "snapshot-development-1",
    "authority_snapshot_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  },
  "request": {
    "method": "invoke",
    "params": {
      "operation_ref": "sip.dial",
      "connection_ref": "connection-asterisk-development",
      "description_ref": "description-sha256-...",
      "input": {"target": "asterisk-dev"}
    }
  }
}
```

Obtain `description_ref` from `describe` immediately before invocation. `sip.dial` returns only
after the SIP dialog and authenticated RTVBP application channel are both established. Its
`execution_ref` is then used with `session_status`, `session_terminate`, or `session_reconcile`.
After a daemon process restart, an execution absent from the new generation reconciles as
`outcome_unknown`; the process cannot pretend it still owns an old socket task.

The example file uses documentation-only addresses. Replace every route, aperture, identity,
snapshot, and key path with reviewed deployment facts before use. This is a development profile
while sipx remains pinned to a release candidate; it is not a stable or hosted support claim.
