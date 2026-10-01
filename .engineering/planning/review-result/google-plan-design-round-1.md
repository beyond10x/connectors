---
format: aep.planning-md/3
id: review-result:google-plan-design-round-1
kind: review-result
status: active
title: Plan critic (design), round 1, epic:google-workspace-reads
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
revision: 1
---
needs-revision

story:catalog-google-drive-reads — its live-evidence acceptance (`files.export` of the deck) and the epic's live run go through `connections connect` browser consent, which `story:cli-oauth-loopback-acquisition` builds, yet only `story:catalog-google-slides-writes` carries `depends_on` to it, so add `depends_on story:cli-oauth-loopback-acquisition` here (slides-reads inherits it) — .engineering/planning/story/catalog-google-drive-reads.md:74 and `aep plan artifact graph`

story:catalog-discovery-projection — its acceptance runs over copies of the four pinned documents "from the same commit" that `story:catalog-google-drive-reads` pins (a Drive story owning Slides, Calendar and Gmail pins), so the same bytes are vendored twice and the pin's owner has no edge to its consumer (an edge from projection to drive-reads would cycle); move the pin of all four sources into this story and have drive-reads consume it — .engineering/planning/story/catalog-discovery-projection.md:79 and .engineering/planning/story/catalog-google-drive-reads.md:31

story:cli-oauth-loopback-acquisition — it requires the client file's `auth_uri` and `token_uri` to equal "the instance configuration's declared URLs" and builds the authorize URL from the configuration, but the only channel from adapter to host is `Profile` (no URL fields; `adapters/catalog/src/local.rs:335-345`), and `story:catalog-oauth2-refresh-profile` adds only a private `token_url` and no authorize URL, so the body must say where the host gets both URLs and which story adds the field — .engineering/planning/story/cli-oauth-loopback-acquisition.md:57 and .engineering/planning/story/catalog-oauth2-refresh-profile.md:35

epic:google-workspace-reads — the only stated reason for the eight-story chain is that each story rewrites `bundles/index.json` and `bundle_drift.rs`, but the four write stories list neither, so the four write edges have no reason beside them, and the `slides-writes` to `gmail-reads` edge (in the graph, absent from this table) holds all writes until every read has landed; the trade-off is to keep the order and name `docs/catalog-google.md` as the shared file, or to split it per provider and drop the write-chain edges — .engineering/planning/epic/google-workspace-reads.md:73 and `aep plan artifact graph`

credential-blocker:google-oauth-client — `story:catalog-google-slides-writes` has a live-evidence acceptance that needs the same Google desktop client, yet the blocker's `blocks` edges omit it, so add `blocks story:catalog-google-slides-writes` — .engineering/planning/story/catalog-google-slides-writes.md:43 and .engineering/planning/credential-blocker/google-oauth-client.md:8-10

**What I read.** 14 artifacts (12 stories, the ADR, the blocker) plus the epic, via `aep plan artifact show` on each, `relations`, `graph` and `validate`. I read `bundle_drift.rs`, `inventory.rs`, `adapters/catalog/src/local.rs` and `docs/local-catalog-provider.md:141-164`.

**What I could not establish.**
- **Cycles:** I walked about 31 edges, including the epic's `informed_by` edges to artifacts outside the set, and found none. The only `depends_on` edges are the eight-story provider chain and the acquisition to refresh-profile edge.
- **Chain trade-off:** I did not raise the four-story read chain (drive, slides, calendar, gmail) as a finding because the epic states its reason. It is a trade-off between serialising and splitting the generated `index.json` / `bundle_drift.rs` surface.
- **`validate`:** it printed `valid` plus review-result outcome reminders, none of them about this set.
- **Out of my lane:**
  - Both `story:cli-oauth-loopback-acquisition` and `story:catalog-google-drive-reads` author `docs/catalog-google.md` with no edge between them (parallel-safety).
  - The write stories' scopes omit `bundles/index.json` (parallel-safety).
  - The ADR is `proposed` while the epic table lists it as a dependency and the stories only have `informed_by` on it (scope or lifecycle).

```findings
- file: .engineering/planning/story/catalog-google-drive-reads.md
  line: 74
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: its live-evidence acceptance (files.export of the deck) and the epic's live run go through connections connect browser consent, which story:cli-oauth-loopback-acquisition builds, yet only story:catalog-google-slides-writes carries depends_on to it, so add depends_on story:cli-oauth-loopback-acquisition here (slides-reads inherits it)
- file: .engineering/planning/story/catalog-discovery-projection.md
  line: 79
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: its acceptance runs over copies of the four pinned documents "from the same commit" that story:catalog-google-drive-reads pins (a Drive story owning Slides, Calendar and Gmail pins), so the same bytes are vendored twice and the pin's owner has no edge to its consumer (an edge from projection to drive-reads would cycle); move the pin of all four sources into this story and have drive-reads consume it
- file: .engineering/planning/story/cli-oauth-loopback-acquisition.md
  line: 57
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: it requires the client file's auth_uri and token_uri to equal the instance configuration's declared URLs and builds the authorize URL from the configuration, but the only channel from adapter to host is Profile (no URL fields; adapters/catalog/src/local.rs:335-345), and story:catalog-oauth2-refresh-profile adds only a private token_url and no authorize URL, so the body must say where the host gets both URLs and which story adds the field
- file: .engineering/planning/epic/google-workspace-reads.md
  line: 73
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: the only stated reason for the eight-story chain is that each story rewrites bundles/index.json and bundle_drift.rs, but the four write stories list neither, so the four write edges have no reason beside them, and the slides-writes to gmail-reads edge (in the graph, absent from this table) holds all writes until every read has landed; the trade-off is to keep the order and name docs/catalog-google.md as the shared file, or to split it per provider and drop the write-chain edges
- file: .engineering/planning/credential-blocker/google-oauth-client.md
  line: 8
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: story:catalog-google-slides-writes has a live-evidence acceptance that needs the same Google desktop client, yet the blocker's blocks edges omit it, so add blocks story:catalog-google-slides-writes
```
