---
format: aep.planning-md/1
id: story:generated-service-factory
kind: story
status: implemented
title: Compose generated services through a fail-closed Connector factory
summary: Bind generated service catalogs and dispatch behind explicit deployment policy without granting authority at registration.
revision: 4
---
## Acceptance

- A generator-targetable service factory contributes stable service metadata, a complete operation catalog, and the backend dispatch operations it owns through the existing `ConnectorBackend` seam.
- Registering a factory is inert: it neither exposes an operation nor supplies endpoints, credentials, grants, or provider identity.
- An explicit deployment overlay assigns the permanent provider reference and reverse-DNS authority and states each operation's exposure, risk, approval posture, endpoint bindings, credential bindings, and grant references.
- Bundle construction is deterministic and refuses invalid manifests or deployments, duplicate service/provider/authority/operation identities, unknown or repeated deployments, incomplete operation overlays, factory bind failures, and catalog/dispatch/ownership drift.
- Focused conformance tests cover inert registration, deterministic composition and policy projection, and the closed refusal vocabulary.
