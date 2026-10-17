---
format: aep.planning-md/3
id: decision-blocker:zendesk-draft-reply-shape
kind: decision-blocker
status: open
title: What a Zendesk draft reply is, with no draft endpoint upstream
relations:
- blocks: story:catalog-zendesk-ticket-writes
revision: 1
---
## Question

What is a Zendesk "draft reply" written through the Connectors CLI? The pinned Zendesk Support document has no draft object or endpoint.

## Options

- (a) A private note marked as a draft (`public: false`, a fixed first line), which an agent copies into the public reply. Cost: one more write id; the draft sits in the ticket's history.
- (b) No Zendesk write: the assistant keeps the reply text and shows it to the agent. Cost: none here; the draft is outside Zendesk.
- (c) Leave draft replies out.

## Recommendation

(a): it keeps the draft on the ticket where the agent works, and it reuses `ticket.note`'s guard, so it adds little.
