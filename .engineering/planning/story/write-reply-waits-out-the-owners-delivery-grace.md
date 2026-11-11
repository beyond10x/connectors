---
format: aep.planning-md/3
id: story:write-reply-waits-out-the-owners-delivery-grace
kind: story
status: draft
title: A write reply the owner sends within its delivery grace reaches the CLI
summary: The write client stops reading at the write deadline although the owner may reply up to 5 s later, so a finished write reads as outcome_unknown
tags:
- from-finding
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
- informed_by: story:owner-errors-are-readable
revision: 1
---
## Defect

Found while implementing story:owner-errors-are-readable on 2026-11-10. The defect was already
there before that work; it is the cause that story reasoned from the code for the intermittent
guarded write `outcome_unknown`.

`WriteClient::invoke` (`crates/connectors-host/src/local/owner/transport.rs`, `impl WriteClient`)
reads the owner's write reply only until the write's own deadline (`until`), and maps any send or
read failure to `outcome_unknown`. The owner is explicitly allowed to deliver a terminal write
reply up to 5 s past that deadline (`exchange_write`: "This bounded delivery grace grants no
native dispatch"). The read client already waits `ANSWER_GRACE` (2 s) past its deadline for the
same reason; the write client waits for nothing.

Under load, a guarded write whose owner uses nearly the whole budget sends a `Done` or `Failed`
reply after the client stopped reading. The CLI then reports `outcome_unknown` at stage
`publication` with `attempt: null` and `source_audit` unavailable, although the owner finished
and knows the outcome.

## Acceptance

- The write client reads the owner's reply for a bounded grace past the write deadline that is at
  least the owner's delivery grace; both sides take it from one shared constant.
- A test whose owner replies after the write deadline but within the grace gets the owner's real
  outcome (`Done` or the owner's failure code), not `outcome_unknown`.
- A reply that arrives after the grace, or never, is still `outcome_unknown`, and the write is
  still not retried.
