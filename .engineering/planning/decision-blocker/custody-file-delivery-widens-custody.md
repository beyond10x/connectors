---
format: aep.planning-md/3
id: decision-blocker:custody-file-delivery-widens-custody
kind: decision-blocker
status: cleared
title: Nobody has decided whether a custody credential may be delivered to an owner-only file
relations:
- blocks: story:custody-credential-file-delivery
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "open", to: "cleared", at: "2026-10-23T20:45:34Z", actor: "human:timo", revision: 5}
---
## Question

May a credential held in custody leave Connectors through one admitted delivery to an exact
owner-only file (story:custody-credential-file-delivery)? The accepted outcome of
story:openrouter-management-provider says a created key "never leaves Connectors", and the
mounted-secret custody threat model admits no value in any output.

## Options

| option | what | cost |
|---|---|---|
| A | Accept a file delivery path: an architecture decision records it (approval, exact path, 0600, atomic write, revision-guarded overwrite, receipt of path and revision only) and amends the two records; the story is then scoped and implemented. | A first disclosure path out of custody; its threat model has to be written and reviewed. |
| B | Keep custody closed: the consumer reads keys through Connectors instead (an invoke that proxies the upstream call, or a consumer launch that hands the credential to a child process). | The consumer has to change how it reads upstream credentials. |

Recommended: A, scoped to one connection per delivery and refused for mounted custody.

Decider: operator

Credential custody is on the operator never-list, so only the operator decides this (directing session, 2026-10-23). It is row 41 of the operator queue.

## Decision

Decided 2026-10-23 by the operator, option A. The directing session `b10x-chef-orchestrator` relayed the answer verbatim with its date: "A A" (A for this blocker, A for decision-blocker:mcp-tool-call-without-revision). Its instruction for this item:

> custody-file-delivery-widens-custody, A. An architecture decision records the file delivery path: approval, exact path, 0600, atomic write, revision-guarded overwrite, and a receipt of path and revision only. It amends the two records, and then the story is scoped and implemented. Use your recommendation's scope: one connection per delivery, refused for mounted custody. The threat model is written and reviewed before the code.

What follows from it:

- An architecture decision records the delivery path and amends the accepted outcome of story:openrouter-management-provider and the mounted-secret custody threat model.
- Scope: one connection per delivery; a delivery from mounted custody is refused.
- The threat model for the delivery is written and reviewed before any code of story:custody-credential-file-delivery.
- The story is then scoped and implemented in wave C.
