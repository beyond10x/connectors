---
format: aep.planning-md/3
id: decision-blocker:zendesk-draft-reply-shape
kind: decision-blocker
status: cleared
title: What a Zendesk draft reply is, with no draft endpoint upstream
relations:
- blocks: story:catalog-zendesk-ticket-writes
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-17T06:47:40Z", actor: "human:timo", revision: 3}
---
## Question

What is a Zendesk "draft reply" written through the Connectors CLI? The pinned Zendesk Support document has no draft object or endpoint.

## Options

- (a) A private note marked as a draft (`public: false`, a fixed first line), which an agent copies into the public reply. Cost: one more write id; the draft sits in the ticket's history.
- (b) No Zendesk write: the assistant keeps the reply text and shows it to the agent. Cost: none here; the draft is outside Zendesk.
- (c) Leave draft replies out.

## Recommendation

(a): it keeps the draft on the ticket where the agent works, and it reuses `ticket.note`'s guard, so it adds little.

## Decided (2026-10-17)

(a). A draft reply is a private (internal) note whose first line is a fixed draft marker; a person copies it into the public reply. It is one more write id, `ticket.draft_reply`, with the same `safe_update` guard and approval as `ticket.note`, and it never sends anything to the requester. Why: the person who sends the reply works in Zendesk, so the draft belongs where they read it; dropping the one id undoes it.
