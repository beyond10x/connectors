# Connect Slack

> Personal-local alpha. The final product presents this flow in **Settings → Connections**; the CLI
> is the current first-party surface for the same operation.

Ask whoever manages your Slack app to enable Socket Mode and subscribe it to the message events you
want. The person authorized to handle its app-level token then completes **Add Slack** directly;
the token should not be sent to another person, agent, or harness. This is one-time app setup. See
Slack's [Socket Mode setup guide](https://docs.slack.dev/apis/events-api/using-socket-mode/) for the
app-side steps.

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
Events: app_mention, message
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

For this personal-local alpha, the person configuring the deployment also owns the Slack app. A
hosted multi-workspace product must keep the app-level Socket Mode token on the Integration and use
Slack OAuth to create workspace Connections; that production flow is not claimed here.
