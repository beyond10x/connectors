---
format: aep.planning-md/3
id: story:catalog-zendesk-ticket-writes
kind: story
status: draft
title: Zendesk ticket update and internal note through the catalog provider, behind approvals
relations:
- depends_on: story:catalog-zendesk-reads
- depends_on: story:catalog-github-reads
scope:
- confidence: cited
  path: adapters/catalog/generated/bundles/index.json
- confidence: cited
  path: adapters/catalog/generated/bundles/zendesk.bundle.json
- confidence: cited
  path: adapters/catalog/providers/zendesk/operations.json
- confidence: cited
  path: adapters/catalog/spec/ess/domains/guard.yaml
- confidence: cited
  path: adapters/catalog/spec/ess/domains/selection.yaml
- confidence: cited
  path: adapters/catalog/src/lib.rs
- confidence: inferred
  path: adapters/catalog/src/local.rs
- confidence: cited
  path: adapters/catalog/tests/zendesk.rs
- confidence: cited
  path: contracts/catalog/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-core/src/lib.rs
- confidence: inferred
  path: crates/connectors-core/src/v1alpha2.rs
- confidence: cited
  path: crates/connectors-docs/src/status.rs
- confidence: cited
  path: docs/catalog-zendesk.md
- confidence: cited
  path: spec-kinds/adapter/v2/schema.json
- confidence: cited
  path: spec-kinds/adapter/v2/semantics.md
revision: 18
---
## Outcome

The assistant repository calls reviewed Zendesk Support writes through the Connectors CLI (`operations invoke` after `approvals prepare|issue`), on the existing `zendesk` catalog provider and its `zendesk.basic` profile, so it triages a ticket without a second Zendesk client. The assistant only calls; it holds no Zendesk credential.

## Operations (each `effect: write`, each behind an approval)

| id | `operationId` | endpoint | body |
|---|---|---|---|
| `ticket.update` | `UpdateTicket` | `PUT /api/v2/tickets/{ticket_id}` | `ticket.group_id`, `ticket.tags`, `ticket.priority`, `ticket.type`; at least one present |
| `ticket.note` | `UpdateTicket` | `PUT /api/v2/tickets/{ticket_id}` | `ticket.comment.body`, `ticket.comment.public` fixed `false` |
| `ticket.draft_reply` | `UpdateTicket` | `PUT /api/v2/tickets/{ticket_id}` | `ticket.comment.body` behind a fixed draft-marker first line, `ticket.comment.public` fixed `false` |

- Lost updates are refused, not applied: a preflight `ShowTicket` reads `updated_at`, and the write sends `safe_update: true` with `updated_stamp`; Zendesk's 409 is `refused` code `conflict`.
- `ticket.tags` replaces the ticket's tag set, as the pinned document says; the description says so. Additive tagging (`PUT /api/v2/tickets/{ticket_id}/tags`) is out of this story unless the assistant needs it.
- `priority` and `type` admit only the pinned document's enum values; anything else is `invalid_input` naming the field, before anything is sent.
- A note sent twice is two notes (no compare before an append); the description says so. A timeout after dispatch is `outcome_unknown`, never re-sent.

## Draft reply (decided 2026-10-17)

The pinned document has no draft object or endpoint. A draft reply is a private note whose first line is a fixed draft marker, which a person copies into the public reply; same guard and approval as `ticket.note`, nothing sent to the requester.

## Engine prerequisites (scoped 2026-10-17)

The catalog write shape cannot carry these writes today (`adapters/catalog/src/lib.rs`): `body_keys` closes top-level keys only and `ValueType` has no object; `body_fixed` fixes top-level keys only; enums exist for bounded parameters only; writes map 409 to `invalid_input` and `ErrorCode` has no conflict. Each is modelled first in `adapters/catalog/spec/ess/domains/{selection,guard}.yaml` and `spec-kinds/adapter/v2`, then implemented:

1. Nested closed bodies: body keys, types and required/non-empty rules addressed by JSON pointer (`/ticket/group_id`, `/ticket/comment/body`).
2. Fixed values at a pointer (`/ticket/comment/public` = false, `/ticket/safe_update` = true), refused if the caller sends them.
3. An enum on a body value (`/ticket/priority`, `/ticket/type`), refused as `invalid_input` naming the field.
4. A generic `conflict` refusal code in the shared contract (`connectors-core` `ErrorCode`): the provider answered that the target changed since it was read; nothing was applied. A write declares which status maps to it (Zendesk 409).

The caller passes `updated_stamp` (read from `ticket.show`); the preflight `ShowTicket` check compares it with `/ticket/updated_at` and refuses before dispatch on a mismatch, so the 409 only covers the window between the preflight and the write.

## Acceptance

- `operations.json` adds exactly the ids above with `effect: write`; `adapters/catalog/tests/zendesk.rs` pins the id list, `operationId`s and paths.
- Recorded-fixture tests per operation assert the exact request (method, path, body including `safe_update` and `updated_stamp`, auth header) and the answer; a 409 fixture yields `refused`/`conflict` and no retry; an out-of-enum priority is `invalid_input` naming `priority` with no request sent.
- `ticket.note` and `ticket.draft_reply` cannot send `public: true`, and the draft marker is always the first line: tests prove both fields are fixed.
- No write runs without an issued approval: a test invokes each id unapproved and gets the refusal.
- `docs/catalog-zendesk.md` documents each write, its guard and the move steps for a local install (config block and commands), with the release link.
- Bundle drift reproduces the Zendesk bundle byte for byte.
