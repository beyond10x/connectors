---
format: aep.planning-md/3
id: review-result:google-plan-scope-round-1
kind: review-result
status: active
title: Plan critic (scope), round 1, epic:google-workspace-reads
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
approve

Read 15 artifacts: the epic, the 12 stories, the ADR and the credential blocker. I ran `aep plan artifact show` on each and `aep plan artifact graph` for the store. I wrote the epic's promise list before opening any story. It has 16 promises and I traced 16 to an item. Fifteen are claimed by an item. The sixteenth, "drop the hand-written TOML prototype", is honoured by omission. `git grep` finds no Google TOML action in the tree, and no story builds on the authored-TOML path (`story:catalog-local-toml-action`, an unrelated, implemented story). No item reaches beyond the epic. The pinned Discovery documents (drive-reads), the id-token/tokeninfo identity source (refresh-profile) and the `semantics.md` amendment (acquisition, ADR point 4) all trace to an epic sentence or to the epic's cited ADR. Nothing lands in the epic's out-of-scope list: media upload paths are excluded and listed, and `alt=media` is not projected. No two items claim the same outcome. The only overlap is the shared `docs/catalog-google.md`, and each story owns a separate section. Not every Discovery method is selected (for example 6 of 64 for Drive). The epic promises "reads" and never all methods, so this is not a narrowing.

What I could not establish:
- Where the TOML prototype lived. It is not in the tracked tree, so I could not check that nothing of it remains.
- Out of lane, for design or parallel-safety: the epic's story table gives `story:catalog-google-slides-writes` the dependencies "acquisition, Slides reads", but the story also depends on `story:catalog-google-gmail-reads`.
- Out of lane, for design or parallel-safety: `story:catalog-discovery-projection` tests use a second copy of the four pinned documents under `crates/connectors-catalog/tests/discovery/pinned/`, and it cites "the four pinned documents" that `story:catalog-google-drive-reads` pins while depending on nothing itself.
- Out of lane, for acceptance: `story:catalog-google-slides-writes` requires live evidence on "a scratch deck the operator names", and no credential blocker covers that. The epic asks for live evidence on the read path only.

```findings
[]
```
