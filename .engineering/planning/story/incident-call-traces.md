---
format: aep.planning-md/3
id: story:incident-call-traces
kind: story
status: draft
title: Follow a call's traces by its identifier across sources
relations:
- decomposes: epic:incident-collection-workflow
- depends_on: story:incident-call-lookup
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

A read operation follows a call's traces by its call identifier: the log lines every source that
handled the call wrote, in time order, each naming its source, within a bounded window. It is
callable through `connectors operations describe` and `invoke` with no mutation profile.

## Source

LogQL queries over the configured Loki connections (`story:parity-loki-query`), selected by the
call identifier and the window the call lookup (`story:incident-call-lookup`) answered. A tracing
backend is a later source if one is configured on the host.

## Open before design

- Which labels or line fields carry the call identifier in each source, read from the host's
  configuration and a sample, not assumed.
- Whether the operation is a fixed query template over existing Loki reads or its own operation.

## Acceptance

- A fixture-backed test answers the lines of one call from two sources in time order, bounded by
  the window, and none of another call's.
- The output schema is published in `operations describe`.
