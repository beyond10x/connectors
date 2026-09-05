# Connect local Kubernetes contexts

Enable Kubernetes policy in the personal-local Connector configuration; the complete example is
[`kubernetes-discovery.example.toml`](../../crates/connectors-config/examples/kubernetes-discovery.example.toml).
No kubeconfig path or credential is copied into this file. The Connector uses the user's standard
merged kubeconfig privately.

Start the Connector daemon as usual. The normal Zwirn flow first lists detected contexts:

```bash
zwirn connect kubernetes
```

This step is passive. It does not contact a cluster and cannot execute an auth helper. If more than
one context exists, choose the exact one:

```bash
zwirn connect kubernetes --context dev-cluster
```

That second command is the active boundary. It authenticates through the Connector, verifies the
API-server identity view, checks read permission, and lists only bounded Service metadata in the
configured namespace scope. It prints recognized Grafana, Prometheus, Loki, and Alertmanager
observations. It never returns tokens, certificates, keys, API-server URLs, or kubeconfig user
bindings.

Then select exactly one supported Service:

```bash
zwirn connect kubernetes --context dev-cluster --service monitoring/prometheus
```

This materializes a child Prometheus, Loki, or Alertmanager Connection only when the Connector has
an independent target Grant. Zwirn persists only opaque Connection references and compiles the
separate, session-scoped Harness Endpoint Grant on its next start. Provider calls stay inside the
Connector: it rechecks `get` on the exact Kubernetes Service and its `services/proxy` subresource,
then verifies that the Service UID, recognized provider, and selected port still match the sealed
observation. It permits only the catalog operation's fixed GET path through the API server. There
is no arbitrary host, port, path, or generic proxy operation. Grafana Services remain observations
in this slice because they need a separate credential acquisition step.

Contexts using an exec or legacy auth-provider plugin are refused by default because kubeconfig can
name local credential helpers. Review the context and set `allow_exec_auth = true` only if running
that helper is intended. The helper still runs only during explicit activation. API-server routes
must be canonical HTTPS; ambient and kubeconfig HTTP proxies are not used.

`connectors setup connect kubernetes` provides the lower-level diagnostic activation flow, and the
generic `connection observations` / `connection materialize` methods expose the same value-free
contract. Direct in-cluster satellite Connections remain the preferred zero-user-credential
topology for deployed environments.

## Read an activated cluster's inventory

The operation CLI can list admitted namespaces and Deployments without a deployment name. Keep
the opaque Connection reference returned by activation, and obtain each operation's description
lease before invoking it:

```bash
connectors operation search --query kubernetes
connectors operation describe --operation kubernetes.namespace.list
connectors operation invoke --operation kubernetes.namespace.list \
  --connection "$connection_ref" --description-ref "$namespace_description_ref" \
  --input-json '{}'

connectors operation describe --operation kubernetes.workload.list
connectors operation invoke --operation kubernetes.workload.list \
  --connection "$connection_ref" --description-ref "$workload_description_ref" \
  --input-json '{"namespace":"monitoring","limit":25}'
```

Set `connection_ref` to the activated Connection and the two description variables to the
`description_ref` values returned by their respective describe calls. Every activated Connection
is offered by these reads; choose the Connection explicitly for each invocation.

`kubernetes.namespace.list` returns `connection_ref` and the namespaces admitted by the local
configuration. It makes no cluster request. An empty configured namespace list admits no inventory
namespaces; it does not expand to all namespaces. Kubernetes RBAC is checked by the API server when
`kubernetes.workload.list` reads Deployments in an admitted namespace.

The workload result contains `connection_ref`, `namespace`, and `deployments`. Each deployment
has a name, container names and images from its Pod template, and desired and ready replica counts.
Images are available even when replicas are zero. These operations do not read Pods or Secrets.

`limit` defaults to 25 and accepts 1–100. A call fetches at most eight upstream pages of at most
five Deployments each, so a page can be shorter than the requested limit. When `next_cursor` is
present, pass it as `cursor` in the next workload input with the same Connection and namespace.
Cursors are opaque, single use, and expire after five minutes; restarting the daemon also discards
them. If a cursor expires, restart the listing. A result that exceeds the response byte bound is
refused; retry from the start with a smaller limit. The existing `kubernetes.workloads` datasource
retains its compact record format.
