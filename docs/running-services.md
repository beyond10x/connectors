# Run connector services

These instructions describe the implemented first-slice runtime. Run commands from
the repository root after [building the workspace](development.md). For a complete
disposable environment, follow the [live acceptance recipe](live-e2e.md).

Connector, instance and connection are the terms of the [vocabulary](../README.md#vocabulary).

Copy the matching file from `examples/`, choose a unique instance/listener, configure
the permitted resources, and bind private credential files or environment references.
Credential files must be regular files owned by the running user with mode `0600`.
Credentials never belong in operation input. Config changes require a restart.
Credential file replacement is resolved on subsequent requests, including gateway
downstream credentials. Environment references reflect the process environment;
changing a parent shell's environment cannot update an already running service.

```sh
target/debug/connectors-kubernetes --config kubernetes.yaml
target/debug/connectors-sql --config sql.yaml
```

A caller presents the service credential as a bearer token, reads the current descriptor with
`GET /v1/describe` and names its revision in `POST /v1/invoke`:

```sh
token="authorization: Bearer $(cat service.secret)"
revision=$(curl -s -H "$token" http://127.0.0.1:7102/v1/describe | jq -r .revision)
jq -n --arg revision "$revision" --slurpfile input examples/requests/kubernetes-endpoints.json \
  '{version: "v1alpha1", request_id: "discover-1", operation: "endpoints.discover",
    revision: $revision, input: $input[0]}' \
  | curl -s -H "$token" -H 'content-type: application/json' --data @- \
    http://127.0.0.1:7102/v1/invoke
```

The `connectors` command does not call a standalone service: its `describe`, `invoke --endpoint`
and `serve` commands were removed in 0.60.0. The local CLI reaches a provider through a configured
instance; `connections connect <connector>/<instance> --endpoint <url>` saves the provider
address there.

GitLab has no standalone service: the [catalog provider](local-catalog-provider.md)
serves it under the local CLI.

A federation host puts several services behind one endpoint. It is the library
`connectors_host::federation`, which a Rust program embeds and serves with
`connectors_host::server::serve`; `examples/federation.yaml` is its configuration, and
[Federate services](../website/docs/guides/federate-adapter-services.md) shows the program's
shape. Through it the operation above is `kubernetes__endpoints.discover`. Source prefixes
distinguish instances; the host does not interpret provider data. It supports one federation hop
and never retries a provider request.

## Supported surfaces

| Service | Operations | External binding |
|---|---|---|
| Kubernetes | `resources.list`, `endpoints.discover`, optionally `hosts.discover`, optionally `helm_releases.history`/`.status`/`.values`/`.manifest` | Kubernetes API; namespace/kind restrictions; bearer token and CA; Helm release reads need their own `helm_release_reads` selection |
| SQL | `schema.list`, `query.read`, `database.list`, `table.list`, `table.describe`, `index.list` | PostgreSQL or MySQL; explicit role or user and database; read-only transactions or sessions and deadlines |

SQL uses the `postgresql-native-text` profile: column names and native type names
accompany rows of strings or JSON null. Exact numeric, array, timestamp, and JSON
representations are preserved without numeric coercion. Parameters are text or null
and PostgreSQL resolves their types. Use explicit SQL casts where inference needs
help. The database role's grants govern accessible schemas and functions. Each request
opens a fresh connection and read-only transaction, initialized with `search_path = public, pg_catalog`;
qualify tables in other schemas (for example `reporting.sales`). There is no pool in
this bounded local profile. Fresh connections resolve the current credential each
time and discard session state; connection setup therefore contributes to latency.

Endpoint discovery reports observations with provenance, readiness and reachability.
It does not create a connection or dial the discovered address. Explicitly configure
an SQL binding and its credential after selecting an authorized reachable endpoint.

Service listeners speak HTTP; use loopback locally or a trusted TLS-terminating ingress
for remote deployments. The 32-operation limit and 20-second execution deadline
apply after request admission and reading; configure connection and request-read
limits at the ingress. Upstream TLS certificate validation is enabled by default.
For HTTP providers and SQL, `ca_file` replaces public roots with that PEM bundle.
Federation downstream clients currently use built-in public roots and have no
private-CA setting; local downstreams use explicitly permitted loopback HTTP.
`allow_plaintext` is an explicit local-test configuration, not a TLS verification bypass.

SQL keeps its execution deadline independent of caller-controlled database settings.
Timeouts and dropped invocations trigger a bounded cancellation/cleanup attempt;
the request budget includes cleanup. Remote termination cannot be guaranteed after
a network partition or process loss. See the [service contract](../contracts/service/v1alpha1/semantics.md)
for the exact budgets. Live timeout, caller-drop and TLS checks run in the SQL adapter's ignored tests.

No service in this slice advertises OAuth acquisition, writes, durable events, process
execution, or media sessions. Those remain separate contracts in the full design.
