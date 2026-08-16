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
Zwirn instead starts an HTTPS Connect Session and opens the exact same-origin setup page. That page
accepts the Integration's app-level Socket Mode token plus one workspace's bot and delegated-user
tokens and submits all three directly to hosted Connectors. The Connector validates that the bot
and user belong to the same Slack workspace, prepares all credential changes in Vault, commits the
transaction, and only then publishes a value-free workspace Connection. Zwirn learns only the
terminal Connection reference.

In the hosted companion flow, one Integration supervisor owns the Socket Mode WebSocket and routes
events by Slack workspace to the correct Connection. It stays connected while Zwirn is offline.
The bot credential receives mentions and performs `chat.postMessage`/reaction writes; the delegated
user credential performs conversations-history and user-info reads. Mutating operations require
normal correlated human approval. Zwirn pulls only normalized admitted events with a short-lived
`connectors.events.read` token; it never receives the Socket Mode ticket or any Slack token.

The finished product asks only for credentials required by the chosen features:

- **Receive over Socket Mode:** app-level token (`xapp`) for the socket plus bot authorization for
  the selected events and replies.
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
