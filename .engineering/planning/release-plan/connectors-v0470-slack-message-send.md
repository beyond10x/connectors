---
format: aep.planning-md/3
id: release-plan:connectors-v0470-slack-message-send
kind: release-plan
status: implemented
title: 'Release 0.47.0: Slack message send, boolean guard checks, non-empty body keys'
relations:
- serves: vision:independent-contract-adapters
- informed_by: decision-blocker:wave-20261013a-scope
revision: 3
transitions:
- {from: "draft", to: "active", at: "2026-10-13T05:01:17Z", actor: "human:timo", revision: 2}
- {from: "active", to: "implemented", at: "2026-10-13T11:49:05Z", actor: "human:timo", revision: 3}
---
## Outcome

Minor release 0.47.0: the Slack catalog provider posts messages, `chat.postMessage` and `conversations.open` as writes under approval, and the catalog engine gains a JSON boolean guard expectation and non-empty body keys.

Minor rather than patch: two new write selections, two new selection-format members, and a load-time refusal of guard literals `"true"` and `"false"` that a configuration carrying its own selection file must migrate.

## Scope

| surface | change |
|---|---|
| `adapters/catalog/providers/slack/operations.json`, `adapters/slack/upstream/slack-web.amendments.json`, Slack bundle | `chat.postMessage` and `conversations.open` writes, each read-guarded by `auth.test`, JSON request bodies |
| `adapters/slack/spec/ess/domains/records.yaml` | send and open inputs and answers, `WriteOutcome` |
| `adapters/catalog/spec/ess/domains/{guard,selection}.yaml`, `adapters/catalog/src/lib.rs` | `Expectation.boolean`, `ClosedBody.body_nonempty`, refusal of guard literals `"true"`/`"false"` |
| `adapters/catalog/providers/{gitlab,jira}/operations.json` | three guards compare JSON booleans |
| docs, website, status page, CHANGELOG, README | documentation and version 0.47.0 |

Story: `story:slack-message-send` (https://github.com/beyond10x/connectors/issues/158); decided in `decision-blocker:slack-message-writes-scope` and `decision-blocker:wave-20261013a-scope`.

## Evidence

- Wave PR https://github.com/beyond10x/connectors/pull/161, merged as `8e3faa38e`: Rust gate, Custody platforms, Documentation validation, Planning store and Shared source gates passed on `e55ab2f67`.
- Package gates on `79b5a772d`: clippy and test of connectors-catalog-provider, connectors-build (`catalog_guard_model`) and connectors-docs, 641 passed, 0 failed; `connectors-docs generate --check` current.
- Adversary review: `review-result:slack-message-send-adversary-20261013`, 4 findings, all fixed.

## Completion boundary

Verified against local fixtures only; no live Slack call. `chat.update` and `chat.delete` stay in `story:parity-slack-message-writes`. GitLab `merge_request.discussion.resolve` still compares `/resolved` against an input reference as text. `story:host-runs-on-macos` follows this release.
