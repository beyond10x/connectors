# Design 06: personal Slack Socket Mode through Connection custody

**Status:** personal-local alpha plus hosted companion slice implemented; full M3/M4 remain open · **Date:** 2026-08-16

This document fixes the boundary for receiving Slack messages without letting Agent, a harness, a
model, configuration, or ambient process state read the Slack app token. It specializes the generic
domain model; it does not add Slack nouns to that model.

Slack's current Socket Mode contract is the vendor authority: an app-level `xapp-…` token carrying
`connections:write` authorizes `apps.connections.open`; that call returns a temporary
secret-bearing WebSocket URL; Slack sends Events API envelopes over the authenticated socket; each
envelope is acknowledged by `envelope_id`. Slack recommends acknowledging within three seconds and
periodically refreshes connections. See [Using Socket Mode](https://docs.slack.dev/apis/events-api/using-socket-mode/)
and [apps.connections.open](https://docs.slack.dev/reference/methods/apps.connections.open/).

## 1. Three similar names, three different lifetimes

| Noun | Owner | Lifetime | Secret-bearing? | Purpose |
|---|---|---:|---:|---|
| Connect Session completion endpoint | Connectors | minutes; one completion attempt | accepts one credential submission | acquire or repair vendor authorization |
| Connection | Connectors | durable across restart and reauthorization | references internal credential custody only | one authorized provider instance plus initiation and Grant policy; the personal alpha binds one Slack app |
| Endpoint | Agent/harness | task/session/product policy | never | opaque reachability projection admitted into a harness profile |

The completion endpoint is the thing “called once.” It is not the durable Connection and it is not
the harness Endpoint. The harness may later observe a value-free Connector Connection summary and
project it as its own opaque Endpoint. It must not receive the completion endpoint, app token,
credential address, Socket Mode ticket URL, or Slack transport configuration.

The resulting flow is:

```text
value-free Slack Integration policy
  -> create Connect Session over owner-authenticated Connector control socket
  -> operator submits app token once to owner-only completion socket
  -> prepared credential transaction + durable value-free Connection metadata
  -> per-Connection Slack Socket Mode Channel supervisor
  -> durable normalized Connector Event, then Slack acknowledgement
  -> generic Event receive/replay contract
  -> later: product projects Connection/Channel as opaque harness Endpoint + admitted operations
```

**2026-08-15 capability-vocabulary amendment.** The earlier `Endpoint + datasource` wording used
datasource for a generic Agent context node. Under architecture ADR 0031, the materialized Slack
Connection projects as an Endpoint and its operations target that Endpoint directly. A datasource
is added only if an owner separately declares a read-only record surface and concrete binding; a
Connection or Channel does not manufacture one.

Agent's endpoint grant remains independent from the Connector Grant. Effective access is the
intersection of both owner decisions. Provider initiation on the Connection admits Slack starting
an event path; it does not grant every inbound event. The Integration's closed `allowed_events` and
Connector Grant remain the member gate.

### The acquisition protocol is not the user journey

Connect Session and completion endpoint are protocol nouns, not onboarding instructions. The
first-party product surface owns their orchestration and shows the human only:

```text
Add Slack -> protected credential input -> Connecting -> Connected
```

It creates the session, renders the protected input directly to the human, submits it once, follows
the terminal state, and discards the acquisition references. It shows only the resulting durable
Connection. The personal-local CLI façade is `connectors setup connect slack`; it never prints a session
reference, completion endpoint, or raw protocol envelope. The lower-level verbs exist for protocol
tests and support tooling and are hidden from normal command help. A hosted or graphical product
renders the same orchestration as **Settings → Connections → Add Slack**, not as a sequence of API
calls a user must copy between terminals.

## 2. Acquisition and durable state

**2026-08-15 token-role amendment.** The initial document described only the credential already
implemented by the personal alpha. Slack's catalog now distinguishes all reviewed credential
purposes and their credential-local scope requirements.

`slack.app_token` is declared with `entry = "connect_session"` and no `env`. Channel binding
`com.slack.api:v1#socket` names that credential in its channel-level `auth` requirement and requires
`connections:write`. It is an app-level `xapp` token used only to mint Socket Mode tickets—not the
bot token that subscribes to events or calls `chat.postMessage`, not a user/admin OAuth token, and
not Slack's short-lived App Manifest configuration token. The field is distinct from generic RFC
6455 `connect.auth`: Slack authenticates a fixed HTTPS ticket-minting call before a WebSocket URL
exists. [Design 09](09-curation-and-credential-capability-admission.md) fixes the generic
credential-purpose and scope-admission boundary.

Slack defines an app-level token as representing the app across organizations, whereas an installed
bot token belongs to a workspace installation. Consequently, storing one `xapp` token on each
Connection and running one supervisor per Connection is deliberately bounded to this
single-app personal-local alpha. It is not the production multi-workspace model. Before a hosted
claim, app-level Socket Mode custody moves to the Integration, transport workers are supervised for
that Integration, Slack OAuth creates stable workspace Connections with their installation
credentials, and inbound `team_id`/`enterprise_id` attribution routes each event to the right
Connection and Grant. The generic Connection and Event contracts do not change for that move.

**2026-08-16 hosted companion amendment.** The first hosted slice now applies that ownership split
with operator-entered credentials: one Vault-held Integration `xapp` token drives one supervised
Socket Mode connection, while every workspace Connection binds a bot `xoxb` token and a delegated
user `xoxp` token validated against the same `team_id`. Bot writes and delegated-user reads are
separate admitted operations. A short-lived same-origin HTTPS Connect Session submits all three
values directly to Connectors, prepares them in Vault as one transaction, and publishes no
Connection metadata until commit. Only capability-fragment digests and terminal value-free state
exist outside SecretStore custody.

The personal-local daemon creates one Unix socket below `<state>/connect-sessions/`, mode `0600`
inside an owner-only directory. It accepts one same-UID peer, removes the socket before processing,
bounds and validates one newline-terminated app-level token, and returns only
`accepted: true|false`.
Expiry or the first completion attempt makes the endpoint terminal. The ordinary Connection
protocol cannot spell a credential field; adversarial unknown fields are refused before backend
work.

The credential lands in `connector-secrets::FileStore` under the Connection instance address. The
store is owner-protected, durable, and explicitly not encrypted. Connection metadata is a separate,
value-free JSON document. Creation uses the store's prepared transaction protocol:

This backend is development evidence only. It proves one-use acquisition, atomic mutation, restart
recovery, and value-free contracts; it does not meet the release custody posture. A released local
Connector replaces it with the OS keychain or an external secret-provider binding as fixed by
[Design 07](07-credential-custody-topologies.md), without changing the Connection or Event
contracts.

1. reserve and persist a monotonic transaction generation;
2. prepare the complete credential mutation while old values remain visible;
3. persist the value-free commit decision and proposed Connection;
4. commit the credential mutation;
5. publish the authorized Connection and reclaim the terminal transaction.

Startup completes any persisted commit decision before supervising the Connection. A crash cannot
publish metadata naming a credential mutation that was never durably decided, and the app token
never enters metadata, logs, errors, audit, config, argv, environment, catalog artifacts, or a
client result. Reauthorization-in-place is still required for general M3; the alpha creates a new
Connection but preserves its id after creation.

Authorization alone does not claim reachability. The public Connection is `authorized` while the
Socket Mode channel starts, becomes `callable` only after the WebSocket handshake succeeds, and is
`degraded` while reconnecting or after supervision stops.

## 3. Owned Socket Mode channel arm

One supervisor runs per durable Slack Connection. On each connection attempt it resolves the app
token internally, POSTs only to the fixed `https://slack.com/api/apps.connections.open` endpoint
with redirects and proxies disabled, bounds the response, and accepts only a `wss` ticket on
`slack.com` or a subdomain, with no userinfo, fragment, or non-TLS port. The temporary URL is held
only long enough to open the socket and is never rendered into an error.

The supervisor reconnects with bounded backoff and treats Slack's `disconnect` envelope as a
refresh request. A Socket Mode transport needs no per-event HMAC: provenance comes from the
pre-authenticated socket opened with the Connection credential. The Events API webhook binding
remains the alternative transport and independently uses `slack.signing_secret` verification.

For `events_api` envelopes the runtime:

1. validates the bounded transport envelope and delivery id;
2. reads only `payload.event` and matches its `type` against the Connection's closed event set;
3. drops `message` events carrying `bot_id` or `subtype` as the catalog-declared loop guard;
4. appends the normalized inner event with native provenance and syncs it durably;
5. acknowledges the Slack `envelope_id`.

Unknown or ungranted envelopes are acknowledged and discarded. An admitted event is never
acknowledged before it is durable. If the bounded event store is full or unavailable, the
supervisor does not acknowledge and Slack can redeliver. Deduplication uses Slack's `event_id` per
Connection. The outer envelope's legacy `token` and every other transport field are absent from the
stored/client payload.

The alpha verifies the `xapp` credential on the fixed Socket Mode path but does not yet acquire a
bot credential or persist general granted-scope evidence. Consequently it proves secure transport
custody and event normalization, not the future generic claim that Web/Admin operations and event
subscriptions are dynamically surfaced from a Connection's current Slack scopes.

## 4. Client and harness boundary

The alpha `b10x.connector-event.v0alpha1` contract exposes bounded `search`, `receive`, and
`replay`. It carries opaque Connection/Channel/Event references, the catalog event type, native or
polled provenance, receipt time, and the schema-bound normalized payload. It cannot carry a
credential address, completion endpoint, WebSocket URL, or Slack transport envelope.

Personal local uses long-poll receive over the owner-authenticated Unix socket. Hosted uses the
same normalized durable event model behind an exact-scope Identity-authenticated HTTPS pull; the
Socket Mode supervisor remains hosted and does not depend on a connected Agent. That proves durable
pull and replay for one provider; it does not claim M4 completion. M4 still requires general
Connection/Grant/subscription persistence, authenticated multiplexed WebSocket subscriptions,
durable signed push deliveries and retry queues, operational-event separation, retention, audit,
hosted identity, and the same event reaching both push and pull.

The future product-owned Agent adapter consumes only a read-only, grant-limited projection of
Connection summaries/channels plus admitted Event delivery; it does not receive the raw owner
control socket and is not authorized to create or inspect Connect Sessions. It projects a
Connection/Channel observation into an opaque Agent Endpoint and maps admitted events to product
input. Agent never implements Slack, opens Socket Mode, resolves `slack.app_token`, or interprets a
Connect Session completion endpoint as a reusable endpoint.

## 5. Repository shape

```text
connectors/
├── providers/slack.toml                         # app-token entry + socket channel auth
├── catalog/slack.catalog.json                   # generated declaration
├── contracts/
│   ├── connector-connection/v0alpha1/           # value-free Connection/Connect Session control
│   └── connector-event/v0alpha1/                # durable generic data-event pull/replay
├── crates/
│   ├── protocol/src/{connection,event}.rs       # strict wire readers
│   ├── server/src/local.rs                      # one owner socket, protocol dispatch
│   └── connectors-cli/
│       ├── src/slack_backend.rs                 # custody, recovery, supervisor, durable store
│       ├── src/main.rs                          # guided connect façade + internal protocol verbs
│       └── examples/slack-socket-mode.example.toml
└── docs/
    ├── design/06-personal-slack-socket-mode.md
    └── guides/connect-slack.md                   # public-facing onboarding flow
```

No Slack code, secret accessor, token source, or Socket Mode dependency is added to Agent. The
runtime closure stays in the isolated product workspace, outside the deterministic compiler graph.

**2026-08-15 composition-boundary amendment.** The repository shape above records the original
slice, not its reusable boundary. Slack custody, recovery, and supervision now live in the focused
`integration-slack` adapter. Generic Connect Session state lives in `service`, its owner-only
one-use socket transport lives in `connect-session-transport`, strict value-free configuration and
examples live in `connectors-config`, and `connectors-runtime` injects the credential capability
and installs the adapter in an exact registry. Provider-neutral guided workflows live in
`connectors-client`; `connectors-cli` only parses and renders them. Durable Slack metadata is
admitted against the current configured Grant, initiation policy, and event set before it can be
searched, described, supervised, or used for event delivery.
