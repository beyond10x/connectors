---
format: aep.planning-md/3
id: story:live-reads-jira-confluence-hubspot
kind: story
status: draft
title: Jira, Confluence and HubSpot reads run once against live sandboxes
relations:
- decomposes: epic:tech-debt-review-20260930
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: docs/catalog-confluence.md
- confidence: cited
  path: docs/catalog-hubspot.md
- confidence: cited
  path: docs/catalog-jira.md
revision: 2
---
## Defect

Only GitLab has run against a live provider (`docs/local-catalog-provider.md` Limits). Jira,
Confluence and HubSpot reads are verified against local HTTPS fixtures only, including their identity
reads; the HubSpot guide also leaves open whether HubSpot decodes `%2C` in list values.

## Change

One recorded live run per provider against a sandbox account: connect, each shipped read once, one
two-page walk, with the requests and outcomes kept as evidence under `docs/evidence/` without any
credential or personal data.

## Scope

- `docs/evidence/live-reads-<date>/` (new) — inferred.
- `docs/catalog-jira.md`, `docs/catalog-confluence.md`, `docs/catalog-hubspot.md` Limits — cited.

## Acceptance

- Per provider: `connections connect` succeeds and records the expected identity kind; every shipped
  read returns 200; one list walks two pages to its documented end condition.
- HubSpot: `properties=email%2Cfirstname` returns both properties, or the guide states it does not.
- The evidence holds no token, email address or record content.

## Blocked

`credential-blocker:sandbox-accounts-jira-confluence-hubspot`: no sandbox account or token exists
for this repository.
