---
format: aep.planning-md/1
id: story:generated-service-activation
kind: story
status: implemented
title: Activate generated services in the hosted runtime
summary: Compose generated service bundles, durable grants, and exact-input approvals into hosted Connectors.
relations:
- derived_from: story:generated-service-factory
revision: 4
---
## Acceptance

- Hosted composition activates an explicit generated ServiceBundle and publishes its reviewed provider/Connection identity.
- Deployment grant references become durable exact-operation grants without weakening read-path or approval enforcement.
- A human Identity principal can issue one short-lived, exact-input approval through a typed hosted route; it remains one-time at invocation.
- Realm stays an optional verified PrincipalContext claim and never becomes a route, selector, or operation input.
- Focused conformance and the full repository gate pass.
