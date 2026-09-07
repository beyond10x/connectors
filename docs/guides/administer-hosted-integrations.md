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

## Supply a catalog token without a browser form

The same principal-owned Connect Session can read a token from an owner-only file. Sign in to the
exact public Connector API base through normal Identity once:

```shell-session
connectors session login https://connectors.example/api/connectors/v1
connectors setup connect grafana --target hosted \
  --auth-profile grafana.service_account_token \
  --credential-file /private/grafana-token --label Monitoring
```

This path supports the hosted catalogue's declared profiles with one secret credential entry,
including service-account tokens and API keys. It uses the active saved Identity login and creates
a Connection owned by that principal. It does not create a Grafana service account, mint a token,
or install a shared administrative credential. OAuth consent and native flows requiring multiple
fields retain their declared acquisition routes. Local setup remains the default without
`--target hosted`; local configuration, provider settings, network overrides and instruction files
cannot be combined with hosted token setup.

The CLI checks the selected profile against its pinned catalogue before reading the credential
file or creating a session. Unknown profiles and incompatible acquisition shapes are refused;
profiles introduced by a newer catalogue need a matching CLI release. The running Connector
still decides whether that profile is currently admitted by its deployment.

The file must be regular, owned by the invoking user, have no group/other permission bits, contain
nonempty UTF-8, and be at most 8192 bytes. Final symlinks are refused and the same opened handle is
checked and read. Neither the credential nor the one-use completion capability appears in output.
The client sends it once to the exact returned session route beneath the selected API base, with
redirects and automatic retries disabled. The existing server performs provider verification,
credential custody, owner checks and endpoint pinning. Success requires both completed session
status and a callable principal-owned Connection with the requested provider and profile. If
submission cannot be confirmed, inspect hosted Connections before starting again: the first
submission may already have stored the credential.

For automation that already has a short-lived Identity bearer, the reusable Rust
`HostedClient::connect_with_credential_file` accepts that bearer, its ordinary `OwnerContext`, the
existing `ConnectSessionCreateRequest`, and the private file path. The bearer needs
`connectors.connections.self` and `connectors.catalog.read`; the server derives its actual tenant
and owner from Identity. No desktop login or OS keyring is required by this client method. Its caller
must select a declared token profile with one secret entry; the reusable protocol client embeds no
catalogue and the server remains responsible for admission and credential verification. For
callers already implementing session creation and status checks,
`HostedClient::complete_connect_session` submits an issued token session directly. It acknowledges
submission only and must be followed by the normal owner-scoped status/description checks.

Private development CAs use the platform trust store or `SSL_CERT_FILE` pointing to the retained
PEM CA bundle. Certificate and hostname verification remain enabled. Use the same trusted public
API base for login and acquisition; a returned completion origin or API prefix change is refused.
