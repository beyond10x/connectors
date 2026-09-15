---
format: aep.planning-md/1
id: epic:subscription-custody
kind: epic
status: archived
title: Subscription custody
revision: 5
---
## Provenance

Read from the `epic: subscription-custody` key in 7 file(s) under `docs/stories/`. No source document
exists for this epic itself; the grouping is a frontmatter value and nothing else. Migrated
2026-09-04 by the `aep-planning:story-migration` skill. **read**

## Stories

- `S-070` `story:a-provider-can-hold-a-credential-it-cannot-spend` — done — A provider can hold a credential it cannot spend
- `S-071` `story:claude-code-joins-the-catalog` — done — Claude Code joins the catalog as a custody-only provider
- `S-072` `story:the-anthropic-api-key-arrives-through-a-connect-session` — backlog — The Anthropic API key arrives through a Connect Session
- `S-073` `story:the-hosted-posture-connects-a-catalogued-provider` — backlog — The hosted posture connects a catalogued provider
- `S-074` `story:the-cli-drives-a-hosted-connection` — backlog — The CLI drives a hosted connection
- `S-076` `story:an-agent-attempt-can-lease-a-user-subscription` — done — An agent attempt can lease a user's subscription
- `S-077` `story:claude-subscriptions-connect-with-pkce-and-refresh` — done — Claude subscriptions connect with PKCE and refresh

## Status

`active`, derived from its stories (backlog: 3, done: 4). **inferred** — no source document states a status
for this epic, so the rung follows the work underneath it: all done is `implemented`, any work
started is `active`, nothing started is `draft`.

## Disposition

Archived 2026-09-15 on the acceptance of `docs/design/21-clean-room-rewrite.md` (org-state review
decision sheet 2026-09-15, items 1 and 15; atlas ADR 0051, 2026-09-15).

**Excluded by design 21 §4, fourth row.** "Subscription custody + leases + hosted vault family —
`subscription-custody`, `hosted-vault`, `hosted-secrets`, `hosted-state`; designs 16, 17 — 3,366 LOC —
Drop — design 01 already lists leases as 'designed in the predecessor, never used in anger'." The epic
is that family by name, and its stories are its parts: S-070 a credential held but not spendable, S-076
an agent attempt leasing a user subscription, S-077 PKCE subscription connect and refresh, S-072/S-073/
S-074 the hosted Connect Session and hosted-posture path. §7 D3 confirms the strip list as proposed.

The successor does not carry it: "This slice does not advertise writes, managed OAuth acquisition,
durable events, process execution or media sessions" (`connectors_v2/README.md`), and design 21 §2 keeps
only K7 — "owner-bound secret store, requirements-not-values in catalog" — which is ordinary custody, not
subscription leasing.

Its 7 `docs/stories/S-*` records keep their own statuses (4 `done`, 3 `backlog`). This repository's `AGENTS.md` states no
convention for stories under an archived epic, so none was moved. Archiving keeps the record.
