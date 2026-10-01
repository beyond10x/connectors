---
format: aep.planning-md/3
id: review-result:google-plan-design-round-2
kind: review-result
status: active
title: Plan critic (design), round 2, epic:google-workspace-reads
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

The round-1 findings all landed. I found one unrecorded dependency and two lesser problems.

- **Live evidence on drive-reads:** the `cli-oauth-loopback-acquisition` edge is still not on drive-reads, but the live evidence now sits under slides-reads, which depends on acquisition.
- **Projection pin:** the four Discovery pins are owned by `catalog-discovery-projection`, and drive-reads consumes them.
- **Profile URLs:** `acquisition {authorize_url, token_url, scopes}` on `Profile` closes the URL gap. `Capture` carries `runtime::Profile` directly (`crates/connectors-host/src/local/owner/transport.rs:64-69`).
- **Chain reasons:** the epic states the reasons for the read chain and for writes running in parallel.
- **Blocker edge:** `blocks story:catalog-google-slides-writes` is present.

**Findings**

story:catalog-google-drive-reads — its acceptance requires `docs/catalog-google-drive.md` to link `docs/catalog-google-oauth.md`, a file only `story:cli-oauth-loopback-acquisition` creates, and drive-reads has no `depends_on` to it (slides-reads does), so add that edge or move the link to slides-reads — .engineering/planning/story/catalog-google-drive-reads.md:75 and .engineering/planning/story/cli-oauth-loopback-acquisition.md:73

story:catalog-google-slides-reads — it is the only story with a blocked live-evidence acceptance sitting inside the shared-file read chain, so `calendar-reads`, `gmail-reads` and (through acquisition, which is blocked the same way) all four write stories wait on an operator step in the Google Cloud console that is unrelated to the shared `index.json` and `bundle_drift.rs` ordering the edges exist for. Move the live evidence to an epic-level acceptance or its own verification item, or put the blocked story last in the chain — .engineering/planning/story/catalog-google-slides-reads.md:56 and `aep plan artifact graph`

credential-blocker:google-oauth-client — the "What it withholds" section still names `story:catalog-google-drive-reads` (whose live evidence moved to slides-reads, and which has no `blocks` edge) and omits `story:catalog-google-slides-writes` (which has the edge), so the body contradicts its own relations — .engineering/planning/credential-blocker/google-oauth-client.md:30

**What I read.**
- 15 artifacts: the epic, the 12 stories, the ADR, the blocker and review-result:google-plan-design-round-1.
- Commands: `aep plan artifact show` on each, `relations`, `graph` and `validate`.
- Code read: `adapters/catalog/src/lib.rs` (guard references at `:119-126` and `:449-470`, `declare` at `:669-720`), `crates/connectors-catalog/src/inventory.rs`, `adapters/catalog/tests/bundle_drift.rs`, `crates/connectors-build/src/main.rs`, `docs/local-catalog-provider.md:100-170` and `apps/connectors/src/local/session.rs`.
- I walked about 50 edges, including the `informed_by`, `serves` and `blocks` edges outside the set, and found no cycle. The `depends_on` graph is acyclic.

**What I could not establish**
- **Blocker gating:** I did not confirm how `aep` gates `depends_on` on a story whose acceptance carries blocked evidence. Finding 2 assumes a dependent waits for `implemented`.
- **Link check:** I did not confirm the repository gate rejects a link to a missing `docs/` file. The website docs pass rewrites links (`crates/connectors-build/src/docs.rs:91`), so finding 1 may only show at website build.
- **Out of my lane:**
  - The ADR is `proposed` while the epic table lists it as the decider (scope or lifecycle).
  - The drive-reads title still says "and the four pinned Google Discovery sources" though the body says it does not re-pin them (naming).
  - The projection story's scope omits `crates/connectors-catalog/src/pipeline.rs`, though `--derived-from` reaches the pipeline (`crates/connectors-catalog/src/pipeline.rs:1`) (parallel-safety).
- **`validate`:** it printed `valid` plus outcome reminders on old review-results, none about this set.

```findings
- file: .engineering/planning/story/catalog-google-drive-reads.md
  line: 75
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance requires docs/catalog-google-drive.md to link docs/catalog-google-oauth.md, a file only story:cli-oauth-loopback-acquisition creates, and drive-reads has no depends_on to it (slides-reads does), so add that edge or move the link to slides-reads"
- file: .engineering/planning/story/catalog-google-slides-reads.md
  line: 56
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it is the only story with a blocked live-evidence acceptance sitting inside the shared-file read chain, so calendar-reads, gmail-reads and (through acquisition, blocked the same way) all four write stories wait on an operator console step unrelated to the index.json and bundle_drift.rs ordering the edges exist for; move the live evidence to an epic-level acceptance or its own verification item, or put the blocked story last in the chain"
- file: .engineering/planning/credential-blocker/google-oauth-client.md
  line: 30
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the What it withholds section still names story:catalog-google-drive-reads (whose live evidence moved to slides-reads and which has no blocks edge) and omits story:catalog-google-slides-writes (which has the edge), so the body contradicts its own relations"
```
