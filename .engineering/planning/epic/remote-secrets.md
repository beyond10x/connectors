---
format: aep.planning-md/1
id: epic:remote-secrets
kind: epic
status: implemented
title: Use shared encrypted secret custody
summary: Replace runtime Vault custody with the shared Secrets API.
revision: 4
---
## Outcome

Connectors stores credential bytes in the released Secrets service while retaining provider exchange, refresh and revocation ownership.

## Acceptance

The runtime uses Secrets for normal and prepared operations, preserves owner identity, and migrates Vault values without writing plaintext to disk.
