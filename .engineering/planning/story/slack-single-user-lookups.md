---
format: aep.planning-md/3
id: story:slack-single-user-lookups
kind: story
status: draft
title: Slack single-user reads by id and e-mail
relations:
- decomposes: epic:fluxplane-plugin-parity
revision: 1
---
## Outcome

Slack single-user reads: `users.info` (by id) and `users.lookupByEmail` through the catalog provider, so `slack.user.info`, `slack.user.lookup` and `slack.user.show` move from partial (served only by listing) to covered.

## Acceptance

- Spec first: the selections are added to the pinned Slack source selection and the `User` noun in `adapters/slack/spec/ess`, validated with the newest `ess`.
- Each read answers through `connectors operations invoke` on a saved connection against a fixture, with `token` as a credential parameter.
- The parity page rows move to covered.
