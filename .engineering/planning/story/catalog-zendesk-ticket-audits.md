---
format: aep.planning-md/3
id: story:catalog-zendesk-ticket-audits
kind: story
status: implemented
title: Zendesk ticket audits through the catalog provider
relations:
- decomposes: epic:catalog-knowledge-sources
- informed_by: story:catalog-zendesk-reads
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-29T23:23:33Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-29T23:23:33Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-30T03:03:08Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
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
- Paged by cursor with `page[size]` and `page[after]`. `sort_order` is withheld, as in the
  connectors-definitions operation; the upstream operation has no `per_page`.
- Description, kept identical to the connectors-definitions operation (D239, 2026-10-30): "List one
  ticket's audits by cursor, one page per call; the walk sends meta.after_cursor as page[after].
  When meta.has_more is present it decides: false ends the walk, true continues even past an empty
  page and is an error without meta.after_cursor. Without meta.has_more the walk ends on a page with
  no meta.after_cursor or no audits. A cursor seen before is an error." `ticket.comments` gets the
  same rule, with "comments".
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
- A walk that follows the description against the stand-in:
  - ends on a page with `meta.has_more` false;
  - continues past an empty page with `meta.has_more` true and a cursor;
  - is an error on `meta.has_more` true without `meta.after_cursor`;
  - without `meta.has_more`, ends on a null cursor or an empty page;
  - is an error on a repeated cursor.
- The generated bundle `adapters/catalog/generated/bundles/zendesk.bundle.json` has no drift, and
  `docs/catalog-zendesk.md`, the website's Zendesk page and the CHANGELOG name the operation.
