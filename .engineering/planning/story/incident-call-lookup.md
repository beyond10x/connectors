---
format: aep.planning-md/3
id: story:incident-call-lookup
kind: story
status: draft
title: Look up a call by its number, one typed line per leg
relations:
- decomposes: epic:incident-collection-workflow
- serves: vision:independent-contract-adapters
revision: 2
---
## Outcome

A read operation looks up a call by its number and answers one typed line per leg: direction,
caller and callee numbers (masked), start and end, the call identifier and the leg's type. It is
callable through `connectors operations describe` and `invoke` with no mutation profile, so a
workflow reads it without a model.

## Source

The SIP proxy's CDR lines in Loki, joined to the platform's call table read with SQL on a read
replica, matched on direction, numbers and a time window (see `epic:incident-collection-workflow`,
"A call by its number"). Homer SIP capture is a later source, once its credential profile exists.

What the join must handle, as an existing operator tool found:

- a transfer leg writes no call-table row, and the CDR line carries no leg type, so neither source
  alone lists every leg with its type;
- the SQL read returns at most 100 rows whatever the query's `LIMIT` says, so the operation pages;
- end-user numbers are masked by default and stay masked in the typed output.

## Decided before design

- Two typed reads, joined by the calling workflow; no composite operation. Each read is callable
  through `operations describe/invoke` without a mutation profile and returns typed output that
  carries the join key (the call identifier), so a deterministic workflow step joins them without a
  model. A composite operation would be a new runtime noun and stays out until two consumers need it.
- Each read binds to a configured connection the caller names by connection id (one Loki, one SQL);
  nothing is discovered from the host configuration implicitly.

## Acceptance

- A fixture-backed test answers every leg of a transferred call, with its type, from the two
  sources, across a page boundary of the SQL read.
- The output schema is published in `operations describe`, and numbers are masked.
