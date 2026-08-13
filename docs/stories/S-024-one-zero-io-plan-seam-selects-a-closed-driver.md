---
id: S-024
title: "One zero-IO plan seam selects a closed built-in driver"
pillar: Platform
status: backlog
priority:
design: docs/design/03-beyond-http.md
epic: beyond-http
areas: [domain, service, server, connector-resolve]
note: "ADR 0010 delivery item 1; planning remains data and dispatch has one composition point"
---

# One zero-IO plan seam selects a closed built-in driver

## Goal

Extend request planning beyond HTTP without letting planning perform IO or letting each driver
recompose grants, credentials, egress, redaction, and audit independently.

## Acceptance

- [ ] A typed zero-IO plan names one closed driver and its reviewed operation/channel facts.
- [ ] Grant admission and permission subjects are fixed before credential placement.
- [ ] Exactly one dispatch composition point applies egress, credential, redaction, and audit
      policy before handing a driver its bounded plan.
- [ ] Unknown drivers and unmet capabilities refuse by name; there is no process/plugin fallback.
- [ ] Fence tests fail if a second policy-composition or vendor-dial path appears.

## Progress

- (not started)
