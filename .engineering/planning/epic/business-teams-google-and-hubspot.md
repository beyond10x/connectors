---
format: aep.planning-md/3
id: epic:business-teams-google-and-hubspot
kind: epic
status: active
title: Business teams read Gmail, Calendar, Drive and HubSpot from Claude through Connectors, read-only by default
relations:
- serves: vision:independent-contract-adapters
- informed_by: epic:google-workspace-reads
- informed_by: epic:hubspot-crm-reads
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-12T16:07:25Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-12T16:07:26Z", actor: "human:operator", revision: 3}
---
## Outcome

Sales, CS, marketing and finance of an adopting company read Gmail, Google Calendar, Google Drive
and HubSpot from Claude through Connectors instead of Claude's prebuilt Google connectors, under the
guardrails that company proposed on 2026-11-12: read-only by default, writes only for a named use
case (Gmail drafts for Sales), company account only, HubSpot through a read-only service key kept in
a password manager and passed with `op run`, and a short how-to.

The operator answered that `beyond10x/connectors` supports this. An audit on 2026-11-12
(origin/main 66c17e6, v0.71.0 the same) found it partly true:

- Met: Gmail search, read, attachments; Calendar reads; Drive search, metadata, Docs/Sheets/Slides
  export; HubSpot has no write; Google writes need an approval policy.
- Gaps, one story each: HubSpot owners and line items; a credential from an environment variable
  (`op run`); Google instances that hold read-only scopes and only a company account; Drive file
  content (PDFs); the MCP server on macOS and Windows; one how-to for non-developers.
- Outside code: a company Google OAuth client (credential-blocker:google-oauth-client) and a
  HubSpot token for a live read (credential-blocker:sandbox-accounts-jira-confluence-hubspot).

The operator made this the first priority on 2026-11-12.

## Deadline: done by 2026-11-13

- [ ] The six stories of this epic implemented, merged, and in a released Connectors version.
- [ ] The company Google OAuth client exists (Internal, Desktop app, read scopes plus
      `gmail.compose`), its file in the password manager; credential-blocker:google-oauth-client
      cleared. The operator approved it on 2026-11-12; a Google Workspace admin creates it.
- [ ] A read-only HubSpot key (deals, companies, line items, owners) for one live read; approved
      by the operator on 2026-11-12.
- [ ] The guide's commands run once against the company client and a real mailbox.
- [ ] The guide shared with the teams who asked.
- [ ] The operator's own Google Calendar connected on the workstation on the read-only Calendar
      profile, and the exact command that lists its events for a date range handed to the
      operator's planning session. Needs the read-only profile, the company OAuth client, and the
      operator's consent click.
