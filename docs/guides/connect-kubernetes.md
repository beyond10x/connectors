# Connect local Kubernetes contexts

Enable Kubernetes policy in the personal-local Connector configuration; the complete example is
[`kubernetes-discovery.example.toml`](../../crates/connectors-cli/examples/kubernetes-discovery.example.toml).
No kubeconfig path or credential is copied into this file. The Connector uses the user's standard
merged kubeconfig privately.

Start the Connector daemon as usual. The normal Zwirn flow first lists detected contexts:

```console
zwirn connect kubernetes
```

This step is passive. It does not contact a cluster and cannot execute an auth helper. If more than
one context exists, choose the exact one:

```console
zwirn connect kubernetes --context dev-cluster
```

That second command is the active boundary. It authenticates through the Connector, verifies the
API-server identity view, checks read permission, and lists only bounded Service metadata in the
configured namespace scope. It prints recognized Grafana, Prometheus, Loki, and Alertmanager
observations. It never returns tokens, certificates, keys, API-server URLs, or kubeconfig user
bindings.

Then select exactly one supported Service:

```console
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

`connectors connect kubernetes` provides the lower-level diagnostic activation flow, and the
generic `connection observations` / `connection materialize` methods expose the same value-free
contract. Direct in-cluster satellite Connections remain the preferred zero-user-credential
topology for deployed environments.
