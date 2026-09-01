---
format: aep.planning-md/1
id: story:runtime-cutover
kind: story
status: implemented
title: Cut the runtime over to Secrets
summary: Compose the remote backend and bounded transition mode.
relations:
- derived_from: epic:remote-secrets
revision: 4
---
## Acceptance

Runtime configuration selects Secrets, its exact projected-token audience and endpoint; Vault remains available only for an explicit migration window.
