---
format: aep.planning-md/3
id: epic:google-workspace-reads
kind: epic
status: active
title: Google Slides, Drive, Calendar and Gmail through the catalog provider, with OAuth
summary: Discovery ingest, OAuth with refresh, reads for four Google APIs, then guarded writes.
relations:
- informed_by: architecture-decision-record:declarative-http-provider-runtime
- informed_by: specification:catalog-http-runtime-handoff
- serves: vision:independent-contract-adapters
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T10:10:52Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"artifact":1,"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-01T10:10:52Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"artifact":1,"review_outcome":1}}}
---
## Outcome

The local `connectors` CLI reads the operator's Google Slides, Drive, Calendar and Gmail through
the catalog provider, authenticated by OAuth with refresh, over bundles compiled from Google's
Discovery documents. Guarded writes to the same four follow. The first concrete target is the deck
`1HurXAVXiOQszM7a_tpgYqRj5Uhy3gT_W6Iz-X4ZVYWY`, read as text (`drive.files.export`, `text/plain`)
and as structure (`slides.presentations.get`).

Operator decisions, 2026-09-30: implement OAuth; ingest Discovery through an exact projection to
OpenAPI ("make no mistakes"); drop the hand-written TOML prototype. The account-type question was
not answered, so the default holds: a personal account, an external OAuth app in Testing, whose
refresh tokens expire after 7 days (developers.google.com/identity/protocols/oauth2), so a weekly
`connections repair`.

## Findings (read 2026-09-30)

| Fact | Source |
|---|---|
| Ingest reads OpenAPI 3.0/3.1 only; a Discovery document is refused `VersionAbsent` | `crates/connectors-catalog/src/lib.rs:92-101,133-136` |
| `SourceRecord` has no transform or derivation field | `crates/connectors-catalog/src/lib.rs:77-90` |
| `{+name}` placeholders are refused `PlaceholderUndeclared`; path values are fully escaped | `crates/connectors-catalog/src/template.rs:298-332,437-442,626` |
| Query parameters are scalar only | `adapters/catalog/src/lib.rs:128-136,381-392`; `template.rs:571` |
| Text responses need `"response":"text"`; no binary path; 4 MiB body cap | `adapters/catalog/src/lib.rs:67-75,610-656`; `crates/connectors-core/src/lib.rs:9` |
| One catalog configuration is one base URL and one bundle, so one connection per Google API | `crates/connectors-host/src/http.rs:51-57` |
| No OAuth code on `main`; the registry admits `static_entry` only | `crates/connectors-host/src/local/registry.rs:60-61` |
| The adapter child lives across invokes and receives the secret per request; it cannot write material back | `crates/connectors-host/src/local/owner/supervisor.rs:617-640`; `crates/connectors-host/src/local/runtime/server.rs:12-36` |
| Google Discovery method counts: Slides 5, Drive 64, Calendar 38, Gmail 79; constructs in use: one `{+presentationId}`, 11 media methods, 11 `repeated` parameters, `any`, `int64`, `uint64`, `byte`, `google-datetime`, `google-fieldmask` | live Discovery documents fetched 2026-09-30 |
| Endpoints: authorize `https://accounts.google.com/o/oauth2/v2/auth` in Google's guide, but a downloaded Desktop client file carries `auth_uri` `https://accounts.google.com/o/oauth2/auth`, and the connectors configuration uses the latter because the consent flow compares them byte for byte (review-result:adversary-google-slides-writes-pass-2, F1); token `https://oauth2.googleapis.com/token`; loopback `http://127.0.0.1:port`; PKCE S256; installed apps "cannot keep secrets" | developers.google.com/identity/protocols/oauth2/native-app |
| BSD-3-Clause copies of the Discovery documents: `googleapis/google-api-go-client` at `ec13a0cd76ecc7fede8932d040a3156804515da9` | `git ls-remote`, HTTP 200 on each file, 2026-09-30 |

## Design

- **OAuth** (`architecture-decision-record:oauth-material-as-static-entry`): the CLI acquires
  `{client_id, client_secret, refresh_token}` by loopback + PKCE during `connections connect` and
  stores it through the existing static-entry capture; the catalog child exchanges it for an access
  token per call, caches the token in memory, and never writes material back.
- **Discovery ingest**: a pure, deterministic projection Discovery → OpenAPI 3.0 in
  `crates/connectors-catalog`, recorded on the bundle's `SourceRecord`, then the existing pipeline.
- **Repeated query parameters** in the inventory, template and engine.
- **Per-API bundles and selections** under `adapters/google/`, one story per API, then guarded
  writes per API.

## Stories

| Story | Depends on | Reason |
|---|---|---|
| `story:catalog-oauth2-refresh-profile` | — | ADR decides the design |
| `story:cli-oauth-loopback-acquisition` | refresh profile | reads the profile's `acquisition` metadata |
| `story:catalog-discovery-projection` | — | pins all four sources |
| `story:catalog-repeated-query-parameters` | — | |
| `story:catalog-google-drive-reads` | projection, refresh profile | shared `index.json` and `bundle_drift.rs` start here |
| `story:catalog-google-slides-reads` | Drive reads | `index.json` |
| `story:catalog-google-calendar-reads` | repeated parameters, Slides reads | `index.json` |
| `story:catalog-google-gmail-reads` | repeated parameters, Calendar reads | `index.json` |
| `story:catalog-google-slides-writes` | Slides reads, acquisition | `providers/google-slides/operations.json`, `tests/google_slides.rs` |
| `story:catalog-google-drive-writes` | Drive reads, acquisition | `providers/google-drive/operations.json`, `tests/google_drive.rs` |
| `story:catalog-google-calendar-writes` | Calendar reads, acquisition | `providers/google-calendar/operations.json`, `tests/google_calendar.rs` |
| `story:catalog-google-gmail-draft-writes` | Gmail reads, acquisition | `providers/google-gmail/operations.json`, `tests/google_gmail.rs` |
| `story:catalog-google-live-deck-read` | Slides reads, acquisition | blocked by `credential-blocker:google-oauth-client`; outside the chain |

Read stories run in sequence because each rewrites
`adapters/catalog/generated/bundles/index.json` and `adapters/catalog/tests/bundle_drift.rs`.
Write stories touch only their own provider's selection and test file, so they may run in
parallel with each other. Guides are one file per provider (`docs/catalog-google-<api>.md`) plus
`docs/catalog-google-oauth.md`, so no two stories share a guide. Create writes carry no guard
(the guard model has nothing to compare before a create); no story edits the guard engine.

## Acceptance

- The four read selection sets load against bundles compiled from pinned Discovery documents, and
  `adapters/catalog/tests/bundle_drift.rs` reproduces the projected documents and the bundles byte
  for byte.
- A live run on the operator's account reads the target deck through `connections connect` with
  browser consent and `operations invoke`, recorded as evidence against
  `story:catalog-google-slides-reads`.
- Each write story's operations are `effect: write` and refused without an approval.

## Out of scope

Service accounts and domain-wide delegation; push notifications (Drive/Calendar watch, Gmail
Pub/Sub); media upload; binary downloads (PDF, images); one OAuth grant shared across the four
connections (each connection consents on its own); Sheets and Docs APIs.

## Status

2026-10-01: all 13 delivery stories are implemented and released in v0.21.0 (tag v0.21.0, merge c90adeaf4; gate --msrv 136 suites, 1096 passed, 0 failed). The epic stays active for one item: `story:catalog-google-live-deck-read`, the human live test, blocked on `credential-blocker:google-oauth-client`. It moves to implemented when that story does.
