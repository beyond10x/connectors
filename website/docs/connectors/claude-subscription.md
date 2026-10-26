---
title: Claude subscription
sidebar_position: 13.7
description: Sign your own Claude plan in once through the Claude Code sign-in, and hand a program a current access token with connections token.
lede: A pinned instance per profile signs in to your own Claude plan through the Claude Code sign-in, keeps the grant in your platform's secret store and issues a current access token on request, refreshing it within five minutes of its expiry.
source: adapters/catalog/providers/claude-subscription/connector.json, adapters/catalog/providers/claude-subscription/operations.json, adapters/catalog/providers/claude-subscription/README.md, adapters/catalog/tests/local_runtime/claude_subscription.rs, adapters/catalog/tests/local_runtime/claude_subscription_journey.rs
connectors: "claude-subscription"
---

# Claude subscription

The `claude-subscription` connector of the provider Anthropic signs in to a Claude plan through the
sign-in the Claude Code binary uses: an OAuth 2.0 authorization code grant with PKCE at
`https://claude.com/cai/oauth/authorize`, its tokens from `https://platform.claude.com/v1/oauth/token`.
A program then asks `connections token` for a current access token. The words connector, instance,
connection and operation are those of the
[README vocabulary](https://github.com/beyond10x/connectors#vocabulary).

It is built for one person's use of their own Claude plan, on their own machine. Read the
provider's terms before you connect.

:::caution[The provider's terms]

Anthropic's
[Legal and compliance page](https://code.claude.com/docs/en/legal-and-compliance#authentication-and-credential-use),
section "Authentication and credential use", read 2026-10-25, verbatim:

> developers may not collect, store, or intermediate Claude.ai credentials or session tokens — sign-in to a Claude account must complete through Anthropic's own flow.

The same page permits an end user "signing in to the unmodified Claude Code binary with their own
Claude subscription". This connector is not that binary: it stores the tokens of the account that
signs in, in your platform's secret store. Connect only your own Claude plan, and never run it for
another person's account.

:::

Connectors ships no client id for this sign-in. Each profile declares `client_id: null`, so no
instance can be derived from the declaration: an instance is pinned, and its native configuration
names the client id you are entitled to use in `oauth2_client_id`. A configuration without one is
refused as `InvalidConfiguration`. Commands that need a Claude account are shown without output;
every behaviour stated here was run against a local fake authorization server only.

## What it reads and writes

| Profile | Redirect | Reads | Writes |
|---|---|---|---|
| `claude.subscription` | loopback, any port: `http://localhost:<port>/callback` | `account.profile` | none |
| `claude.subscription-pasted` | the provider's page `https://platform.claude.com/oauth/code/callback`, whose `code#state` line you paste | `account.profile` | none |

Both profiles request `user:profile` and `user:inference` and require both. `account.profile`
reads `GET https://api.anthropic.com/api/oauth/profile`, the same read the connection's identity
check makes; the connection's subject is the answer's `account.uuid`, kind `claude.account`. The
connector selects no inference operation: what you do with the access token is your program's.

## Prerequisites at the provider

- A Claude plan you hold yourself.
- The client id you are entitled to use with this sign-in, for `oauth2_client_id`. Connectors does
  not ship or print one.
- For `claude.subscription`, a browser on the machine that runs `connectors`: the provider
  redirects it to a listener on `127.0.0.1`. Without one, use `claude.subscription-pasted`.

## Enable the connector and add an instance

Write the native configuration of the profile you use. `bundle_directory` and `operations_file`
are absolute paths into your copy of the repository's `adapters/catalog`, and the file is mode
`600`. For the loopback redirect, `/absolute/path/claude.json`:

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "claude.mine",
  "provider": "claude-subscription",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://api.anthropic.com",
  "oauth2_client_id": "REPLACE_WITH_YOUR_CLIENT_ID",
  "auth": {
    "profile": "claude.subscription",
    "scheme": "oauth2_grant",
    "header": "Authorization",
    "bearer": true,
    "label": "Claude subscription sign-in",
    "identity": {
      "path": "api/oauth/profile",
      "kind": "claude.account",
      "subject_pointer": "/account/uuid"
    },
    "minimum_scopes": [
      "user:inference",
      "user:profile"
    ],
    "oauth2": {
      "authorize_url": "https://claude.com/cai/oauth/authorize",
      "token_url": "https://platform.claude.com/v1/oauth/token",
      "device_authorization_url": null,
      "client_id": null,
      "client_kind": "public",
      "request_encoding": "json",
      "refresh_encoding": "json",
      "redirect_kind": "loopback_any_port",
      "redirect_uri": "http://localhost/callback",
      "authorize_parameters": [
        {
          "name": "code",
          "value": "true"
        }
      ],
      "requested_scopes": [
        "user:profile",
        "user:inference"
      ],
      "required_scopes": [
        "user:profile",
        "user:inference"
      ],
      "rotation": "rotating",
      "refresh_lead_seconds": 300,
      "grant_types": [
        "authorization_code"
      ],
      "sources": [
        "Claude Code 2.1.310 bundled JavaScript, read 2026-10-25: CLAUDE_AI_AUTHORIZE_URL, TOKEN_URL",
        "Claude Code 2.1.310: function wVr (authorize extra code=true), async function Tqo (JSON exchange carrying state), async function koe (JSON refresh), function rC (refresh 300000 ms before expiry)"
      ],
      "state_in_exchange": true,
      "scope_in_refresh": true
    }
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/claude-subscription/operations.json"
}
```

An instance serves the one profile its configuration names, so the pasted code is a second
instance, `claude.pasted`, whose `auth` is the profile `claude.subscription-pasted`. Write it to
`/absolute/path/claude-pasted.json`:

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "claude.pasted",
  "provider": "claude-subscription",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://api.anthropic.com",
  "oauth2_client_id": "REPLACE_WITH_YOUR_CLIENT_ID",
  "auth": {
    "profile": "claude.subscription-pasted",
    "scheme": "oauth2_grant",
    "header": "Authorization",
    "bearer": true,
    "label": "Claude subscription sign-in, pasted code",
    "identity": {
      "path": "api/oauth/profile",
      "kind": "claude.account",
      "subject_pointer": "/account/uuid"
    },
    "minimum_scopes": [
      "user:inference",
      "user:profile"
    ],
    "oauth2": {
      "authorize_url": "https://claude.com/cai/oauth/authorize",
      "token_url": "https://platform.claude.com/v1/oauth/token",
      "device_authorization_url": null,
      "client_id": null,
      "client_kind": "public",
      "request_encoding": "json",
      "refresh_encoding": "json",
      "redirect_kind": "pasted_code",
      "redirect_uri": "https://platform.claude.com/oauth/code/callback",
      "authorize_parameters": [
        {
          "name": "code",
          "value": "true"
        }
      ],
      "requested_scopes": [
        "user:profile",
        "user:inference"
      ],
      "required_scopes": [
        "user:profile",
        "user:inference"
      ],
      "rotation": "rotating",
      "refresh_lead_seconds": 300,
      "grant_types": [
        "authorization_code"
      ],
      "sources": [
        "Claude Code 2.1.310 bundled JavaScript, read 2026-10-25: CLAUDE_AI_AUTHORIZE_URL, TOKEN_URL, MANUAL_REDIRECT_URL",
        "Claude Code 2.1.310: function wVr (authorize extra code=true), async function Tqo (JSON exchange carrying state), async function koe (JSON refresh), function rC (refresh 300000 ms before expiry), the pasted line split on # into code and state"
      ],
      "state_in_exchange": true,
      "scope_in_refresh": true
    }
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/claude-subscription/operations.json"
}
```

Print each file's bootstrap for its configuration revision:

```sh
connectors-catalog-provider --local-config /absolute/path/claude.json --print-local-bootstrap
connectors-catalog-provider --local-config /absolute/path/claude-pasted.json --print-local-bootstrap
```

Then pin each instance, its executable and that executable's SHA-256 in the CLI configuration.
Keep the table of the instance whose profile you use; `mine` permits the loopback profile and
`pasted` the pasted code:

```toml
[connectors.claude-subscription]
enabled = true

[connectors.claude-subscription.instances.mine]
alias = "claude"
instance_id = "claude.mine"
adapter_id = "catalog"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/2"
startup = "on-demand"
restart = "never"

[connectors.claude-subscription.instances.mine.permissions]
profiles = ["claude.subscription"]
operations = ["account.profile"]

[connectors.claude-subscription.instances.mine.executable]
path = "/absolute/path/connectors-catalog-provider"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/claude.json"]

[connectors.claude-subscription.instances.pasted]
alias = "claude-pasted"
instance_id = "claude.pasted"
adapter_id = "catalog"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/2"
startup = "on-demand"
restart = "never"

[connectors.claude-subscription.instances.pasted.permissions]
profiles = ["claude.subscription-pasted"]
operations = ["account.profile"]

[connectors.claude-subscription.instances.pasted.executable]
path = "/absolute/path/connectors-catalog-provider"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/claude-pasted.json"]
```

## Connect

```sh
connectors connections connect --connector claude-subscription --instance mine \
  --profile claude.subscription
```

The grant runs in the CLI process. It prints the authorization address on your terminal; open it
in a browser on the same machine and sign in to your Claude account. The provider redirects to the
listener on `127.0.0.1`, and Connectors exchanges the code.

For the pasted code, connect the `pasted` instance:

```sh
connectors connections connect --connector claude-subscription --instance pasted \
  --profile claude.subscription-pasted
```

The provider's page shows a `code#state` line instead of redirecting; paste it at the hidden
prompt. A line whose `state` is not the flow's own, or with more than one `#`, is refused. The
commands below name `--instance mine`; use `--instance pasted` for a pasted-code connection.

The connection is Live once the profile read with the new access token names the account and the
grant holds both scopes. The tokens go into your platform's secret store only: no output, no
configuration file and no state file holds them.

## A first read

Take `--schema` and `--revision` from `operations describe --connector claude-subscription
--instance mine --operation account.profile`, then:

```sh
connectors operations invoke --connector claude-subscription --instance mine \
  --connection "$connection" --operation account.profile \
  --schema "$schema" --revision "$revision" --input-json '{}'
```

The access token itself, for a program of yours:

```sh
connectors connections token --connector claude-subscription --instance mine
connectors connections token --connector claude-subscription --instance mine --reveal
```

Without `--reveal` it prints what was issued, never the token; with it, standard output is the
token and one newline. Within 300 seconds of the access token's expiry the owner refreshes it
first, with a JSON request; an answer without a refresh token keeps the one held.

## A first guarded write

None: the connector selects no write.

## Reconnect when the connection is Stale

A refresh token the provider refuses makes the connection `stale`, and `connections token` answers
`repair_connection`. Sign in again on the same connection:

```sh
connectors connections list --connector claude-subscription --instance mine
connectors connections reconnect --connector claude-subscription --instance mine \
  --expected-revision "<revision from the list>"
```

`reconnect` runs the same grant as `connect` and replaces the stored tokens on the same
connection. The Claude journeys do not run it; the generic grant journey
(`oauth2_grant_reconnect_replaces_the_entry_on_the_same_connection`) does.

## Limits

- Verified against a local fake authorization server only
  (`adapters/catalog/tests/local_runtime/claude_subscription_journey.rs`); nothing has run against
  a live Claude account. The shapes it answers are those the Claude Code 2.1.310 binary sends and
  reads, as `oauth2.sources` cites them.
- The Claude Code binary sends `anthropic-beta: oauth-2025-04-20` beside the access token.
  Connectors sends no such header, so whether `account.profile` and the identity check are
  answered without it is unknown.
- One redirect per profile: a failed loopback sign-in does not fall back to the pasted code; run
  `connect` again on the `pasted` instance with `--profile claude.subscription-pasted`.
- Pinned only: no client id ships, so the instance cannot be derived from the declaration.
