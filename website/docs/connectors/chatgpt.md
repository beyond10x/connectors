---
title: ChatGPT
sidebar_position: 13.7
description: Sign in with a ChatGPT subscription by browser or device code, keep the access token refreshed, hand it to the person who owns it, and list the models the account may use.
lede: One pinned chatgpt instance per ChatGPT account signs in as the Codex CLI does, refreshes its token with JSON and rotation, and names the connection by the ChatGPT account id; its client id is the instance's configuration, never a shipped default.
source: adapters/catalog/providers/chatgpt/connector.json, adapters/catalog/providers/chatgpt/operations.json, adapters/catalog/src/chatgpt.rs, adapters/catalog/tests/local_runtime/chatgpt.rs, adapters/catalog/tests/local_runtime/chatgpt_journey.rs
connectors: "chatgpt"
---

# ChatGPT

The `chatgpt` connector of the provider OpenAI holds a ChatGPT subscription sign-in. It is served
by the catalog provider, and its profile `chatgpt.subscription` declares the authorization server
the [Codex CLI](https://github.com/openai/codex/tree/970111f6cb62cc02fe36a82ea7d71b37a2889c97/codex-rs/login)
signs in to, value by value, each with the source line it was read from
([declaration](https://github.com/beyond10x/connectors/blob/main/adapters/catalog/providers/chatgpt/connector.json)).
The words connector, instance, connection and operation are those of the
[README vocabulary](https://github.com/beyond10x/connectors#vocabulary).

The profile ships no client id. An instance names the one it signs in as in its own configuration
(`oauth2_client_id`), so the instance cannot be derived from the declaration: it pins its
executable, that executable's SHA-256 and its configuration revision. Commands use `connectors` on
your `PATH` with your own configuration; a command that needs a ChatGPT account is shown without
output.

## What it reads and writes

| | |
|---|---|
| Reads (1) | `models.list`: the models the signed-in account may use, from `GET https://chatgpt.com/backend-api/codex/models` |
| Writes (0) | none |
| Token | `connections token --reveal` prints the connection's current access token for its owner, refreshed first when it expires within 300 seconds |

The connection's identity is the ChatGPT account id, which the sign-in's `id_token` carries in its
`https://api.openai.com/auth` claim (`chatgpt_account_id`), kind `chatgpt.account`.
The Codex CLI sends the token as `Authorization: Bearer <token>` and the account id as
`ChatGPT-Account-ID`.

## Prerequisites at the provider

- A ChatGPT account with a plan. OpenAI's terms decide who may use plan access through a client:
  "ChatGPT plan usage is available to all open-source partners and selected private clients"
  ([Sign in with ChatGPT](https://developers.openai.com/siwc/quickstart)).
- A client id registered with OpenAI for this sign-in, with `http://127.0.0.1:1455/auth/callback`
  as its redirect. Connectors ships none. The Codex CLI's own client id is in
  [its source](https://github.com/openai/codex/blob/970111f6cb62cc02fe36a82ea7d71b37a2889c97/codex-rs/login/src/auth/manager.rs#L1784);
  whether you may use it is OpenAI's terms' question, not this guide's.
- For the device sign-in, device code sign-in switched on in the account's ChatGPT security
  settings.

## Enable the connector and add an instance

Write the instance's native configuration to an owner-only file, `/absolute/path/catalog.json`,
with your client id. Its `auth` is the declared profile as shipped:

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "chatgpt-personal",
  "provider": "chatgpt",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://chatgpt.com/backend-api/codex",
  "oauth2_client_id": "REPLACE_WITH_YOUR_CLIENT_ID",
  "auth": {
    "profile": "chatgpt.subscription",
    "scheme": "oauth2_grant",
    "header": "Authorization",
    "bearer": true,
    "label": "ChatGPT sign-in",
    "identity": {"source": "id_token", "kind": "chatgpt.account", "subject_pointer": "/https:~1~1api.openai.com~1auth/chatgpt_account_id"},
    "oauth2": {
      "authorize_url": "https://auth.openai.com/oauth/authorize",
      "token_url": "https://auth.openai.com/oauth/token",
      "device_authorization_url": null,
      "client_id": null,
      "client_kind": "public",
      "request_encoding": "form",
      "refresh_encoding": "json",
      "redirect_kind": "loopback_fixed_port",
      "redirect_uri": "http://127.0.0.1:1455/auth/callback",
      "authorize_parameters": [
        {"name": "id_token_add_organizations", "value": "true"},
        {"name": "codex_cli_simplified_flow", "value": "true"},
        {"name": "originator", "value": "codex_cli_rs"}
      ],
      "requested_scopes": ["openid", "profile", "email", "offline_access", "api.connectors.read", "api.connectors.invoke"],
      "required_scopes": [],
      "rotation": "rotating",
      "refresh_lead_seconds": 300,
      "grant_types": ["authorization_code"],
      "sources": [
        "https://github.com/openai/codex/blob/970111f6cb62cc02fe36a82ea7d71b37a2889c97/codex-rs/login/src/server.rs#L77 (issuer), #L576-L605 and #L708 (authorize and token endpoints), #L597-L601 (scope), #L78-L80 and #L629-L670 (fixed loopback redirect)",
        "https://github.com/openai/codex/blob/970111f6cb62cc02fe36a82ea7d71b37a2889c97/codex-rs/login/src/oauth/authorization.rs#L20-L36 and codex-rs/login/src/auth/default_client.rs#L42 (authorize extras)",
        "https://github.com/openai/codex/blob/970111f6cb62cc02fe36a82ea7d71b37a2889c97/codex-rs/login/src/oauth/pkce.rs#L15-L29 (S256)",
        "https://github.com/openai/codex/blob/970111f6cb62cc02fe36a82ea7d71b37a2889c97/codex-rs/login/src/oauth/client.rs#L79-L95 (form code exchange), #L51-L63 and #L97-L112 (JSON refresh without scope)",
        "https://github.com/openai/codex/blob/970111f6cb62cc02fe36a82ea7d71b37a2889c97/codex-rs/login/src/auth/manager.rs#L1647-L1690 (rotation), #L214-L215 and #L3300-L3328 (refresh 5 minutes before expiry), #L1784 (the Codex CLI's own client id, not shipped)",
        "https://github.com/openai/codex/blob/970111f6cb62cc02fe36a82ea7d71b37a2889c97/codex-rs/login/src/token_data.rs#L42-L56 and #L92 (the account id claim of the id_token)",
        "https://github.com/openai/codex/blob/970111f6cb62cc02fe36a82ea7d71b37a2889c97/codex-rs/login/src/device_code_auth.rs#L66-L218 (the device sign-in, not RFC 8628; adapters/catalog/src/chatgpt.rs)"
      ],
      "state_in_exchange": false
    }
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/chatgpt/operations.json"
}
```

Print its bootstrap and hash the provider:

```sh
connectors-catalog-provider --local-config /absolute/path/catalog.json --print-local-bootstrap
sha256sum /absolute/path/connectors-catalog-provider
```

Then pin the instance with the bootstrap's `configuration_revision` and that hash:

```toml
[connectors.chatgpt]
enabled = true

[connectors.chatgpt.instances.personal]
alias = "personal"
instance_id = "chatgpt-personal"
adapter_id = "catalog"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
startup = "on-demand"
restart = "never"

[connectors.chatgpt.instances.personal.permissions]
profiles = ["chatgpt.subscription"]
operations = ["models.list"]

[connectors.chatgpt.instances.personal.executable]
path = "/absolute/path/connectors-catalog-provider"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/catalog.json"]
```

An instance derived with `--connector chatgpt` alone is refused as `InvalidConfiguration`: it has
no client id. A second ChatGPT account is a second instance.

## Connect

Sign in with the browser. The CLI prints the authorization address, listens on
`127.0.0.1:1455` for the redirect, and exchanges the code it receives with PKCE (S256):

```sh
connectors connections connect --connector chatgpt --instance personal \
  --profile chatgpt.subscription
```

Port 1455 must be free; a running Codex CLI sign-in holds it.

Without a browser on this machine, sign in with a device code instead:

```sh
connectors connections connect --connector chatgpt --instance personal \
  --profile chatgpt.subscription --device
```

The CLI shows a user code and `https://auth.openai.com/codex/device`; enter the code there from
any browser within 15 minutes. Without a terminal it writes the single line
`connectors: device-code <code> https://auth.openai.com/codex/device` on standard error. This is
the ChatGPT device sign-in, not RFC 8628: the code and verifier it ends in are exchanged at the
token endpoint as the browser's code is.

`connections describe` shows the connection `ready`, its `external_identity` the ChatGPT account
id, the granted scopes and the access token's expiry, never a token.

## A first read

Take `--schema` and `--revision` from `operations describe --connector chatgpt --instance personal
--operation models.list`, then:

```sh
connectors operations invoke --connector chatgpt --instance personal --connection "$connection" \
  --operation models.list --schema "$schema" --revision "$revision" \
  --input-json '{"client_version":"0.0.0"}'
```

To use the subscription from your own tool, print the access token to its standard input:

```sh
connectors connections token --connector chatgpt --instance personal \
  --connection "$connection" --reveal
```

Standard output is the token and one newline. Without `--reveal` the command prints only the
expiry, scopes and the recorded issue. A token that expires within 300 seconds is refreshed first:
the CLI sends `{"grant_type":"refresh_token","client_id":…,"refresh_token":…}` as JSON and keeps
the refresh token the answer rotates in.

## A first guarded write

The connector serves no write. Every request you send with the revealed token is yours, outside
Connectors' approvals and ledger.

## Reconnect when the connection is Stale

A refresh the server refuses (a revoked sign-in, a spent refresh token) makes the connection
`stale`; `connections token` and an invoke answer `next_action: repair_connection`. Sign in again
on the same connection, by browser or with `--device`:

```sh
connectors connections list --connector chatgpt --instance personal
connectors connections reconnect --connector chatgpt --instance personal \
  --expected-revision "<revision from the list>"
```

A sign-in to another ChatGPT account is refused as an identity mismatch.

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/local_runtime/chatgpt.rs`
  and `chatgpt_journey.rs`), which answers the shapes of the Codex CLI's source at the pinned
  commit; no sign-in has run against OpenAI.
- The core requires the token answer's `token_type` and a positive `expires_in`; the Codex CLI
  reads the expiry from the token itself, so whether OpenAI's answer carries both is unverified.
- The redirect port is 1455 only; the Codex CLI's fallback to 1457 is not taken.
- `models.list` sends no `ChatGPT-Account-ID` header, which the Codex CLI sends; whether the
  backend needs it there is unverified.
- The identity is read from the sign-in's `id_token`; a refresh keeps it and does not read it
  again.
- `connections revoke` ends local use and does not sign out at OpenAI.
