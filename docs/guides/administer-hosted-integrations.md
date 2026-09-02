# Administer hosted Integrations

Hosted configuration activates an Integration and supplies its non-secret policy: origins, OAuth
client IDs, callback URLs, scopes, and Grants. Secret values do not belong in TOML, Helm values,
environment variables, or CI inputs. An operator sends each value once to the running Connectors
instance, which writes it directly to the deployment's configured `SecretStore` address.

First inspect what the active configuration requires:

```console
connectors admin integrations status \
  --endpoint https://connectors.example/api/connectors/v1
```

The command opens Identity's browser login by default. Identity issues a short-lived token for the
exact `urn:b10x:connectors` audience and `connectors.integrations.manage` scope. The server also
requires membership in one of its configured operator groups. The CLI keeps the Identity session
and access token in memory only.

Then write one named requirement. For example, a GitLab OAuth application secret is supplied with:

```console
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
