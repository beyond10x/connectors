# Connect Jira Cloud

The hosted Jira Integration deliberately creates two different Connections. They share provider
catalog metadata, but never share authority or credentials.

`jira.organization_read` is one deployment-owned, tenant-scoped app Connection. It exposes only the
`jira.issues` datasource for the exact configured Jira Cloud site and sorted project allowlist. The
projection contains bounded issue fields and excludes raw/custom fields, email addresses, comments,
attachments, worklogs, changelogs, avatars, and unknown provider members. This Connection admits no
operation and cannot write.

`jira.oauth_user` is a principal-owned Atlassian OAuth 2.0 (3LO) Connection. Zwirn starts the setup,
but the Connector owns the callback, OAuth state, token exchange, token refresh, and Vault custody.
Connectors accepts the result only when the selected accessible resource exactly matches the
configured Cloud ID and `*.atlassian.net` origin, the required read/write scopes are present, the
Atlassian account is active, and `/me` returns an email that exactly matches the signed-in Identity
email after case normalization. Refresh must preserve the same account and site.

The delegated Connection can read and can invoke only the curated Jira operations. Every create,
summary edit, comment add/edit, transition, and issue link requires fresh approval evidence tied to
the current operation-description lease. Deletes, attachments, arbitrary Jira `fields`/`update`
objects, description edits, and automatic transitions are not admitted. A failed delegated
credential never falls back to the organization credential.

## Hosted configuration

Jira is value-free and disabled by default. One exact site and at least one sorted project key are
required:

```toml
[jira]
cloud_id = "11111111-2222-3333-4444-555555555555"
site_origin = "https://example.atlassian.net"
public_origin = "https://api.example.com/api/connectors/v1"
allowed_project_keys = ["OPS", "SUPPORT"]
shared_auth = "service_oauth"
service_oauth_client_id = "SERVICE_CLIENT_ID"
user_oauth_client_id = "USER_3LO_CLIENT_ID"
oauth_redirect_uri = "https://api.example.com/api/connectors/v1/oauth/jira/callback"
organization_read_grant_ref = "grant:jira:organization-read"
user_grant_ref = "grant:jira:delegated-user"
initiation = "platform"
connect_session_ttl_seconds = 300
refresh_skew_seconds = 300
```

The user OAuth app needs `read:jira-work`, `write:jira-work`, `read:me`, and `offline_access`, with
the exact callback above. The recommended shared credential is an Atlassian service-account OAuth
2.0 client-credentials app limited to Jira read access. `shared_auth = "service_api_token"` is an
explicit fallback for a service-account API token; in that mode `service_oauth_client_id` must be
absent.

An operator supplies only the selected fixed credentials through the hosted administrative API:

```shell-session
connectors admin credentials set jira oauth_client_secret --endpoint URL --secret-stdin
connectors admin credentials set jira service_oauth_client_secret --endpoint URL --secret-stdin
```

Use `service_api_token` instead of `service_oauth_client_secret` when that shared-auth mode is
selected; `connectors admin integrations status --endpoint URL` reports the exact active
requirements. Supply either the service OAuth secret or the service API token, never both. The user OAuth app
secret is always required for delegated setup. Delegated access and rotating refresh tokens are
stored under Connector-owned per-Connection instance paths and never reach Zwirn, Agent, Helm, or
the provider catalog.
