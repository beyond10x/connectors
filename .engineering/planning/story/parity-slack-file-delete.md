---
format: aep.planning-md/3
id: story:parity-slack-file-delete
kind: story
status: implemented
title: Slack file delete
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/catalog/generated/bundles/slack.bundle.json
- confidence: cited
  path: adapters/catalog/providers/slack/operations.json
- confidence: cited
  path: adapters/catalog/providers/slack/user-operations.json
- confidence: cited
  path: adapters/catalog/tests/slack.rs
- confidence: cited
  path: adapters/slack/generated/slack-web.openapi.json
- confidence: cited
  path: adapters/slack/upstream/slack-web.amendments.json
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
- {from: "draft", to: "proposed", at: "2026-10-18T22:42:37Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-18T22:42:37Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-19T05:54:49Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":2,"review_outcome":1}}}
---
## Outcome

`files.delete` through the catalog provider as `file.delete`, `effect: write` under an issued
approval, in the shape of `chat.delete`: a preflight `auth.test`, a further preflight `files.info`
on the file id, a postflight `/ok` true with Slack's `file_not_found` and `file_deleted` refused as
`not_found` and `cant_delete_file` as `forbidden`. Parity unit U13's delete half:
`slack.file.delete`, 7 calls. The upload half stays in story:parity-slack-files behind
story:catalog-composed-write.

## Surface

One write selection in `adapters/catalog/providers/slack/operations.json` (bot set), admitted in
the Slack declaration; a `correct_media_type` amendment for `files_delete` in
`adapters/slack/upstream/slack-web.amendments.json` (the pinned document lists form data and JSON;
JSON is sent, as for `chat_delete`), so the Slack projection and bundle are rebuilt. The read
envelope of every Slack read gains `user_not_visible`, `enterprise_is_restricted` (forbidden) and
`users_not_found` (not_found), shared with story:slack-single-user-lookups.

## Acceptance

- `connectors operations invoke --connector slack --instance <i> --operation file.delete` with a
  file id, under an approval file, sends one JSON body to the fixture after the two preflights and
  answers `ok`; a fixture answering `ok:false, error: file_deleted` is refused as `not_found`
  naming Slack's error, sent once.
- `adapters/catalog/tests/bundle_drift.rs` passes with the rebuilt projection and bundle.
- The parity page row `slack.file.delete` moves to covered; the Slack guide's "Files" section names
  the write with the command run in the tree.

## Scope

Primary: `adapters/catalog/providers/slack/operations.json`,
`adapters/slack/upstream/slack-web.amendments.json`, `adapters/slack/generated/slack-web.openapi.json`,
`adapters/catalog/generated/bundles/slack.bundle.json`, `adapters/catalog/tests/slack.rs`,
`docs/catalog-slack.md`, `website/docs/connectors/slack.md`,
`crates/connectors-catalog/tests/fixtures/definitions/connectors/slack.yaml`. Collides with
story:slack-single-user-lookups on the same files: one implementor does both.
