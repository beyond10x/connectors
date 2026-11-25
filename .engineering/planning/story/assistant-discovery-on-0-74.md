---
format: aep.planning-md/3
id: story:assistant-discovery-on-0-74
kind: story
status: draft
title: A private consumer keeps the instances connectors 0.74 discovers
relations:
- decomposes: epic:review-20261123-findings
revision: 3
---
## Finding 4

4. A private consumer silently drops discovered instances on 0.74.1. Its discovery code looks for an adapter field on each item. The release returned two configured items, identified by connector and instance. Neither item survived the consumer's filter. The consumer also pins Loom 0.25.0, whose client uses the removed invocation command. The consumer's design names Connectors 0.53.1. This is an upgrade gap for the consumer.
https://github.com/beyond10x/loom/blob/fa6a8a136692da8693cbac2f78923ab4a0e54f03/crates/loom-connectors/src/cli.rs#L347

## Source

Source: the connectors review of 2026-11-23 (Connectors 0.74.1 with Definitions 0.8.0, checked against main 45276d755c4), handed to this session by Timo the same day. The finding is restated from it: the consumer is a private repository of the organization, so its name is replaced by "a private consumer" and the review's three links into that repository are left out. The link that stays is the review's citation of Loom.

## Acceptance

- In the private consumer, discovery keeps items identified by connector and instance, against recorded 0.74.1 output.
- Owned by the consumer's repository; this story tracks it until a change there lands.
