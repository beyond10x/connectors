---
format: aep.planning-md/3
id: story:zendesk-oauth-ticket-writes
kind: story
status: draft
title: The Zendesk OAuth profile can run the ticket writes
summary: zendesk.oauth asks read only; API tokens end for new accounts and stop on 2027-04-30
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

The `zendesk.oauth` profile requests a scope that covers the ticket writes shipped in 0.54.0, so the
writes run on an OAuth connection, not only on `zendesk.basic` (API token).

## Why now

`adapters/catalog/providers/zendesk/connector.json:17` asks `["read"]` only; CHANGELOG.md:160-161 and
`docs/catalog-zendesk.md:208-209` record that the 0.54.0 Zendesk writes run only on the API-token
profile. Accounts created on or after 2026-07-28 cannot use API tokens, existing accounts cannot
create new ones after 2026-10-27, and every API token stops working on 2027-04-30
(`docs/catalog-zendesk.md:265-267`), so from those dates an operator has no profile that can write. Release review r6 F2 (`updated_at` compared to
the whole second) is untested and needs a live account.

## Acceptance

- `zendesk.oauth` requests the write scope Zendesk documents for tickets (cite the pinned document);
  the definitions repository mirrors it.
- `ticket.update` runs on an OAuth connection against a fixture.
- One live run on a Zendesk account confirms the scope and settles r6 F2; its output is recorded as
  evidence.
