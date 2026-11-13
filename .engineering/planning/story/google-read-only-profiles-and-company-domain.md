---
format: aep.planning-md/3
id: story:google-read-only-profiles-and-company-domain
kind: story
status: implemented
title: Google instances hold read-only scopes by default and admit only the company domain
relations:
- decomposes: epic:business-teams-google-and-hubspot
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-12T16:07:30Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-12T16:07:30Z", actor: "human:operator", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-13T07:29:38Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Need

The guardrail is read-only by default and company account only. Today every Gmail instance requests
and requires `gmail.compose` beside `gmail.readonly`
(`adapters/catalog/providers/google-gmail/connector.json:17-26`); Calendar requests
`calendar.events` and Drive `drive.file`. So a consent grants Google write rights even when no
approval policy allows a write. Nothing checks which Google account connects: the ID token claims
read are `iss`, `aud`, `sub`, `exp`, `iat` (`adapters/catalog/src/local.rs:1100-1111`).

## Acceptance

- Gmail, Calendar and Drive each offer a read-only auth profile requesting only read scopes
  (`gmail.readonly`, `calendar.readonly`, `drive.readonly`), and it is the documented default. An
  instance on it offers no write operation; the existing write-capable profile stays for named use
  cases (Gmail drafts).
- An instance may set the company domain(s) it admits. Connect and reconnect refuse an account whose
  ID token `hd` claim is not one of them (and one with no `hd`), naming the domain, before any
  credential is stored. Tests pin admit, wrong domain and missing `hd`.
- The connectors-definitions side (`connectors/google-*.yaml`, `auth/google.yaml`) is either changed
  to match in the same release or its gap is recorded as a draft there.
