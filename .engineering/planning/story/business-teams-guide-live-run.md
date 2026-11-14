---
format: aep.planning-md/3
id: story:business-teams-guide-live-run
kind: story
status: draft
title: The business-teams setup guide passes one live run
summary: An operator follows the guide with their own Google OAuth client and HubSpot key
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:business-teams-google-and-hubspot
revision: 1
---
# The business-teams setup guide passes one live run

## Need

`epic:business-teams-google-and-hubspot` shipped in 0.72.0 and was tested against fakes. The setup
guide (Google OAuth client, HubSpot private-app key, read-only profiles) has not been followed end to
end against real accounts.

## Acceptance

- An operator follows the guide as written, with their own Google OAuth client and HubSpot
  private-app key, and reads one mailbox, one calendar range, one Drive file and one HubSpot deal.
- Every step that differed from the guide is fixed in the guide.
- No agent creates an OAuth client, key or account; the credentials are the operator's.
