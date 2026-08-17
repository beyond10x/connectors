# Connect Grafana and use its monitoring data sources

This development slice connects one Grafana instance and makes recognized Prometheus, Loki, and
Alertmanager data sources callable without giving B10x their backend URLs or credentials.
Requests still travel through Grafana. “Prometheus Connection” describes the PromQL API contract;
it does not mean a direct Prometheus network route.

## What you provide

Create an owner-only config from
`crates/connectors-config/examples/grafana-federation.example.toml`. Set the HTTPS Grafana origin and
the independent Connector Grant references already approved for Grafana and each target Provider.
The file contains no token, password, backend URL, Grafana data-source UID, or secret path.

Create a Grafana service account with only the read permissions needed for data-source listing,
dashboard inspection, and proxying the selected monitoring queries. Keep its token out of shell
arguments, environment variables, config, and Harness input.

## Connect once per daemon generation

Start the Connector:

```sh
cargo run --manifest-path crates/connectors-cli/Cargo.toml --locked -- \
  serve --config crates/connectors-config/examples/grafana-federation.example.toml \
  --state-root /absolute/owner-only/state/root
```

In another terminal, use the guided flow:

```sh
cargo run --manifest-path crates/connectors-cli/Cargo.toml --locked -- \
  connect grafana \
  --config crates/connectors-config/examples/grafana-federation.example.toml \
  --state-root /absolute/owner-only/state/root \
  --label "Infrastructure Grafana"
```

The hidden prompt sends the token directly to a one-use Connector socket. The Connector verifies
Grafana, discovers its data sources, and materializes recognized targets covered by the configured
target Grants. It prints only safe labels and opaque Connection references. Unsupported plug-in
types stay visible as a count and never fall through to generic proxy access.

The token is currently retained only in daemon memory. A restart requires this guided action again;
the alpha does not write a plaintext credential file. OS-keychain and satellite-local write-only
custody are subsequent persistence backends.

## Inspect or invoke from the CLI

List the callable surface:

```sh
connectors operation search \
  --config /absolute/path/connectors.toml \
  --state-root /absolute/owner-only/state/root \
  --query prometheus
```

Describe `prometheus-query-range`, then use the returned `description_ref` and one returned
`connection_ref` in an invocation:

```sh
connectors operation describe \
  --config /absolute/path/connectors.toml \
  --state-root /absolute/owner-only/state/root \
  --operation prometheus-query-range

connectors operation invoke \
  --config /absolute/path/connectors.toml \
  --state-root /absolute/owner-only/state/root \
  --operation prometheus-query-range \
  --connection 'connection:prometheus:…' \
  --description-ref 'description-sha256-…' \
  --input-json '{"query":"up","start":"1723676400","end":"1723676700","step":"30s"}'
```

Description leases deliberately go stale when the catalog, owner snapshot, or available Connection
set changes. Describe again instead of caching them.

## Use it from the Harness

Build the `agent-harness` binary from the Agent repository and add the Connector capability to the
run request:

```json
{"agent":{"capabilities":["agent.connector.operation"]}}
```

Then select the owner-only socket:

```sh
agent-harness run \
  --adapter codex \
  --request /absolute/path/run-request.json \
  --cwd /absolute/path/workspace \
  --connectors-socket /absolute/owner-only/state/root/connectors.sock
```

The run request's tenant, Agent identity/revision, and authority snapshot must exactly match the
Connector's current owner configuration. The model receives only generic search, describe, and
invoke tools. When one operation has exactly one admitted Connection it may omit `connection_ref`;
with several, it chooses only among opaque references returned by describe. The Agent adapter
retains the Connector description lease and never reads a credential, backend URL, Grafana UID, or
proxy route.
