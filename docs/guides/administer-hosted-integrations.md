# Administer hosted Integrations

Hosted configuration activates an Integration and supplies its non-secret policy: origins, OAuth
client IDs, callback URLs, scopes, and Grants. Secret values do not belong in TOML, Helm values,
environment variables, or CI inputs. An operator sends each value once to the running Connectors
instance, which writes it directly to the deployment's configured `SecretStore` address.

First inspect what the active configuration requires:

```shell-session
connectors admin integrations status \
  --endpoint https://connectors.example/api/connectors/v1
```

The command opens Identity's browser login by default. Identity issues a short-lived token for the
exact `urn:b10x:connectors` audience and `connectors.integrations.manage` scope. The server also
requires membership in one of its configured operator groups. The CLI keeps the Identity session
and access token in memory only.

Then write one named requirement. For example, a GitLab OAuth application secret is supplied with:

```shell-session
connectors admin credentials set gitlab oauth_client_secret \
  --endpoint https://connectors.example/api/connectors/v1 \
  --secret-stdin
```

The secret is not accepted as an argument. Omit `--secret-stdin` for a hidden terminal prompt, or
use `--secret-file PATH`; files must be regular, owned by the invoking user, have no group/other
permission bits, and remain within the size bound. An existing credential is preserved unless
`--replace` is explicit.

For non-interactive operation, `--access-token-stdin` or `--access-token-file PATH` accepts an
already-issued, short-lived Identity access token. The access token and provider secret cannot both
consume stdin in one invocation. This path does not weaken the server checks: audience, scope,
tenant, and operator-group membership are always derived from Identity.

GitLab and Slack expose `oauth_client_secret`. Jira exposes `oauth_client_secret` for delegated
user OAuth and, according to its selected shared-auth mode, either `service_oauth_client_secret` or
`service_api_token`. `admin integrations status` is the authority for the exact requirements of a
running deployment.

Every accepted write appends attempted/completed audit metadata containing the tenant, actor,
request, Integration, and logical credential name. Neither audit records nor status and write
responses contain credential bytes.

## Deployment-selected catalog destinations

For a catalog provider whose API base URL contains an endpoint variable, hosted configuration
must select that endpoint before a Connect Session is offered. The binding belongs in the private
deployment configuration. For example, an operator can enable the existing Grafana service account
token profile with this non-secret policy, alongside the normal enabled `[catalog]` configuration:

```toml
[catalog.bindings.grafana]
network = "public"

[catalog.bindings.grafana.endpoints]
origin = "https://grafana.monitoring.example"
```

The provider must also appear in `catalog.providers` when that allowlist is nonempty. Endpoint
keys must be declared by the provider's catalogue, and origin values are exact HTTPS origins
without an API path or trailing slash. Missing or invalid bindings never create a destination.
`network = "public"` is the default and refuses private DNS answers. An explicit `"operator"`
selection admits public or private addresses for the exact configured origin through the existing
post-DNS transport policy; local, link-local and reserved addresses remain refused.

The ordinary hosted Connect Session form displays the selected destination and accepts the token
directly into Connector custody. Connectors calls the catalogue's declared verification operation
before storing it. The caller cannot supply a replacement endpoint. Each connection retains its
bindings across restarts; changing or removing a binding degrades incompatible connections and
refuses their calls until the owner connects again. An existing credential is never redirected to
a newly configured host. Fixed-origin providers continue to work without bindings.
