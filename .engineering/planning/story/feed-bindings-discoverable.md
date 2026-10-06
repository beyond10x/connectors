---
format: aep.planning-md/3
id: story:feed-bindings-discoverable
kind: story
status: active
title: A consumer finds a connection's feed operations at run time
relations:
- decomposes: epic:generic-datasource-feeds
- serves: vision:independent-contract-adapters
- depends_on: story:feed-contract
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T17:00:04Z", actor: "agent:claude", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-06T17:00:04Z", actor: "agent:claude", revision: 3}
---
## Outcome

A consumer asks Connectors, for one saved connection, which operations bind `datasource.feed/v1alpha1` and with which profile, and gets the answer at run time, so a new or upgraded adapter is usable by a consumer that was built before it.

## Acceptance

- `connectors operations list --connection <id> --family datasource.feed/v1alpha1` (or the existing describe surface, whichever the CLI contract settles) answers the containers and items operations of that connection's adapter with the profile id and contract version.
- A fixture adapter added after the consumer binary was built is found and read through the family in a test, with no change to the consumer.

## Depends on

`story:feed-contract`.
