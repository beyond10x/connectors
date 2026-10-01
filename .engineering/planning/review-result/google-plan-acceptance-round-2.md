---
format: aep.planning-md/3
id: review-result:google-plan-acceptance-round-2
kind: review-result
status: active
title: Plan critic (acceptance), round 2, epic:google-workspace-reads
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
- reviews: epic:google-workspace-reads
- reviews: credential-blocker:google-oauth-client
- reviews: architecture-decision-record:oauth-material-as-static-entry
revision: 1
---
needs-revision

Nine of the 14 round-1 findings are fixed. The other five (the derivation, `Unsupported`, oauth2 and repeated-parameter findings) are fixed in the drafts but have no outcome recorded (the drive-reads pin finding was moved). The findings below are new defects in the revised set.

cli-oauth-loopback-acquisition — the Design's refusal `ProtectedEntryUnavailable` when the profile's field names differ from the triple, and its cancellation by `protected::cancellation()`, have no named test. The unit-test list stops at `client_file_uri_mismatch_refused` — .engineering/planning/story/cli-oauth-loopback-acquisition.md:66
cli-oauth-loopback-acquisition — the live-evidence line ("one `connections connect` against Google with browser consent, recorded") names no observed result, so a record of a failed connect satisfies it — .engineering/planning/story/cli-oauth-loopback-acquisition.md:82
catalog-google-slides-reads — the three live-evidence lines (connect, `files.export`, `presentations.get`) name an action but no pass condition, such as the connection ready, non-empty `text/plain`, or a `presentationId` equal to the deck's — .engineering/planning/story/catalog-google-slides-reads.md:44
catalog-google-slides-writes — the live `batchUpdate` line names no result to observe and no named scratch deck, so a recorded failure passes — .engineering/planning/story/catalog-google-slides-writes.md:37
catalog-google-slides-writes — the guard table gives `presentations.create` a postflight ("the response `presentationId` is present"), but no acceptance line exercises it. `Check` compares only against an input or a literal (`adapters/catalog/src/lib.rs:41-44`), so "present" is not an expressible check — .engineering/planning/story/catalog-google-slides-writes.md:23
catalog-google-drive-writes — the `files.create` and `files.copy` postflights ("`id` present") have no acceptance line, and "present" is not an expressible `Check` — .engineering/planning/story/catalog-google-drive-writes.md:23
catalog-google-calendar-writes — the `events.insert` postflight ("`id` present") has no acceptance line, and "present" is not an expressible `Check` — .engineering/planning/story/catalog-google-calendar-writes.md:22
catalog-google-gmail-draft-writes — the `users.drafts.create` postflight ("`id` present") has no acceptance line, and "present" is not an expressible `Check` — .engineering/planning/story/catalog-google-gmail-draft-writes.md:23
catalog-google-drive-reads — `changes.list` `pageSize` "bounded 1–1000" is stated in the table, but `files_list_page_size_bounds` covers only `files.list` — .engineering/planning/story/catalog-google-drive-reads.md:48
catalog-google-gmail-reads — `users.threads.list` is "as messages.list", which includes the `maxResults` 1–500 bound, but `messages_list_max_results_bounds` covers only `users.messages.list` — .engineering/planning/story/catalog-google-gmail-reads.md:44
catalog-discovery-projection — "An `aep:adversary` pass on the projector before merge" names no record where the pass is observable, so it cannot be checked twice the same way — .engineering/planning/story/catalog-discovery-projection.md:101

**What I read:** all 14 ids in full (12 stories, the ADR, the blocker), plus `review-result:google-plan-acceptance-round-1`, using `aep plan artifact show`. I also read `adapters/catalog/src/lib.rs:30-100,570-600` and `docs/local-catalog-provider.md:141-164`.

**Could not establish:**
- Whether the engine can express an "id present" postflight. `Check` looks equality-only, and no write story's scope includes `adapters/catalog/src`. I judged only the missing acceptance line.
- Out of my lane, and they set no verdict:
  - The ADR's Decision 1 says acquisition runs "before the owner's capture window opens". `story:cli-oauth-loopback-acquisition` runs it inside the 300 s window (line 39). This is a design contradiction.
  - `story:catalog-discovery-projection` has no `depends_on story:catalog-repeated-query-parameters`, though `discovery_ingests` needs the projected `repeated` parameters ingested.
  - The `story:catalog-google-drive-reads` title still says "and the four pinned Google Discovery sources", but the pinning now lives in `story:catalog-discovery-projection`.
  - The blocker does not list `story:catalog-google-drive-reads`, although its body says it withholds that story's evidence.

```findings
- file: .engineering/planning/story/cli-oauth-loopback-acquisition.md
  line: 66
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Design's ProtectedEntryUnavailable refusal when the profile's field names differ from the triple, and its cancellation by protected::cancellation(), have no named test; the unit-test list stops at client_file_uri_mismatch_refused"
- file: .engineering/planning/story/cli-oauth-loopback-acquisition.md
  line: 82
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the live-evidence line names no observed result, so a recorded failed connect satisfies it"
- file: .engineering/planning/story/catalog-google-slides-reads.md
  line: 44
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the three live-evidence lines name an action but no pass condition, such as connection ready, non-empty text/plain, or a presentationId equal to the deck's"
- file: .engineering/planning/story/catalog-google-slides-writes.md
  line: 37
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the live batchUpdate line names no result to observe and no named scratch deck, so a recorded failure passes"
- file: .engineering/planning/story/catalog-google-slides-writes.md
  line: 23
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the guard table gives presentations.create a postflight (the response presentationId is present) but no acceptance line exercises it, and Check compares only against an input or a literal so 'present' is not an expressible check"
- file: .engineering/planning/story/catalog-google-drive-writes.md
  line: 23
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the files.create and files.copy postflights (id present) have no acceptance line, and 'present' is not an expressible Check"
- file: .engineering/planning/story/catalog-google-calendar-writes.md
  line: 22
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the events.insert postflight (id present) has no acceptance line, and 'present' is not an expressible Check"
- file: .engineering/planning/story/catalog-google-gmail-draft-writes.md
  line: 23
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the users.drafts.create postflight (id present) has no acceptance line, and 'present' is not an expressible Check"
- file: .engineering/planning/story/catalog-google-drive-reads.md
  line: 48
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "changes.list pageSize 'bounded 1-1000' is stated in the table but files_list_page_size_bounds covers only files.list"
- file: .engineering/planning/story/catalog-google-gmail-reads.md
  line: 44
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "users.threads.list is 'as messages.list', which includes the maxResults 1-500 bound, but messages_list_max_results_bounds covers only users.messages.list"
- file: .engineering/planning/story/catalog-discovery-projection.md
  line: 101
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'An aep:adversary pass on the projector before merge' names no record where the pass is observable, so it cannot be checked twice the same way"
```
