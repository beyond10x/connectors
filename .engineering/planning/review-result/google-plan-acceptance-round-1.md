---
format: aep.planning-md/3
id: review-result:google-plan-acceptance-round-1
kind: review-result
status: active
title: Plan critic (acceptance), round 1, epic:google-workspace-reads
relations:
- reviews: story:catalog-oauth2-refresh-profile
- reviews: story:cli-oauth-loopback-acquisition
- reviews: story:catalog-discovery-projection
- reviews: story:catalog-repeated-query-parameters
- reviews: story:catalog-google-drive-reads
- reviews: story:catalog-google-slides-reads
- reviews: story:catalog-google-calendar-reads
- reviews: story:catalog-google-gmail-reads
- reviews: story:catalog-google-slides-writes
- reviews: story:catalog-google-drive-writes
- reviews: story:catalog-google-calendar-writes
- reviews: story:catalog-google-gmail-draft-writes
revision: 1
---
needs-revision

cli-oauth-loopback-acquisition — the acceptance has no check for the contract variant, the `registry.rs` and `docs/local-connection-registry.md` amendments, or the `docs/catalog-google.md` guide. The Design promises them and the ADR's point 4 requires them, so they can be skipped and the story still closes — .engineering/planning/story/cli-oauth-loopback-acquisition.md:64
cli-oauth-loopback-acquisition — the Outcome says the flow serves `connections connect` "(and `repair`)", but every acceptance line exercises `connect` only, so repair can fail while the story closes — .engineering/planning/story/cli-oauth-loopback-acquisition.md:33
catalog-discovery-projection — the Outcome says the bundle "records the derivation" and `--derived-from` refuses a non-matching `--source`. The acceptance checks only that `derivation` is absent from existing bundles (negative), so it reads the same whether or not the new field is ever written or the mismatch refused — .engineering/planning/story/catalog-discovery-projection.md:90
catalog-discovery-projection — `discovery_ingests` allows zero `Unsupported` entries "except the ones the record lists", but the `ProjectionRecord` lists excluded methods, rewritten paths and excluded parameters, not unsupported ingest entries. The exception is uncheckable as written — .engineering/planning/story/catalog-discovery-projection.md:88
catalog-oauth2-refresh-profile — the Design's `iss` check, https-only `token_url` and cache eviction on `InvalidCredential` have no test. Only a wrong `aud` is named — .engineering/planning/story/catalog-oauth2-refresh-profile.md:79
catalog-oauth2-refresh-profile — the acceptance adds a token route to the TLS fixture, but the Design's token client has `ca = None`, so it cannot trust the fixture's CA. The body does not say how the token route is reached — .engineering/planning/story/catalog-oauth2-refresh-profile.md:40
catalog-repeated-query-parameters — the Outcome promises a JSON array in `operations invoke` sent as one pair per element, and a declared input schema of `type: array`. The acceptance has template-level and refusal tests only, no positive engine test and no schema assertion — .engineering/planning/story/catalog-repeated-query-parameters.md:44
catalog-google-drive-reads — the Outcome says "This story also pins all four Google Discovery sources", but the acceptance checks only Drive's regeneration. Nothing observes the Slides, Calendar and Gmail pins, hashes or LICENSE files — .engineering/planning/story/catalog-google-drive-reads.md:31
catalog-google-drive-reads — `files.list` `pageSize` "bounded 1–1000" appears only in the operations table. No acceptance line asserts the bound or its `invalid_input` refusal — .engineering/planning/story/catalog-google-drive-reads.md:57
catalog-google-calendar-reads — `events.list` `maxResults` "bounded 1–2500", taken from description text, has no acceptance line asserting the bound — .engineering/planning/story/catalog-google-calendar-reads.md:39
catalog-google-gmail-reads — `users.messages.list` `maxResults` "bounded 1–500", taken from description text, has no acceptance line asserting the bound — .engineering/planning/story/catalog-google-gmail-reads.md:39
catalog-google-slides-writes — "The connection's scopes include `presentations`" names no place or command where the scope is observed, so it cannot be checked twice the same way — .engineering/planning/story/catalog-google-slides-writes.md:41
catalog-google-drive-writes — the `files.update` guard "checks `version` or `modifiedTime`" without choosing one, so the "stale preflight sends no write" test has no defined expected field — .engineering/planning/story/catalog-google-drive-writes.md:30
catalog-google-calendar-writes — "`sendUpdates` defaults to `none`… the approval binds its value" is stated behaviour with no acceptance line (no default test, no approval-binding test) — .engineering/planning/story/catalog-google-calendar-writes.md:32

**What I read:** all 14 given ids in full (12 stories, the ADR and the blocker), plus `epic:google-workspace-reads`, using `aep plan artifact show` and `aep plan artifact kinds` / `lifecycle story`. I also checked `docs/local-catalog-provider.md:141-164`, `adapters/catalog/src` (guard) and `crates/connectors-host/src/http.rs:95-140`.

**Could not establish:**
- Whether the fixture token host can be trusted under the Design's `ca = None`. I read `http.rs`, which applies `tls_certs_only` only when a CA is passed, but I did not run a test.
- The ADR has no acceptance section, so I judged it only as context.
- Out of my lane, and they set no verdict:
  - The blocker lists only three stories, but the write story `story:catalog-google-slides-writes` needs the same OAuth client for its live evidence, and that evidence needs an operator-named scratch deck. Coupling and coverage lanes.
  - `story:catalog-discovery-projection` has no dependency on `story:catalog-repeated-query-parameters`, although the projected output contains `repeated` parameters that `discovery_ingests` must ingest. Design lane.

```findings
- file: .engineering/planning/story/cli-oauth-loopback-acquisition.md
  line: 64
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: the acceptance has no check for the contract variant, the registry.rs and docs/local-connection-registry.md amendments, or the docs/catalog-google.md guide that the Design and the ADR's point 4 require, so they can be skipped and the story still closes
- file: .engineering/planning/story/cli-oauth-loopback-acquisition.md
  line: 33
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: the Outcome says the flow serves connections connect "(and repair)" but every acceptance line exercises connect only, so repair can fail while the story closes
- file: .engineering/planning/story/catalog-discovery-projection.md
  line: 90
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: the Outcome says the bundle records the derivation and --derived-from refuses a non-matching --source, but the acceptance checks only that derivation is absent from existing bundles, so it reads the same whether or not the field is ever written or the mismatch refused
- file: .engineering/planning/story/catalog-discovery-projection.md
  line: 88
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: discovery_ingests allows zero Unsupported entries "except the ones the record lists" but the ProjectionRecord lists excluded methods, rewritten paths and excluded parameters, not unsupported ingest entries, so the exception is uncheckable as written
- file: .engineering/planning/story/catalog-oauth2-refresh-profile.md
  line: 79
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: the Design's iss check, https-only token_url and cache eviction on InvalidCredential have no named test; only a wrong aud is asserted
- file: .engineering/planning/story/catalog-oauth2-refresh-profile.md
  line: 40
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: the acceptance adds a token route to the TLS fixture but the Design's token client has ca = None and cannot trust the fixture CA, and the body does not say how the token route is reached
- file: .engineering/planning/story/catalog-repeated-query-parameters.md
  line: 44
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: the Outcome promises a JSON array in operations invoke sent as one pair per element and a declared input schema of type array, but the acceptance has only template-level and refusal tests, no positive engine test and no schema assertion
- file: .engineering/planning/story/catalog-google-drive-reads.md
  line: 31
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: the Outcome says the story pins all four Google Discovery sources but the acceptance checks only Drive's regeneration, so nothing observes the Slides, Calendar and Gmail pins, hashes or LICENSE files
- file: .engineering/planning/story/catalog-google-drive-reads.md
  line: 57
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: files.list pageSize "bounded 1-1000" appears only in the operations table and no acceptance line asserts the bound or its invalid_input refusal
- file: .engineering/planning/story/catalog-google-calendar-reads.md
  line: 39
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: events.list maxResults "bounded 1-2500", taken from description text, has no acceptance line asserting the bound
- file: .engineering/planning/story/catalog-google-gmail-reads.md
  line: 39
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: users.messages.list maxResults "bounded 1-500", taken from description text, has no acceptance line asserting the bound
- file: .engineering/planning/story/catalog-google-slides-writes.md
  line: 41
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: '"The connection''s scopes include presentations" names no place or command where the scope is observed, so it cannot be checked twice the same way'
- file: .engineering/planning/story/catalog-google-drive-writes.md
  line: 30
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: the files.update guard "checks version or modifiedTime" without choosing one, so the stale-preflight test has no defined expected field
- file: .engineering/planning/story/catalog-google-calendar-writes.md
  line: 32
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: '"sendUpdates defaults to none... the approval binds its value" is stated behaviour with no acceptance line (no default test, no approval-binding test)'
```
