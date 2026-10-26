---
format: aep.planning-md/3
id: release-plan:connectors-v0580-oauth2-subscriptions
kind: release-plan
status: implemented
title: 'Release 0.58.0: OAuth2 grants, connections token, ChatGPT and Claude subscriptions, ESS 0.69.2'
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "active", at: "2026-10-26T11:44:29Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"test_result":1}}}
- {from: "active", to: "implemented", at: "2026-10-26T12:53:49Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":2}}}
---
## Outcome

Minor release 0.58.0 of wave 20261025-oauth2 (`approval-record:wave-20261025-oauth2`, main `49a201250`, PR #209) and the ESS pin move to 0.69.2:
- `oauth2_grant` profiles: `connections connect` runs the authorization code grant with PKCE or the device grant and stores the token response in custody.
- The owner refreshes through a durable refresh ledger; `connections token` and `connectors_host::local::access_token` hand out a current access token, printed only with `--reveal`.
- `connections launch` puts only the current access token on descriptor 3 for an OAuth2 grant.
- The `chatgpt` and `claude-subscription` catalog connectors, with no client id shipped.
- ESS 0.69.2 (from 0.67.0 at 0.57.0).

Minor rather than patch, because there is a new command (`connections token`), a new auth profile kind and two new connectors. No consumer-facing contract breaks: the CLI semantics page gains one documented source of `unsupported` after dispatch (a device grant whose poll interval does not fit the sign-in window).

Approved by the directing session `b10x-chef-orchestrator` under its grant (dispatch-queue item 1, 2026-10-26): move the ESS pin to 0.69.2 and release per AGENTS.md.

## Scope

| story | status |
|---|---|
| story:oauth2-grant-core | implemented |
| story:oauth2-refresh-metadata-authority | implemented |
| story:connect-runs-an-oauth2-grant | implemented |
| story:owner-rotating-refresh | implemented |
| story:connections-token-issues-an-access-token | implemented |
| story:launch-hands-an-oauth2-access-token | implemented |
| story:chatgpt-subscription-connector | implemented |
| story:claude-subscription-connector | implemented |

`story:google-consent-on-the-oauth2-core` stays draft and is not in this release.

## Limitations

These are the limitations the CHANGELOG 0.58.0 section states:
- The subscription sign-ins and refreshes are verified against a local fake authorization server only, never live.
- A device sign-in waits about 4 minutes, inside the 300 s acquisition entry window.
- The Google consent flow does not yet use the OAuth2 core.
