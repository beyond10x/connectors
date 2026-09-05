---
title: Events and durable state
description: How provider events are attributed, admitted, persisted, and consumed, and where delivery guarantees stop.
sidebar_label: Events and durable state
sidebar_position: 4
b10x:
  documentType: architecture
  audiences: [developer, operator]
---

# Events and durable state

An event is a normalized fact from an external provider, attributed to a Channel, Connection, and
Integration. Event access is governed: a provider event is not visible to every caller merely
because Connectors has received it.

## Follow an incoming event

The Slack Socket Mode path illustrates the implemented intake boundary:

```mermaid
sequenceDiagram
    participant Slack as Slack
    participant Channel as Connector channel
    participant Store as Event store
    participant Client as Authorized consumer
    Slack->>Channel: Provider envelope
    Channel->>Channel: Attribute, normalize, and admit
    Channel->>Store: Persist admitted event
    Store-->>Channel: Stored
    Channel-->>Slack: Acknowledge envelope
    Client->>Channel: Receive with current authority
    Channel->>Store: Read admitted events
    Store-->>Channel: Normalized facts
    Channel-->>Client: Events and cursor
```

Persisting before acknowledgement lets Slack redeliver when the event cannot be stored. Provider
envelopes, Socket Mode tickets, and app tokens do not become consumer payloads. Loop guards discard
message classes excluded by the channel policy. The [Slack guide](../guides/connect-slack.md)
explains the supported local and hosted profiles.

This sequence describes Socket Mode. Webhook verification, attribution, and polling depend on
their declared binding and implementation. Provider-originated events carry `native` provenance;
synthesized polling events carry `polled` provenance.

## What a consumer receives

The [DataEvent contract](../../crates/protocol/src/event.rs) carries:

| Field | Purpose |
|---|---|
| `event_ref` | Stable reference for the admitted event |
| `channel_ref`, `connection_ref`, `integration_ref` | Attribution to intake path and authorized account |
| `event_type` | Provider event name |
| `provenance` | Native delivery or polling |
| `received_at_unix_ms` | Connector receipt time |
| `payload` | Normalized provider data |

Search discovers event descriptions. Receive reads events using the protocol's cursor; replay
addresses retained events by reference. Admission still applies when reading or replaying a fact.
Method inputs and results appear in the deployed [API reference](interfaces.md#the-deployed-http-reference).

## Durability has a boundary

The personal-local event store is bounded at 10,000 events and 64 MiB. When it cannot admit another
event, Socket Mode intake stops acknowledgement rather than claiming delivery. Retention and
compaction are not a completed general capability across all deployments.

Delivery is not a universal exactly-once claim. Provider redelivery, consumer retries, and replay
are separate from operation idempotency. The hosted Slack companion reply path has a specific
one-time, expiring reply claim; it does not guarantee exactly-once arbitrary external effects.

## State and audit are separate concerns

| Concern | Current mechanism |
|---|---|
| Connector-owned state | Bounded keyed cells through [StateStore](../../crates/connector-state/src/lib.rs), bound to concrete storage by the runtime |
| Provider event history | Normalized events and cursors, with provider/deployment-specific intake and retention limits |
| Approval redemption | One-time claim plus audit/recovery semantics in the [approval gate](../../crates/domain/src/approval.rs) |
| Execution audit | Attempt and outcome records owned by the admitted execution path |

The state port supplies bounded atomic operations; it does not promise arbitrary transactions
across keys. Using it does not mean entities execute through Entity Runtime or that every store
shares one event-sourcing model.

## Operational events are not a shipped general stream

ESS declares lifecycle facts such as Connection authorization and session termination. The generic
event protocol explicitly carries provider data events and excludes that operational family.
Those declarations do not establish that subscribers can receive a complete operational stream.

Delivery and Subscription also have partial ESS definitions. Retry policy, subscription lifecycle,
relationships, and read consistency are not comprehensively modeled there. See
[Specification status](specification.md) for the distinction between a declaration and an
implemented transport or decision rule.

[Previous: Commands and interfaces](interfaces.md) · **Next:** [Deployment and runtime](deployment.md)
