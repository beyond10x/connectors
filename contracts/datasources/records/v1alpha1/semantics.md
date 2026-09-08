# datasource.records/v1alpha1 — `document` profile

- **Status:** proposed, not implemented.
- **Base contract:** `datasource.records/v1alpha1` as implemented in [service v1alpha1](../../../service/v1alpha1/semantics.md): bounded pages (`Page { items, next_cursor, complete, provenance }`, `crates/connectors-contracts/src/lib.rs:15-22`), cursors bound to operation, input, instance, configuration identity and expiry (`crates/connectors-sdk/src/lib.rs:161`), 1–100 records, 300 s cursor TTL. This document adds one profile family and does not restate the base.
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `datasource.records/v1alpha1` |
| New profile family | `document` — a single-item read whose payload is a content body with a declared representation, a version, and a byte bound |
| Existing profile family | `list` — the current paged reads (`kubernetes-list`, `gitlab-*`) |
| Named profiles | `confluence-page`, `jira-issue`; later `gitlab-file` could be re-expressed here |

The design keeps typed profiles for record and document reads under shared paging, provenance and completeness semantics (`docs/design.md:434`) and forbids silently equating a Jira issue with a Confluence document (`docs/design.md:452`). A document read differs from a list page in three ways that the base cannot express: a body representation that is provider-native and must be named, a version that identifies the content read, and a body byte bound with explicit truncation.

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| `confluence-page-get`: one page by id; body returned as Confluence storage format (XHTML-like) because the operation pins `expand = body.storage,version`; omitting `expand` returns no body | `../connectors/providers/confluence.toml:510-548` | preserve: representation `confluence.storage`, version from `version.number` |
| `confluence-space-pages`: page bodies are not returned; one page at a time | `providers/confluence.toml:461-510` | preserve the list/detail split (`docs/design.md:448`) |
| `confluence-page-search`: CQL with `expand = version,body.storage,space`, cursor, limit ≤ 100; dates in the account's timezone | `providers/confluence.toml:708-740` | change: search stays a `list` profile with a declared projection that may include bodies under the same byte bound per item |
| `jira-issue-get`: full default field set including custom fields; ids are strings | `providers/jira.toml:321-340` | preserve as `jira-issue` document: representation `jira.fields.v2`, body is the `fields` object |
| Jira v2 wiki markup versus v3 ADF; ADF not expressible in the old schema; v2 chosen | `providers/jira.toml:18-60` (header comment) | preserve the decision as a representation name: `jira.wiki` for v2 text fields; `jira.adf` reserved for a later v3 profile |
| `ConnectorDatasource v0alpha1`: credential-free, closed `list`/`get` verbs, value projection identity, opaque expiring cursors | `../connectors/contracts/connector-datasource/v0alpha1/README.md` | preserve the `get` verb as this profile; value-projection digests become the `representation` and descriptor revision |

## 3. Types

Input (per operation; example `confluence-page`):

```json
{ "id": "123456", "representation": "confluence.storage", "max_body_bytes": 262144 }
```

Output:

```json
{
  "item": { "id": "123456", "title": "…", "status": "current", "version": { "number": 7, "when": "2026-09-01T10:00:00.000Z" } },
  "body": { "representation": "confluence.storage", "bytes": 12000, "content": "<p>…</p>", "truncated": false },
  "complete": true,
  "provenance": { "instance": "…", "resource": "confluence:page:123456", "observed_at_unix_ms": 0, "source_revision": "7" }
}
```

Rules on fields:

| Field | Rule |
|---|---|
| `representation` | closed per profile: `confluence-page` offers `confluence.storage` (later `confluence.view`); `jira-issue` offers `jira.fields.v2` |
| `body.content` | UTF-8 string of the provider representation; binary attachments are not documents (deferred) |
| `body.bytes` | length of the full provider body before truncation |
| `body.truncated` | true when `content` was cut at `max_body_bytes`; a truncated body is still a valid result (`docs/design.md:963`) |
| `version` | provider version identity; `provenance.source_revision` repeats it |
| `complete` | false only when the provider indicated a partial object (never used to hide truncation) |

Errors: base codes; `NotFound` for a missing id; `Forbidden` when the id is outside the configured spaces/projects (checked before dispatch); `InvalidInput` for an unknown representation.

## 4. Rules

- Scope before dispatch: configured allowlists (spaces, projects) apply to the id before any provider call, as for GitLab project allowlists today.
- One item per read; no cursor. A document read is not paged; large bodies are truncated with `truncated: true`, and a follow-up range read is a later profile decision.
- Representation is explicit in input and echoed in output. The adapter never converts between representations (no HTML→Markdown), so the descriptor stays truthful about what the provider returned.
- Version identity: `version` is copied from the provider; the adapter does not synthesize one. If the provider returns none, `version` is null and `source_revision` is null.
- List/detail projections may differ and the difference is declared in the two operations' output schemas (`docs/design.md:448`).
- Cache: a document is cacheable by (connection, id, representation, version) only when the provider returned a version; freshness rules from `docs/design.md:511-518` apply; a cached body must carry `provenance.observed_at_unix_ms` of the original read.

## 5. Limits

| Concern | Rule |
|---|---|
| `max_body_bytes` | caller-chosen ≤ profile ceiling; first-profile ceiling 1 MiB, below the 4 MiB response limit |
| Deadline | base provider deadline |
| Result size | the full response respects the base result limit |

## 6. Conformance scenarios (`docs/design.md:988`, schema fidelity)

- Fixture body of 300 KiB read with `max_body_bytes` 256 KiB → `truncated: true`, `bytes: 307200`, content is a UTF-8 prefix not split inside a code point.
- Fixture with no `version` → null version, `complete: true`.
- Unknown representation → `InvalidInput`, no dispatch.
- Id outside the configured space → `Forbidden`, fixture sees zero requests.
- Jira issue with custom fields → all fields present under `fields`; none renamed.
- Confluence storage body containing `<script>` → returned verbatim; the contract does not sanitize (consumers do).

## 7. Compatibility

- Additive: new operations on the Atlassian adapter; no change to existing `list` operations or the wire.

## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| Shared `Document<T>` type beside `Page<T>` | `crates/connectors-contracts/src/lib.rs` |
| UTF-8-safe truncation helper | `crates/connectors-sdk` |

## 9. ESS entities

| Entity / value | Notes |
|---|---|
| `OperationDeclaration.profile = document` with `representations` list | value on the existing entity |
| Provider content objects (page, issue) | not modeled as entities; they are provider-native payloads with declared schemas (`docs/design.md:448`) |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Ceiling for `max_body_bytes` | 1 MiB |
| Jira v3/ADF profile | deferred; `jira.adf` reserved |
| Attachments | deferred (binary documents need a streaming profile) |
