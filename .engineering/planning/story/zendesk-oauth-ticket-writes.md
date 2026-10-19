---
format: aep.planning-md/3
id: story:zendesk-oauth-ticket-writes
kind: story
status: draft
title: The Zendesk OAuth profile can run the ticket writes
summary: zendesk.oauth asks read only; API tokens end for new accounts and stop on 2027-04-30
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: adapters/catalog/providers/zendesk/connector.json
- confidence: inferred
  path: adapters/catalog/tests/connector_declarations.rs
- confidence: cited
  path: adapters/catalog/tests/hand_written/catalog-zendesk.0.json
- confidence: cited
  path: adapters/catalog/tests/zendesk.rs
- confidence: cited
  path: crates/connectors-catalog/tests/fixtures/definitions/connectors/zendesk.yaml
- confidence: inferred
  path: crates/connectors-docs/src/status.rs
- confidence: cited
  path: docs/catalog-zendesk.md
- confidence: inferred
  path: website/docs/connectors/zendesk.md
revision: 4
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

## Scope

Derived 2026-10-19 by `story-scoper` on origin/main 0c6cbba7f. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `adapters/catalog/providers/zendesk/connector.json:17` (`requested_scopes: ["read"]`) — cited, story Why now
- **Files:** `docs/catalog-zendesk.md:208-209,278,289,297-299` — cited
- **Files:** `CHANGELOG.md:160-161` (Limitations: writes are basic-only) — cited
- **Files:** `adapters/catalog/tests/zendesk.rs:422-431` (asserts `["read"]`), plus a new OAuth `ticket.update` fixture test — cited; acceptance line 2
- **Files:** `adapters/catalog/tests/hand_written/catalog-zendesk.0.json:15` — cited
- **Files:** `crates/connectors-catalog/tests/fixtures/definitions/connectors/zendesk.yaml:13`, lowered byte for byte by `definition_import.rs:1-9,367` — cited; acceptance line 1
- **Also likely:** `adapters/catalog/tests/connector_declarations.rs:769-770` (pinned `configuration_revision`) — inferred
- **Also likely:** a token-endpoint route in the `zendesk.rs` fixture; `adapters/catalog/tests/local_runtime/oauth2_client_credentials.rs` has the exchange infrastructure — inferred
- **Also likely:** `website/docs/connectors/zendesk.md:93,111`, `crates/connectors-docs/src/status.rs:352` — inferred
- **Outside this tree:** `github.com/beyond10x/connectors-definitions` `connectors/zendesk.yaml` — cited ("the definitions repository mirrors it")
- **Not touched:** `adapters/catalog/src/**` — the engine places no scheme limit on writes (`adapters/catalog/src/local.rs:1308-1318`, `:977-994`) — inferred
- **Confidence:** high for the data and test files; medium for the also-likely lines
- **Would collide with:** `story:catalog-zendesk-ticket-writes` (`docs/catalog-zendesk.md`, `adapters/catalog/tests/zendesk.rs`, `status.rs`), the definitions fixture tree, the documented-config revision pins in `connector_declarations.rs`
- **Not implementable headless:** acceptance line 3 (one live run) needs a Zendesk account and an OAuth client; the scope value (`read write` or per-resource) is a security choice the live run settles
