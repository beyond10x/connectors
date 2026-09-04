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

## Which token kind you hold, and which base URL it needs

Atlassian issues three things an operator can reasonably call "a Jira token", and they are not
interchangeable. `connectors providers jira` lists four declared mechanisms and says nothing about
which one a value in your clipboard is, so start here.

| what you have | mechanism | how it travels | base URL that accepts it |
|---|---|---|---|
| An API token you minted at `id.atlassian.com` under your own account | `jira.api_token` | Basic `email:token` | the Cloud gateway |
| An API token minted **by a service account** (organization-issued, no person behind it) | `jira.service_api_token` | Bearer | the Cloud gateway |
| An OAuth 2.0 (3LO) access token a person granted | `jira.user_oauth` | Bearer | the Cloud gateway |

The base URL is the same for all three, and it is **not** your `*.atlassian.net` site:

```text
https://api.atlassian.com/ex/jira/<cloud id>
```

**The site host is the trap.** It answers, so nothing looks wrong. Measured on one tenant on
2026-09-04 with a service-account API token, `GET /rest/api/3/project/search` returned HTTP 200 and
`total: 0` against `https://<site>.atlassian.net`, and HTTP 200 with `total: 40` against the
gateway. A connector pointed at the site would report an empty Jira rather than a refusal anybody
could act on, so this catalogue does not offer that route at all — `jira` templates a cloud id and
nothing else.

Your cloud id is the UUID your own site reports at `https://<your-site>.atlassian.net/_edge/tenant_info`.

## Personal-local configuration

The personal placement serves Jira generically from the catalogue: there is no `[jira]` section and
no per-provider Rust, only a `[[catalog]]` row and a credential in the store.

A service-account token, which is what an integration that will be deployed should carry:

```shell-session
connectors connect jira \
  --as jira.service_api_token \
  --set cloud_id=11111111-2222-3333-4444-555555555555 \
  --credential-file ~/.config/b10x/atlassian/jira.token
```

A personal API token instead, which is Basic and therefore has two halves — the token goes to the
store, the account email is configuration:

```shell-session
connectors connect jira \
  --as jira.api_token \
  --set cloud_id=11111111-2222-3333-4444-555555555555 \
  --set email=you@example.com
```

Either writes a row like this, and never a credential value:

```toml
[[catalog]]
provider = "jira"
grant_ref = "grant:jira:local"
initiation = "platform"
allow_writes = false
credential = "jira.service_api_token"
operator_approved = true

[catalog.endpoints]
cloud_id = "11111111-2222-3333-4444-555555555555"

# Only for the Basic mechanism. Keyed by the credential the account name joins, not by the
# configuration field's own name.
[catalog.usernames]
"jira.api_token" = "you@example.com"
```

`connectors auth status` reports both halves. A Basic credential whose account name is missing reads
`stored-without-user-half` rather than `stored`, because a token that cannot be joined is a token
that cannot be sent.

**Order is selection.** A host takes the first declared mechanism whose credentials all resolve, and
`jira.service_api_token` is declared first. Storing a personal token beside a service-account one
therefore leaves the service account in charge, which is the intended default for anything that will
be deployed.

Adding a second credential to a Jira row that already exists stores the value and does **not**
rewrite the row; `connect` appends whole blocks rather than editing one an operator may have
commented. If a `[catalog.usernames]` entry was needed, the command says so by name.

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
