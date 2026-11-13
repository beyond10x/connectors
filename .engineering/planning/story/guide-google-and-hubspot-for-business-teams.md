---
format: aep.planning-md/3
id: story:guide-google-and-hubspot-for-business-teams
kind: story
status: active
title: One guide takes a non-developer from install to reading mail and HubSpot from Claude
relations:
- decomposes: epic:business-teams-google-and-hubspot
- serves: vision:independent-contract-adapters
- depends_on: story:hubspot-owners-and-line-items
- depends_on: story:credential-from-environment-variable
- depends_on: story:google-read-only-profiles-and-company-domain
- depends_on: story:drive-file-content-read
- depends_on: story:mcp-stdio-on-macos-and-windows
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-12T16:07:34Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-12T16:07:35Z", actor: "human:operator", revision: 3}
---
## Need

The operator promised to share a guide. A first Gmail read today takes 9 to 11 steps across three
pages (`getting-started.md`, `guides/google-oauth-setup.md`, `connectors/gmail.md`), and the first
read shown is `users.getProfile`, which reads no mail.

## Acceptance

- One website guide for a non-developer on macOS: install, the company OAuth client file (from
  1Password, created once by an admin as an Internal app), connect Gmail, Calendar and Drive on the
  read-only profiles with the company domain set, HubSpot with `op run` and `--credential-env`, and
  use from Claude (MCP registration). It shows a real mail search and read.
- The Sales drafts case: the write-capable Gmail profile plus an approval policy that lists only
  `users.drafts.create`; sending stays a person's act in Gmail.
- Every command in the guide is run once and its output checked before release.
