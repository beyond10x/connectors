# Connect Slack

> Personal-local alpha. The final product presents this flow in **Settings → Connections**; the CLI
> is the current first-party surface for the same operation.

Ask whoever manages your Slack app to enable Socket Mode and subscribe it to the message events you
want. Socket Mode needs an app-level `xapp` token with `connections:write`; receiving the selected
events and replying needs the app installation's bot token with the scopes shown by the setup flow.
Those are two different credentials. The person authorized to handle them completes **Add Slack**
directly; credentials should not be sent to another person, agent, or harness. See Slack's
[Socket Mode setup guide](https://docs.slack.dev/apis/events-api/using-socket-mode/) for the app-side
steps.

Then add Slack:

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

For this personal-local alpha, the person configuring the deployment also owns the Slack app and
the command currently collects only the app-level Socket Mode token. The ordinary bot, delegated
user, and Enterprise Admin Web API surfaces are catalogued but their general OAuth/scope-aware
Connection runtime is not claimed yet.

The finished product asks only for credentials required by the chosen features:

- **Receive over Socket Mode:** app-level token (`xapp`) for the socket plus bot authorization for
  the selected events and replies.
- **Act as a person:** a separate delegated user OAuth connection; never silently substituted for
  the bot.
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
