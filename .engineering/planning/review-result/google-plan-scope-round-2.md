---
format: aep.planning-md/3
id: review-result:google-plan-scope-round-2
kind: review-result
status: active
title: Plan critic (scope), round 2, epic:google-workspace-reads
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

- story:catalog-google-drive-reads — its title still says "and the four pinned Google Discovery sources", but the epic gives the pinning to `story:catalog-discovery-projection` and this body says it "does not re-pin", so two items claim the same outcome by title — .engineering/planning/story/catalog-google-drive-reads.md:6
- story:catalog-google-slides-writes — the live `batchUpdate` on "a scratch deck the operator names" is not in the epic, whose live-run acceptance covers only the read path and whose write acceptance is "`effect: write` and refused without an approval", and no credential blocker covers that deck — .engineering/planning/story/catalog-google-slides-writes.md:44

**What you read.** 15 artifacts: the epic, 12 stories, the ADR and the credential blocker, each with `aep plan artifact show`. I also read `review-result:google-plan-scope-round-1` and ran `aep plan artifact graph`. I wrote the epic's promise list before opening any story.

- **Coverage.** I extracted 16 promises from the epic and traced 16 to an item, so there is no gap.
- **Round-1 point fixed.** The shared guide is now one file per provider plus `docs/catalog-google-oauth.md`, so the earlier overlap on `docs/catalog-google.md` is gone.
- **Round-1 point fixed.** The slides-writes dependency now matches the epic table.

**What I could not establish**
- Whether the earlier "hand-written TOML prototype" survives anywhere. It is not in the tracked tree, and I did not re-check it this round.
- Out of lane (acceptance): the slides-writes live evidence has no blocker, and the credential blocker's "What it withholds" text names `story:catalog-google-drive-reads`, which has no `blocks` relation.

```findings
- file: .engineering/planning/story/catalog-google-drive-reads.md
  line: 6
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the title still claims \"the four pinned Google Discovery sources\" although story:catalog-discovery-projection pins all four and this body says it does not re-pin, so two items claim the same outcome by title"
- file: .engineering/planning/story/catalog-google-slides-writes.md
  line: 44
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance demands live write evidence (a batchUpdate on a scratch deck the operator names) that the epic does not ask for, since its live-run acceptance covers the read path only and its write acceptance is effect write plus approval refusal"
```
