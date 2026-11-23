---
format: aep.planning-md/3
id: story:inventory-lists-instances-awaiting-a-profile
kind: story
status: draft
title: The instance inventory lists an instance still awaiting an auth-profile choice
relations:
- decomposes: epic:review-20261123-findings
revision: 1
---
## Finding 8

8. The inventory omits an instance that still affects target selection. A Zendesk configuration had two instances, one awaiting an auth-profile choice. The catalog listed both. The instance inventory showed only the one with a selected profile. Describing the connector without an instance then refused as ambiguous and named both. Surface the pending instance and the missing choice in the inventory.
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/crates/connectors-host/src/local/config.rs#L603
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/apps/connectors/src/local/invoke.rs#L174
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/apps/connectors/src/local/invoke.rs#L201

## Source

Source: the connectors review of 2026-11-23 (Connectors 0.74.1 with Definitions 0.8.0, checked against main 45276d755c4), handed to this session by Timo the same day. The finding is quoted from it; the links are its citations.

## Acceptance

- A configuration with two Zendesk instances, one without a selected profile, lists both in the inventory, the second with the missing choice named.
