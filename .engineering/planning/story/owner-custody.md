---
format: aep.planning-md/1
id: story:owner-custody
kind: story
status: implemented
title: Preserve credential ownership at custody
summary: Carry owner subject through the additive storage port.
relations:
- derived_from: epic:remote-secrets
revision: 4
---
## Acceptance

Credential creation and refresh send the authenticated owner subject to stores that support ownership without breaking existing store implementations.
