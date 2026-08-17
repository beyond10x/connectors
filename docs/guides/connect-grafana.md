# Connect Grafana and use its monitoring data sources

This integration connects one Grafana instance and makes recognized Prometheus, Loki, and
Alertmanager data sources callable without giving Agent or UI consumers their backend URLs,
Grafana data-source UIDs, or credentials.
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

Grafana service-account tokens inherit the service account's permissions. Use the `Viewer` basic
role as the baseline, or narrower Enterprise RBAC plus data-source permissions when available. The
Connector still exposes only its declared read operations; no annotation, silence, dashboard, or
configuration write is routed.

## Hosted deployment-managed federation

Hosted mode has no Grafana Connect Session. Cloud supplies one exact HTTPS origin, a stable parent
Connection, sorted `dev`/`sre` read groups, and an exact datasource allowlist. Each allowlist row
contains the provider kind and SHA-256 of the expected Grafana UID, never the UID itself. At startup
the Integration reads `/api/datasources`, matches the digests, seals the resolved UID in memory, and
publishes stable child Connections. A missing or changed source remains degraded and cannot fall
through to a generic proxy route.

The token is resolved from Vault as the tenant-bound
`com.grafana.api/default/service_account_token` credential. The deployment refuses Grafana without
Vault, an empty read-group set, an empty target set, an unsafe origin, or a malformed/duplicate
binding. The checked-in development example is synthetic; real mappings belong in the ignored
`.b10x/dev.local.json` deployment config.

Identity-verified `dev` and `sre` members can search, describe, and invoke the read surface. The
operator group remains an independent override and is merged with ordinary memberships. Other
tenant members see no monitoring Connections or operations.

Hosted invocation audit lines are retained in the Connectors PostgreSQL state store. They contain
opaque Connection and audit references plus actor/operation/outcome metadata, never query inputs,
provider responses, datasource UIDs, or credentials.

All results are projections rather than raw provider responses:

- datasource inventory contains only a display name, provider kind, health, and callability;
- dashboards contain bounded titles, tags, and panel kinds, never query definitions;
- Prometheus keeps only bounded structural labels and samples;
- Loki returns at most 1,000 lines, caps each line at 8 KiB, and redacts common credential, bearer,
  JWT, and private-key patterns;
- Alertmanager returns bounded status, selected labels, timestamps, and redacted summaries.

CloudWatch and other Grafana plugins remain inventory-only. Arbitrary text can still contain
sensitive business data that pattern redaction cannot recognize, so callers must treat log and
annotation text as operationally sensitive.

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

In personal-local mode the token is currently retained only in daemon memory. A restart requires
this guided action again; the alpha does not write a plaintext credential file. OS-keychain and
satellite-local write-only custody are subsequent persistence backends.

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
