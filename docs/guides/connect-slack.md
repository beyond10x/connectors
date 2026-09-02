# Connect Slack

> Personal-local CLI and hosted Zwirn setup are available. Both are Connector-owned Connect
> Sessions; neither sends provider credentials through Agent or a model.

Ask whoever manages your Slack app to enable Socket Mode and subscribe it to the message events you
want. Socket Mode needs an app-level `xapp` token with `connections:write`; receiving the selected
events and replying needs the app installation's bot token with the scopes shown by the setup flow.
Those are two different credentials. The person authorized to handle them completes **Add Slack**
directly; credentials should not be sent to another person, agent, or harness. See Slack's
[Socket Mode setup guide](https://docs.slack.dev/apis/events-api/using-socket-mode/) for the app-side
steps.

For personal-local, add Slack with:

```sh
connectors connect slack
```

The command asks for the app token with terminal echo disabled and waits for Slack to become
reachable. A successful connection looks like this:

```text
Connect Slack
Input is hidden and sent only to the local Connector.
Slack app token: [hidden]

Slack is connected and ready to receive messages.
Connection: Slack
Events: app_mention, message.channels
```

That is the whole connection flow. You do not copy session ids, handle socket paths, export token
variables, edit secret files, or give a credential to a model. Once the read-only harness adapter
is enabled, admitted Slack messages appear as normal harness input from the Slack Connection.

## What happens behind the button

The product creates a short-lived Connect Session, renders its protected credential input directly
to the human, submits it once to the Connector, and follows the result until the durable Connection
is callable. The product discards the short-lived acquisition details and shows only the resulting
Connection. Neither an agent nor its harness receives the credential or the one-use completion
endpoint.

For personal-local, the command currently collects only the app-level Socket Mode token. Hosted
Zwirn presents three deliberately separate roles:

- **Organization Slack** is deployment-owned and enabled from value-free policy. Its `xapp`,
  `xoxb`, and OAuth client secret remain in Connector custody. The hosted OAuth client secret is
  supplied through `connectors admin credentials set slack oauth_client_secret`; runtime-issued
  bot and app credentials use their dedicated Connect Sessions. Its bot token
  exposes normalized public-channel and user-directory reads to organization members, but the
  organization bot Connection admits no writes and no event channel.
- **Connect my Slack account** starts Slack's user-centric OAuth flow. Connectors binds the one-use
  state to the initiating principal, requires the configured workspace, compares Slack's verified
  email with the normalized Identity email, and writes the resulting `xoxp` and optional refresh
  token only to that Connection's Vault instance. This Connection can read the consenting user's
  private channels and DMs; user-attributed mutations still require normal correlated approval.
- **Add a companion bot** opens a Connector-owned form for that person's `xapp` and `xoxb` only.
  The bot must belong to the configured organization workspace. Each successful submission creates
  another principal-owned Connection, so one person can connect multiple companion apps without
  sharing a token or Socket supervisor.

Each companion Connection owns its own Socket Mode WebSocket and remains connected while Zwirn is
offline. Zwirn pulls only that principal's normalized Slack events with a short-lived
`connectors.events.self` token; tenant-wide module events still require `connectors.events.read`
and operator admission. A fresh `app_mention` carries one Connector-enforced reply grant pinned to
the same Connection, channel, and thread. Its event reference is durably claimable once and expires
after ten minutes. Every other mutation, including a second reply, follows the normal person
approval path. Neither Zwirn nor the model receives a Socket Mode ticket or Slack token.

The finished product asks only for credentials required by the chosen features:

- **Receive over Socket Mode:** one principal-owned companion app-level token (`xapp`) for the
  socket plus that same companion's bot authorization for selected events and replies.
- **Act as a person:** a separate principal-owned delegated user OAuth Connection, using Slack's
  user-centric consent flow. Zwirn may use it only through an explicit Grant; Slack sees and audits
  the consenting user. The shared bot token is never silently substituted.
- **Read Enterprise administration:** a separate organization-wide OAuth installation completed by
  an Enterprise Org Admin or Owner. Slack grants the selected `admin.*` scopes on a user token;
  there is no magic admin token type inferred from its prefix.
- **Receive by public callback:** the signing secret verifies webhook bytes; no app-level token is
  needed for that transport.

A hosted multi-workspace product keeps app-level Socket Mode authority on the Integration and uses
Slack OAuth to create workspace Connections. Discovery shows an operation or event only when the
selected Connection has current proof of its required credential and scopes. Slack App Manifest
configuration tokens are a separate setup-only type and are not requested because this connector
does not expose app-configuration operations.
