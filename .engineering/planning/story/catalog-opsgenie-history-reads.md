---
format: aep.planning-md/3
id: story:catalog-opsgenie-history-reads
kind: story
status: draft
title: Opsgenie alerts, incidents and their links over a since window, read only
relations:
- decomposes: epic:incident-collection-workflow
- serves: vision:independent-contract-adapters
revision: 2
---

## Placement: a new story, and a narrowed `story:parity-opsgenie-alerts`

Recommendation: **add** `story:catalog-opsgenie-history-reads` (decomposes
`epic:incident-collection-workflow`, row 5 at `.engineering/planning/epic/incident-collection-workflow.md:31`),
and revise `story:parity-opsgenie-alerts` to depend on it and keep only what it alone owns
(on-call, schedules, ack/close/note writes, `opsgenie.test`). Why:

- The parity story is parked by its own Scheduling section ("Not placed in a wave until a session
  uses the plugin", 0 calls); this work has a deadline (the provider shuts down on 2027-04-05)
  and a different consumer (the incident workflow epic). Extending the parked body would either
  unpark writes nobody asked for or keep the backfill parked.
- Its writes need guards and an approval; after the shutdown they are worthless, while the reads
  are the whole value of a backfill. Mixing them makes one unit that cannot ship in halves.
- The new story ships `alerts.list` and `alert.get`, so the parity rows `opsgenie.alert.list`
  and `opsgenie.alert.get` (`docs/fluxplane-plugin-parity.md:895-896`) move to covered by it; the
  parity story's revised body lists the remaining six and `depends_on` the new story for the
  provider bundle and the credential placement.

## Problem

There is no Opsgenie provider (`docs/fluxplane-plugin-parity.md:889`). The incident workflow
needs alerts, incidents, and which alert belongs to which incident, read over a window a caller
advances run by run, so that history can be copied out before the service closes. Three gaps
stand between the catalog provider and that:

1. **Credential placement.** Opsgenie takes `Authorization: GenieKey <key>` [OAS:
   `securityDefinitions.GenieKey` is `apiKey` in header `Authorization`; docs examples send
   `GenieKey <key>`]. The catalog `token` scheme sends the token bare or prefixed `Bearer `
   only, and a stored token may not contain a space, so `GenieKey <key>` cannot be entered as the
   token either.
2. **The link is not in the document.** `GET /v1/incidents/:identifier/associated-alert-ids`
   [docs] is absent from the published OpenAPI document [OAS: 104 paths, none containing
   `associated`], and an amendment can today add a parameter or correct a path or media type,
   never add an operation.
3. **The document lags the service.** It was last changed 2021-02-19; the list-incidents `sort`
   enum lacks `updatedAt` and `insertedAt`, which the reference documents [OAS vs docs].

## Today (file:line)

- Catalog reads are GET only: `adapters/catalog/src/lib.rs:773-779` [tree].
- Credential schemes: `Scheme` `adapters/catalog/src/local.rs:88-113`; `AuthConfig.header`,
  `bearer` `local.rs:118-127`; scheme/placement validation `local.rs:724-760`; protected entry
  token must be visible ASCII without space `local.rs:226-232` [tree].
- Host transport applies the prefix: `HttpConfig.bearer` `crates/connectors-host/src/http.rs:22`,
  `Bearer ` prepended at `http.rs:362-372` [tree].
- ESS model of the scheme: `connectors_catalog.identity.Scheme`
  `adapters/catalog/spec/ess/domains/identity.yaml:42-45` [tree].
- Amendment kinds (add parameter, correct path, correct media type):
  `adapters/catalog/spec/ess/domains/amendment.yaml:17-80`; pipeline
  `crates/connectors-catalog/src/amendment.rs` [tree].
- A locally authored TOML action reader exists (`crates/connectors-catalog/src/authored.rs:1-8`)
  but no bundle build consumes it (only its own tests import it) [tree].
- Bounds per parameter (`minimum`/`maximum` or `values`): `adapters/catalog/src/lib.rs:308-325`;
  the inventory records no `enum`, so a value outside the document's enum is not refused by the
  engine [tree, inferred from `crates/connectors-catalog/src/inventory.rs` having no enum member].
- Swagger 2.0 is projected to OpenAPI 3.1 by `crates/connectors-catalog/src/swagger.rs`
  (`host` and `schemes` required, `basePath` optional) as for Slack [tree].
- Feed bindings (`datasource.feed/v1alpha1`) cannot express a time filter written in a query
  language, nor offset paging: `contracts/catalog/v1alpha1/semantics.md:176-184` [tree]. Opsgenie's
  window is a `query` clause and its paging is offset, so this is a pass-through read set with a
  caller-held cursor, as Zendesk's incremental exports are
  (`adapters/catalog/providers/zendesk/operations.json`).
- Precedent unit: Zendesk reads `0df7500c4` (bundle, selection, upstream README and digests,
  `tests/zendesk.rs`, `docs/catalog-zendesk.md`, `bundle_drift.rs`, build `main.rs`), then
  `e7576164c` (cited amendment) [tree].

## Upstream source and licence

- Document: `swagger.json` of `github.com/opsgenie/opsgenie-oas` at commit
  `71c55343d03076e5544c484ed32b37c3fbc54037` (2021-02-19), blob
  `ca0c91ad0d9f3153f8288a6d70965fe6c9fc7086`;
  <https://raw.githubusercontent.com/opsgenie/opsgenie-oas/71c55343d03076e5544c484ed32b37c3fbc54037/swagger.json>,
  SHA-256 `60901e17d0d4ba28b8d13d3d78f6a5d329b93008c3c1710dfe6a0b6c786ef071`, 659,535 bytes.
  Swagger `2.0`, `info.title` `Opsgenie REST API`, `info.version` `2.0.0`, host
  `api.opsgenie.com`, scheme `https`, no `basePath`, 170 operations [OAS, fetched 2026-10-13].
- Licence: the document's `info.license` is absent; the repository declares **Apache-2.0**
  (GitHub licence API; `LICENSE` at the repository root). Vendored like Slack's MIT document
  (`adapters/slack/upstream/README.md`): the document unmodified plus `LICENSE` from the same
  commit. This is not the case the Confluence v1 hold turned on (a document with no licence at
  all, `.engineering/planning/story/parity-confluence-cql-search-v1-held.md`).
- Secret scan: no `GenieKey ` value and no e-mail address in the document [probe grep]; gitleaks
  not run. The docs' curl examples carry a sample key, but the docs are not vendored.
- EU accounts use `https://api.eu.opsgenie.com` [docs]; `api_base` is per connection.

## Outcome

A saved `opsgenie` catalog connection reads, with a GenieKey credential and no approval:
alerts and incidents created in a window the caller advances run by run, one page per call; one
alert or incident by id; and the alert ids associated with an incident. The caller holds the
cursor: the last `createdAt` it stored, sent back as the next run's lower bound.

## Operations

| operation id | method, path (operationId) | since-window parameter | pagination |
|---|---|---|---|
| `alerts.list` | GET `/v2/alerts` (`listAlerts`) | `query` = `createdAt >= <epoch ms>` (also `< <epoch ms>` for an upper bound); `sort` held to `createdAt`, `order` held to `asc` by `values` bounds [OAS sort enum; docs search help lists `createdAt` and `lastOccurredAt` as time fields, **not** `updatedAt`] | `offset` (min 0) and `limit` 1..100, default 20 [OAS]; "Sum of offset and limit should be lower than 20K" [docs] — bound `offset` 0..19899; a full window is walked by advancing `createdAt`, never past offset 19,899; response `paging.next` [OAS `PageDetails`]; end: `paging.next` absent [inferred] |
| `alert.get` | GET `/v2/alerts/{identifier}` (`getAlert`) | n/a; used to re-read status of an alert seen earlier | single; `identifierType` id/alias/tiny [OAS] |
| `incidents.list` | GET `/v1/incidents/` (`ListIncidents`; the document's path ends with `/`, the reference writes `/v1/incidents`) | `query` = `createdAt >= <epoch ms>` [docs: incidents search lists only `createdAt` as a time field]; `sort` held to `createdAt`, `order` to `asc`; `query` required by the document, optional in the reference | `offset` min 0, `limit` 1..100, default 20 [OAS, docs]; `paging.next`; the 20K cap is documented for alerts only [docs]; applying the same bound to incidents is [inferred] |
| `incident.get` | GET `/v1/incidents/{identifier}` (`getIncident`) | n/a | single; `identifierType` id/tiny [OAS] |
| `incident.alert_ids` | GET `/v1/incidents/{identifier}/associated-alert-ids` (not in the document: added by a cited amendment, see Open question A) | n/a: the link is read per incident found by `incidents.list` | `offset` min 1, `direction` (default `next`), `limit` default 20, max 25, `order` asc/desc [docs]; response `data` is an array of alert ids [docs sample]; paging fields of this answer not shown in the docs [unknown] |

Since-window caveats, to be stated in `docs/catalog-opsgenie.md`:

- Only creation time is a documented filter. A status change after the run that copied an alert
  is caught by `alert.get` / `incident.get`, or by re-reading a window; after the shutdown date
  nothing changes, so a final full pass over created windows is complete [inferred].
- `createdAt >= T` with `T` the last stored instant re-reads alerts at `T`; the caller
  deduplicates by `id` (at-least-once, as the feed family states for watermarks).
- Incident API is "only available to Standard and Enterprise plans" [docs]; a 402/403 on
  `incidents.list` answers through the generic read error table.

## Spec first

1. `adapters/catalog/spec/ess/domains/identity.yaml`: the token placement gains a scheme word.
   Proposed: `Scheme` stays; `AuthConfig` gains optional `prefix` (one visible-ASCII token
   word, 1..32 bytes, e.g. `GenieKey`), mutually exclusive with `bearer: true`, omitted when
   absent so existing configuration revisions hold. Model where `AuthConfig` is typed (today
   `Profile` there carries `scheme` and `identity` only; the placement fields are owned by
   `docs/local-catalog-provider.md` "Configure the provider"). If ESS cannot state "exactly one
   of `bearer: true` and `prefix`" as an invariant, stop and report; do not hand-write it.
2. `adapters/catalog/spec/ess/domains/amendment.yaml`: a fourth amendment kind
   `add_operation` (GET only, path under the base path, query parameters only, a cited https
   reference and a reason), refused for any other method. Or Open question A's other option.
3. `contracts/auth/capability/v1alpha1/semantics.md` and the host's `HttpConfig`
   (`crates/connectors-host/src/http.rs:15-30`): `http-bearer` capability with a declared
   scheme word other than `Bearer`. `HttpConfig` derives a JSON schema, so the generated
   reference moves (`cargo run --locked -p connectors-docs -- generate`).
4. Validate with the pinned `ess` (`crates/connectors-spec/toolchain.json`: 0.59.0; move it to
   the newest release first if one has shipped), regenerate, then implement.

## Acceptance (recorded fixtures; no live provider)

In `adapters/catalog/tests/opsgenie.rs` unless named otherwise:

- `shipped_opsgenie_selections_are_exactly_the_five_reads`: ids `alerts.list`, `alert.get`,
  `incidents.list`, `incident.get`, `incident.alert_ids`, each `effect: read`.
- `the_pinned_source_is_the_one_the_bundle_and_its_record_name`: README, source-hash record,
  bundle index and `LICENSE` name one digest and one commit.
- `alerts_list_holds_sort_and_order_and_bounds_offset_and_limit`: `sort` other than `createdAt`,
  `order` other than `asc`, `limit` 0 or 101, `offset` 19900 are each refused before any request.
- `alerts_list_sends_the_since_query_with_geniekey_and_returns_the_recorded_body`: the stand-in
  provider sees `Authorization: GenieKey <fixture key>` and
  `query=createdAt%20%3E%3D%201760000000000&sort=createdAt&order=asc&offset=0&limit=100`; the
  body comes back byte-identical.
- `a_window_walks_two_pages_and_resumes_from_the_last_created_at`: page 1 full, page 2 short;
  the next window's `query` carries the last `createdAt` in epoch ms.
- `incidents_list_sends_the_documented_path`: the exact request path the stand-in sees
  (pins the trailing-slash question either way).
- `incident_alert_ids_is_added_by_a_cited_amendment`: the amendment is bound to the pinned
  digest, cites the incident API page, and the request is
  `GET /v1/incidents/<id>/associated-alert-ids?identifierType=id&limit=25`.
- `a_prefix_profile_sends_the_word_and_refuses_bearer_with_prefix` (catalog local tests and
  `crates/connectors-host` http tests): prefix + token on the wire; `bearer: true` with `prefix`
  refused at load; a configuration without `prefix` keeps its revision byte for byte.
- `connecting_records_the_configured_instance_as_the_identity`: identity probe
  `GET v2/alerts` (a probe path carries no query) with `source: configuration` (Opsgenie's key belongs to an
  integration, not a user [docs: "Create an API Integration and obtain your apiKey"]).
- `bundle_drift.rs` reproduces `opsgenie.bundle.json` byte for byte.
- `docs/catalog-opsgenie.md` per operation: operationId and path, since parameter, paging,
  end condition, the 20K cap and the shutdown note; a guide test pins those citations.

## Scope (files)

- New: `adapters/opsgenie/upstream/{swagger.json,LICENSE,README.md,opsgenie-source-hashes.json,opsgenie.amendments.json}`;
  `adapters/catalog/providers/opsgenie/operations.json`;
  `adapters/catalog/generated/bundles/opsgenie.bundle.json`; `adapters/catalog/tests/opsgenie.rs`;
  `docs/catalog-opsgenie.md`.
- Changed: `adapters/catalog/generated/bundles/index.json`, `adapters/catalog/tests/bundle_drift.rs`,
  `crates/connectors-build/src/main.rs` (bundle recipe), `crates/connectors-build/src/source_hashes.rs`
  (manifest list) [inferred from `0df7500c4`];
  credential prefix: `adapters/catalog/src/local.rs`, `adapters/catalog/spec/ess/domains/identity.yaml`,
  `crates/connectors-host/src/http.rs`, `contracts/auth/capability/v1alpha1/semantics.md`,
  `docs/local-catalog-provider.md`;
  operation amendment: `adapters/catalog/spec/ess/domains/amendment.yaml`,
  `crates/connectors-catalog/src/amendment.rs`, `crates/connectors-catalog/tests/` amendment cases;
  regenerated references (`crates/connectors-docs` outputs, `website/data/status.json`),
  `docs/fluxplane-plugin-parity.md` rows 895-896, `CHANGELOG.md`, `README.md` provider list.

## Not in scope

On-call, schedules, `opsgenie.test`, ack/close/note and every other write (stay in the revised
`story:parity-opsgenie-alerts`); alert logs, notes, attachments; incident timeline, logs and
notes; `responder-alert-ids`; saved searches (`searchIdentifier` withheld); a
`datasource.feed/v1alpha1` binding; the `/v2/alerts/count` endpoint; Jira Service Management.

## Open questions

**A. How the link operation enters the bundle.**
(a) A new amendment kind `add_operation`, GET only, cited, bound to the pinned digest;
(b) wire the existing authored TOML reader into `bundle --source` as a second source;
(c) ship without the link and let a caller match alerts to incidents some other way.
Recommendation: (a). It reuses the cited, digest-bound path Zendesk and GitLab already use; (b)
is a larger, unscheduled pipeline change; (c) drops a requested outcome.

**B. The credential prefix.**
(a) Optional `prefix` word on the `token` scheme, exclusive with `bearer`; (b) a new scheme
`api_key` with a required `prefix`; (c) a per-provider host flag `genie_key`.
Recommendation: (a). One field, the revision of every existing configuration holds, and it
covers other `<word> <token>` schemes later; (c) is provider vocabulary in shared code.

**C. Since cursor: caller-held or a feed binding.**
(a) Pass-through reads, caller keeps the last `createdAt` (Zendesk precedent); (b) extend the
feed engine with a query-language time template and offset paging
(`story:catalog-feed-engine-extensions` items 1-2, which need a security review) and bind
Opsgenie to `datasource.feed/v1alpha1`. Recommendation: (a). The provider is gone in under six
months, the feed shape wants a container per item list (Opsgenie has none that is natural), and
(b) is a reviewed engine change for one short-lived source.

## Build estimate

Package gates: `connectors-catalog-provider` (adapters/catalog), `connectors-catalog`
(amendment kind, Swagger projection of a new document), `connectors-host` (prefix placement),
`connectors-build` (bundle recipe, source hashes, gate), `connectors-docs` (regenerated
reference); then the repository gate. Similar waves: Zendesk reads `0df7500c4` (19 files,
catalog-provider + catalog + build), the Zendesk amendment `e7576164c` (14 files, adds
`amendment.rs`), Slack reads `b2854830d` (Swagger 2.0 source, 14 files). This unit is those
three together plus a host change: one story, roughly a Zendesk-sized wave with one extra
crate. Risk not yet checked (no build run): whether the Swagger projection ingests all 170
operations of this 2021 document without a refusal that blocks the bundle.

## Decisions taken

Taken 2026-10-13 as design choices inside the requested read-only scope: A (a), an `add_operation` amendment, GET only, cited and bound to the pinned digest; B (a), an optional `prefix` word on the `token` scheme, exclusive with `bearer`; C (a), pass-through reads with a caller-held `createdAt` cursor.
