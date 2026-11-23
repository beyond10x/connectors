---
format: aep.planning-md/3
id: story:definition-lowerer-keeps-walk
kind: story
status: draft
title: An operation loaded from a definition file keeps its pagination walk
relations:
- decomposes: epic:review-20261123-findings
revision: 1
---
## Finding 2

2. Switching to a definition file can remove automatic pagination. The released Slack provider included x-connectors-walk in the description of conversations.list. Describing the same read through Definitions 0.8.0 succeeded and omitted that field. Jira's definition explicitly declares a next-page token. For ordinary operations the lowerer takes the page-size bound and omits the continuation rule. The catalog parity test removes walk before comparing operations. Preserve continuation and completion rules across both paths.
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/adapters/catalog/providers/slack/operations.json#L9
https://github.com/beyond10x/connectors-definitions/blob/3c4d34154abc6fe61705aa9e2e15115238303d2d/connectors/slack.yaml#L32
https://github.com/beyond10x/connectors-definitions/blob/3c4d34154abc6fe61705aa9e2e15115238303d2d/connectors/jira.yaml#L9
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/crates/connectors-catalog/src/definition.rs#L3744
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/crates/connectors-catalog/tests/definition_import.rs#L193

## Source

Source: the connectors review of 2026-11-23 (Connectors 0.74.1 with Definitions 0.8.0, checked against main 45276d755c4), handed to this session by Timo the same day. The finding is quoted from it; the links are its citations.

## Acceptance

- Slack `conversations.list` described through a definition file carries the same `x-connectors-walk` as the built-in selection.
- The catalog parity test compares walk instead of removing it before the comparison.
