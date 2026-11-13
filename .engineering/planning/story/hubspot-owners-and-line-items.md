---
format: aep.planning-md/3
id: story:hubspot-owners-and-line-items
kind: story
status: active
title: HubSpot owners and line items are read for finance
relations:
- decomposes: epic:business-teams-google-and-hubspot
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-12T16:07:27Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-12T16:07:27Z", actor: "human:operator", revision: 3}
---
## Need

Finance reads deal amounts and terms against billing, the company owner or CSM, and line items. The
HubSpot connector offers `objects.list` and `object.get` on CRM Objects `2026-09`
(`adapters/catalog/providers/hubspot/operations.json:5-11`). Owners are a separate HubSpot API the
connector does not include, so an owner id (`hubspot_owner_id`) cannot be turned into a name or
email. Line items are reachable only as an untested free `objectType`. The docs name
`crm.objects.contacts.read` as the example scope (`website/docs/connectors/hubspot.md:29-31`).

## Acceptance

- Reads of HubSpot owners (list, and one owner by id) are offered, read-only, from HubSpot's
  published owners API document, recorded under `adapters/hubspot/upstream/` with its source hash.
- A protocol test reads deals, companies and line items through `objects.list`/`object.get`, and an
  owner by the id a deal or company carries, against a fixture.
- The HubSpot docs name the scopes for this set: deals, companies, line items and owners read; no
  contacts scope needed.
- The exact operation list stays reads only; the test that pins it is updated.
