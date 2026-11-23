---
format: aep.planning-md/3
id: story:assistant-discovery-on-0-74
kind: story
status: draft
title: 'Assistant keeps the instances connectors 0.74 discovers (consumer: beyond10x/assistant)'
relations:
- decomposes: epic:review-20261123-findings
revision: 1
---
## Finding 4

4. Assistant silently drops discovered instances on 0.74.1. Its discovery code looks for an adapter field on each item. The release returned two configured items, identified by connector and instance. Neither item survived Assistant's filter. Assistant also pins Loom 0.25.0, whose client uses the removed invocation command. Assistant's design names Connectors 0.53.1. This is an upgrade gap for Assistant.
https://github.com/beyond10x/assistant/blob/03548eb1dbaca61ba7b62a76a7fc74e1e7a4c56d/src/connectors.rs#L132
https://github.com/beyond10x/assistant/blob/03548eb1dbaca61ba7b62a76a7fc74e1e7a4c56d/Cargo.toml#L18
https://github.com/beyond10x/assistant/blob/03548eb1dbaca61ba7b62a76a7fc74e1e7a4c56d/docs/vision.md#L32
https://github.com/beyond10x/loom/blob/fa6a8a136692da8693cbac2f78923ab4a0e54f03/crates/loom-connectors/src/cli.rs#L347

## Source

Source: the connectors review of 2026-11-23 (Connectors 0.74.1 with Definitions 0.8.0, checked against main 45276d755c4), handed to this session by Timo the same day. The finding is quoted from it; the links are its citations.

## Acceptance

- In beyond10x/assistant, discovery keeps items identified by connector and instance, against recorded 0.74.1 output.
- Owned by the assistant repository; this story tracks it until a change there lands.
