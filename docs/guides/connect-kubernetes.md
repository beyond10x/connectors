# Discover services from Kubernetes

Install Connectors, then enable Kubernetes using your existing local kubeconfig:

```bash
connectors setup connect kubernetes
connectors endpoint list
```

Setup reads the standard merged kubeconfig, selects its current context, saves an explicit
namespace policy, and starts the local daemon. The default namespace is the context's namespace,
or `default`. With several contexts and no current context, pass `--context`. Setup then
checks the selected cluster and refreshes its endpoint inventory.

For a repeatable development setup, select the context, namespaces, and providers explicitly:

```bash
connectors setup connect kubernetes --context development \
  --namespace monitoring --namespace applications \
  --read-provider loki --read-provider postgresql --read-provider asterisk
```

Repeat `--namespace` and `--read-provider` as needed. `--all-namespaces` is a separate explicit
choice. A previously configured empty namespace scope is not silently widened. Contexts that run
an external authentication helper require terminal consent or `--allow-exec-auth`. The selected
context and consent are retained for daemon restarts. Credentials stay in kubeconfig.

Setup writes a private backup before changing an existing configuration. It restarts the daemon
when its configuration changes. Add `--config` and `--state-root` consistently when using paths
other than the defaults.

## Inspect the inventory

```bash
connectors endpoint refresh
connectors endpoint list --query loki --limit 25
connectors endpoint show --endpoint-ref "$endpoint_ref"
```

Copy `endpoint_ref` from the list result. References are opaque and identify a specific discovered
interface. A Kubernetes Service can expose several interfaces, one for each advertised port.
Endpoint records include provenance, provider classification, route, and readiness reasons.
Unknown services remain visible even when no installed driver can call them.

List results have an opaque `next_cursor`; pass it as `--cursor` until the listing is complete.
Use `--source` to restrict list or refresh to one source reference. Refresh reconciles
the inventory against Kubernetes and reports warnings when the source cannot be read. Cached
records remain distinguishable from current, ready endpoints.

Recognition and permission are separate. A known Loki, Asterisk ARI, or database interface still
needs a compatible installed operation, admitted provider policy, a usable route, and any required
credential binding. An AMI TCP port can be inventoried without implying an AMI driver.

## Bind an interface and call an operation

Use a binding when discovery needs provider or credential metadata. Bindings contain references,
not secret values. For example, a PostgreSQL Service can select an existing Secret:

```bash
connectors endpoint bind --endpoint-ref "$endpoint_ref" --provider postgresql \
  --database application --tls required \
  --credential-secret applications/database-access \
  --credential-key username=username --credential-key password=password
```

The named Secret must be in the admitted namespace scope. Its credential fields must match the
provider's declared authentication profile. HTTP bindings can additionally use `--scheme` and
`--base-path`; an explicit externally routed destination uses `--direct-address`.

Describe the intended operation against that endpoint, then invoke it with the returned
description reference:

```bash
connectors operation describe --operation postgresql-query --endpoint-ref "$endpoint_ref"
connectors operation invoke --operation postgresql-query --endpoint-ref "$endpoint_ref" \
  --description-ref "$description_ref" \
  --input-json '{"statement":"select 1"}'
```

Use the returned input schema for the operation's actual arguments. The same target form works
for installed HTTP operations such as Loki or Asterisk ARI. Invoke accepts exactly one of
`--endpoint-ref` and a direct `--connection`.

At invocation, the daemon rechecks the source, current endpoint identity, provider policy, route,
and credential reference. It fetches the named credential through Kubernetes and passes it
privately to the driver. Endpoint descriptions and results never carry credential values.
Crossplane connection details can contribute endpoint metadata and Secret references when their
resources are readable. Discovery does not enumerate Secret contents.

Local routes can use Kubernetes port forwarding owned by the daemon. Externally reachable routes
require their explicit routing policy. A hosted deployment uses its configured cluster access
and network placement; it does not gain access to your laptop's kubeconfig.

## Receive declared events

For an installed streaming channel, subscribe by its declared binding. For example, an admitted
Asterisk ARI endpoint can use its installed channel parameters:

```bash
connectors event subscribe --endpoint-ref "$endpoint_ref" \
  --channel-binding "$channel_binding" --parameters-json '{"app":"developer"}'
connectors event receive --channel "$channel_ref"
connectors event unsubscribe --subscription-ref "$subscription_ref"
```

Use the channel binding declared by the provider. Subscribe returns `subscription_ref` and a
channel; use that channel's reference with receive. Parameters must match the declaration and
cannot override credentials or the provider URL. The daemon owns the stream until unsubscribe or
shutdown. These commands also accept `--target hosted` for an admitted hosted endpoint.

## Manage the daemon

```bash
connectors daemon status
connectors daemon stop
connectors daemon start
```

Status identifies the running process, version, and configuration. Provider access uses
the daemon; `--help` and `inspect providers` remain available offline. Use `serve local` for a
foreground process or an existing supervisor.

For a hosted deployment with Kubernetes enabled, the endpoint and operation contracts are the
same. Select it explicitly after login:

```bash
connectors endpoint --target hosted list
connectors operation --target hosted describe --operation postgresql-query --endpoint-ref "$endpoint_ref"
```

The hosted deployment supplies its own namespace policy, credentials, and caller authority.
The old public `connection candidates`, `activate`, `observations`, and `materialize` workflow
is retired. Use setup to configure a source and the endpoint API to inspect and bind its interfaces.
