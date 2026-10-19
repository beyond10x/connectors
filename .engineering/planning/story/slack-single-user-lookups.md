---
format: aep.planning-md/3
id: story:slack-single-user-lookups
kind: story
status: implemented
title: Slack single-user reads by id and e-mail
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/catalog/providers/slack/operations.json
- confidence: cited
  path: adapters/catalog/providers/slack/user-operations.json
- confidence: cited
  path: adapters/catalog/tests/slack.rs
- confidence: cited
  path: crates/connectors-catalog/tests/fixtures/definitions/connectors/slack-user.yaml
- confidence: cited
  path: crates/connectors-catalog/tests/fixtures/definitions/connectors/slack.yaml
- confidence: cited
  path: crates/connectors-docs/src/status.rs
- confidence: cited
  path: docs/catalog-slack.md
- confidence: cited
  path: website/docs/connectors/slack.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-18T22:42:36Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-18T22:42:36Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-19T05:54:50Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":2,"review_outcome":1}}}
---
## Outcome

Slack single-user reads: `users.info` (by id) and `users.lookupByEmail` through the catalog provider, so `slack.user.info`, `slack.user.lookup` and `slack.user.show` move from partial (served only by listing) to covered.

## Acceptance

Amended 2026-10-19: the `User` noun already exists in `adapters/slack/spec/ess/domains/records.yaml`
(`connectors_slack.records.User`), and by ADR D11 the selection is authored in the
`connectors-definitions` repository (`connectors/slack.yaml`) and lowered into
`adapters/catalog/providers/slack/operations.json`, so no spec change came first.

- `users.info` and `users.lookupByEmail` answer through the catalog provider against a fixture in
  the pinned document's shapes, with `token` as a credential parameter
  (`adapters/catalog/tests/slack.rs`).
- Slack's `ok:false` answers `user_not_visible` and `enterprise_is_restricted` are `forbidden` and
  `users_not_found` is `not_found` on every Slack read (the shared envelope).
- The parity page rows move to covered.
