---
format: aep.planning-md/1
id: story:secrets-adapter
kind: story
status: implemented
title: Implement the Secrets remote-store adapter
summary: Map connector storage ports onto the released Secrets client.
relations:
- derived_from: epic:remote-secrets
revision: 4
---
## Acceptance

An official remote adapter maps every SecretStore and PreparedSecretStore operation, preserves tenant and reference identity, and refuses secret bytes in errors or URLs.
