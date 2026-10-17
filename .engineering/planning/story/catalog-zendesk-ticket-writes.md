---
format: aep.planning-md/3
id: story:catalog-zendesk-ticket-writes
kind: story
status: draft
title: Zendesk ticket update and internal note through the catalog provider, behind approvals
relations:
- depends_on: story:catalog-zendesk-reads
revision: 1
---
## Outcome

The assistant repository calls reviewed Zendesk Support writes through the Connectors CLI (`operations invoke` after `approvals prepare|issue`), on the existing `zendesk` catalog provider and its `zendesk.basic` profile, so it triages a ticket without a second Zendesk client. The assistant only calls; it holds no Zendesk credential.

## Operations (each `effect: write`, each behind an approval)

| id | `operationId` | endpoint | body |
|---|---|---|---|
| `ticket.update` | `UpdateTicket` | `PUT /api/v2/tickets/{ticket_id}` | `ticket.group_id`, `ticket.tags`, `ticket.priority`, `ticket.type`; at least one present |
| `ticket.note` | `UpdateTicket` | `PUT /api/v2/tickets/{ticket_id}` | `ticket.comment.body`, `ticket.comment.public` fixed `false` |
| `ticket.draft_reply` | open, see below | | |

- Lost updates are refused, not applied: a preflight `ShowTicket` reads `updated_at`, and the write sends `safe_update: true` with `updated_stamp`; Zendesk's 409 is `refused` code `conflict`.
- `ticket.tags` replaces the ticket's tag set, as the pinned document says; the description says so. Additive tagging (`PUT /api/v2/tickets/{ticket_id}/tags`) is out of this story unless the assistant needs it.
- `priority` and `type` admit only the pinned document's enum values; anything else is `invalid_input` naming the field, before anything is sent.
- A note sent twice is two notes (no compare before an append); the description says so. A timeout after dispatch is `outcome_unknown`, never re-sent.

## Open: draft reply

The pinned Zendesk Support document has no draft object or endpoint (its only "draft" is the account setting `default_to_draft_mode`). A draft reply needs a decision before it is built: (a) a private note marked as a draft that an agent copies into a public reply; (b) the reply text returned to the assistant unsent, with no Zendesk write; (c) leave it out. Recorded as a decision-blocker.

## Engine prerequisite to check first

Zendesk wraps the body in `ticket` (and `ticket.comment`); the catalog's write bodies so far are flat (`body_keys`). If the operation shape cannot carry a nested body, model that first in the adapter specification kind (spec first), then the operations.

## Acceptance

- `operations.json` adds exactly the ids above with `effect: write`; `adapters/catalog/tests/zendesk.rs` pins the id list, `operationId`s and paths.
- Recorded-fixture tests per operation assert the exact request (method, path, body including `safe_update` and `updated_stamp`, auth header) and the answer; a 409 fixture yields `refused`/`conflict` and no retry; an out-of-enum priority is `invalid_input` naming `priority` with no request sent.
- `ticket.note` cannot send `public: true`: a test proves the field is fixed.
- No write runs without an issued approval: a test invokes each id unapproved and gets the refusal.
- `docs/catalog-zendesk.md` documents each write, its guard and the move steps for a local install (config block and commands), with the release link.
- Bundle drift reproduces the Zendesk bundle byte for byte.
