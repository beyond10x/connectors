---
format: aep.planning-md/3
id: story:catalog-zendesk-reads
kind: story
status: draft
title: Zendesk tickets, users and organizations through the catalog provider
relations:
- decomposes: epic:catalog-knowledge-sources
- depends_on: story:catalog-basic-auth-profile
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: adapters/README.md
- confidence: cited
  path: adapters/catalog/generated/bundles/index.json
- confidence: cited
  path: adapters/catalog/generated/bundles/zendesk.bundle.json
- confidence: cited
  path: adapters/catalog/providers/zendesk/operations.json
- confidence: cited
  path: adapters/catalog/tests/bundle_drift.rs
- confidence: cited
  path: adapters/catalog/tests/zendesk.rs
- confidence: cited
  path: adapters/zendesk/upstream/README.md
- confidence: cited
  path: adapters/zendesk/upstream/zendesk-source-hashes.json
- confidence: cited
  path: adapters/zendesk/upstream/zendesk-support.yaml
- confidence: cited
  path: crates/connectors-build/src/gate.rs
- confidence: cited
  path: crates/connectors-build/src/main.rs
- confidence: cited
  path: crates/connectors-build/src/source_hashes.rs
- confidence: cited
  path: crates/connectors-build/src/upstream_redaction.rs
- confidence: cited
  path: crates/connectors-catalog/src/inventory.rs
- confidence: cited
  path: crates/connectors-catalog/tests/inventory.rs
- confidence: cited
  path: docs/catalog-zendesk.md
- confidence: cited
  path: docs/local-catalog-provider.md
revision: 11
---
## Source

The official Zendesk Support API OpenAPI document, linked as "Download OpenAPI file" from Zendesk's Support API reference: <https://developer.zendesk.com/zendesk/oas.yaml>, retrieved 2026-10-05, upstream SHA-256 `3a477ea89b274f4d3731f1c7ff93dc93d4520de871ac759b06d3297798fd685d` (1,932,113 bytes), OpenAPI `3.0.3`, `info.version` `2.0.0`. Its examples carry example credentials and real-looking email addresses and phone numbers, so it is pinned redacted, not as served: `connectors-build redact` (rule `upstream-redaction/2`, `crates/connectors-build/src/upstream_redaction.rs`) replaced 40 credential values with `example-redacted-token`, 74 addresses with `user@example.com` and 5 numbers with `+15555550100`, giving `adapters/zendesk/upstream/zendesk-support.yaml`, SHA-256 `5dd6cf1eda8febf02f8fb2f2cfa72d7cb6bc7d5bab188ba97b49ab55fc51dbf2` (1,931,767 bytes), on which gitleaks 8.30.1 reports no finding. Rule `/1` (addresses and numbers only) left 14 gitleaks `generic-api-key` findings that the hooks refused. The upstream bytes and any gzip copy are not committed; the gate checks the redacted digest and that the rule would change nothing more (coordinator instruction, 2026-10-05). Provider id `zendesk`. Auth: basic with `<email>/token:<api token>` (`story:catalog-basic-auth-profile`), profile `zendesk.basic`, identity `GET /api/v2/users/me` (`ShowCurrentUser`), subject `/user/id`.

Revision 6 (operator request of 2026-10-05): the read set is tickets, users and organizations, the third entity linking the first two.

## Operations (read-only)

| id | `operationId` | endpoint | paging | end condition | time filter |
|---|---|---|---|---|---|
| `tickets.incremental` | `IncrementalTicketExportCursor` | `GET /api/v2/incremental/tickets/cursor` | `start_time` then `cursor` | `end_of_stream: true` | `start_time` (changed since) |
| `ticket.show` | `ShowTicket` | `GET /api/v2/tickets/{ticket_id}` | single item | n/a | none |
| `ticket.comments` | `ListTicketComments` | `GET /api/v2/tickets/{ticket_id}/comments` | `page[size]`, `page[after]` | `meta.has_more: false` | none; deltas come from `tickets.incremental` |
| `users.incremental` | `IncrementalUserExportCursor` | `GET /api/v2/incremental/users/cursor` | `start_time` then `cursor` | `end_of_stream: true` | `start_time` (changed since) |
| `user.show` | `ShowUser` | `GET /api/v2/users/{user_id}` | single item | n/a | none |
| `organizations.incremental` | `IncrementalOrganizationExport` | `GET /api/v2/incremental/organizations` | `start_time`, next from `end_time` | `end_of_stream: true` | `start_time` (changed since) |
| `organization.show` | `ShowOrganization` | `GET /api/v2/organizations/{organization_id}` | single item | n/a | none |

The pinned document has no cursor export for organizations; `organizations.incremental` is its time-based export.

## Engine prerequisites found

The pinned document declares its parameters under `components.parameters` and references them, and the inventory named every parameter `$ref` unsupported (760 gaps; the operations above would have had no parameters). `crates/connectors-catalog/src/inventory.rs` now follows local `#/components/parameters/<name>` references and expands an exploded `deepObject` query parameter over scalar properties into `name[property]` parameters, which the comment cursor (`page[size]`, `page[after]`) needs. No other committed bundle changes; 35 Zendesk gaps remain (`deepObject` parameters this pass does not expand, none shipped).

## Shared surfaces

- Own files: `adapters/catalog/providers/zendesk/operations.json`, `adapters/catalog/generated/bundles/zendesk.bundle.json`, `docs/catalog-zendesk.md`, `adapters/catalog/tests/zendesk.rs`, and the pinned source under `adapters/zendesk/upstream/`.
- Shared: `adapters/catalog/generated/bundles/index.json` and `adapters/catalog/tests/bundle_drift.rs`, each gaining one Zendesk row; `crates/connectors-catalog/src/inventory.rs` and its tests.
- Dependency change (revision 6): the `depends_on story:catalog-grafana-reads` edge existed only to serialize the shared `index.json`. Grafana is still draft, so this story adds its own row on the current index instead and no longer depends on it; whichever of the two lands second adds its row beside the other's.

## Acceptance

- `operations.json` exposes exactly the seven ids above, each `effect: read`; `adapters/catalog/tests/zendesk.rs` pins that exact id list with each id's `operationId` and path, so a renamed or dropped id fails the gate.
- `operations.json` loads against the committed bundle, which refuses any `operation_id` the pinned document lacks; `docs/catalog-zendesk.md` cites, per operation, the pinned document's `operationId` and path.
- `docs/catalog-zendesk.md` states, per list operation, its paging parameters, its end condition and its time-window filter (or how deltas are taken where there is none).
- Per operation, a recorded-fixture test asserts the exact request the provider sends (path, query including the time filter, auth header) and that `operations invoke` returns the fixture body byte-identical. Fixtures are synthetic, with `example.com` addresses only.
- For every list operation in the table, a paging test walks two fixture pages and asserts the walk stops at that operation's documented end condition.
- `adapters/catalog/tests/bundle_drift.rs` reproduces this provider's bundle byte for byte; no live credential is needed for the gate.
