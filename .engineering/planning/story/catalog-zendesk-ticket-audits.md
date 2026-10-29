---
format: aep.planning-md/3
id: story:catalog-zendesk-ticket-audits
kind: story
status: active
title: Zendesk ticket audits through the catalog provider
relations:
- decomposes: epic:catalog-knowledge-sources
- informed_by: story:catalog-zendesk-reads
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-29T23:23:33Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-29T23:23:33Z", actor: "human:timo", revision: 3}
---
## Problem

The built-in catalog reads a Zendesk ticket's comments (`ticket.comments`, story:catalog-zendesk-reads),
but it does not read the ticket's audits. A consumer that reconstructs a ticket's history needs each
audit, and it is blocked on that today (orchestrator item D235, 2026-10-30).

## Request

Add a read operation `ticket.audits` to the built-in catalog's `zendesk` provider, modelled like
`ticket.comments`.

## Outcome

`ticket.audits` reads `GET /api/v2/tickets/{ticket_id}/audits` (upstream operationId
`ListAuditsForTicket` in `adapters/zendesk/upstream/zendesk-support.yaml`). The orchestrator cited
`ListTicketAudits`, but that operation is `GET /api/v2/ticket_audits`, the audits of every ticket.
The path it named is `ListAuditsForTicket`'s.

- Effect `read`, approval `not_required`.
- Paged by cursor with `page[size]` and `page[after]`. The fields `ticket.comments` withholds are
  withheld here too (`per_page`, `sort_order`).
- Description, kept identical to the connectors-definitions operation: "List one ticket's audits by
  cursor, one page per call; the walk sends meta.after_cursor as page[after] and ends on a page with
  meta.has_more false".
- The response projection keeps, for each audit:
  - `author_id`
  - `created_at`
  - `via.channel` and `via.source.rel`
  - the events, including field changes
  - the id of the comment the audit carried

## Acceptance

- `operations list` on a `zendesk` instance lists `ticket.audits` as a read that needs no approval,
  with that description.
- An invoke of `ticket.audits` against a stand-in Zendesk sends
  `GET /api/v2/tickets/<id>/audits` with `page[size]`, then `page[after]` from `meta.after_cursor`.
  Its result keeps the author, created_at, via channel, via.source.rel, the field-change events and
  the comment event's id.
- The generated bundle `adapters/catalog/generated/bundles/zendesk.bundle.json` has no drift, and
  `docs/catalog-zendesk.md`, the website's Zendesk page and the CHANGELOG name the operation.
