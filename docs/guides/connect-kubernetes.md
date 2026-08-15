# Connect local Kubernetes contexts

Enable Kubernetes policy in the personal-local Connector configuration; the complete example is
[`kubernetes-discovery.example.toml`](../../crates/connectors-cli/examples/kubernetes-discovery.example.toml).
No kubeconfig path or credential is copied into this file. The Connector uses the user's standard
merged kubeconfig privately.

Start the daemon as usual, then list detected contexts:

```console
connectors connect kubernetes
```

This step is passive. It does not contact a cluster and cannot execute an auth helper. If more than
one context exists, choose the exact one:

```console
connectors connect kubernetes --context dev-cluster
```

That second command is the active boundary. It authenticates through the Connector, verifies the
API-server identity view, checks read permission, and lists only bounded Service metadata in the
configured namespace scope. It prints recognized Grafana, Prometheus, Loki, and Alertmanager
observations. It never returns tokens, certificates, keys, API-server URLs, or kubeconfig user
bindings.

Contexts using an exec or legacy auth-provider plugin are refused by default because kubeconfig can
name local credential helpers. Review the context and set `allow_exec_auth = true` only if running
that helper is intended. The helper still runs only during explicit activation. API-server routes
must be canonical HTTPS; ambient and kubeconfig HTTP proxies are not used.

The current release detects and records monitoring Services but does not yet make a target behind a
local cluster callable: Kubernetes-mediated Service transport is fail-closed until its route adapter
is installed. Direct in-cluster satellite Connections remain the preferred zero-user-credential
topology for deployed environments.
