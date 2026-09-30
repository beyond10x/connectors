---
format: aep.planning-md/3
id: review-result:google-plan-parallel-round-1
kind: review-result
status: active
title: Plan critic (parallel), round 1, epic:google-workspace-reads
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

story:cli-oauth-loopback-acquisition — this story and story:catalog-google-drive-reads both create `docs/catalog-google.md`, which does not exist yet (cited, both bodies). The story has no ordering path to drive-reads or to the slides-reads, calendar-reads and gmail-reads that follow it, and none of the bodies mention the collision. Either add an ordering edge whose reason is the shared guide, or split the guide so the OAuth section lives in its own file — .engineering/planning/story/cli-oauth-loopback-acquisition.md:67

**What you read.** 12 stories plus `epic:google-workspace-reads`, via `aep plan artifact show` on each and `aep plan artifact waves --format json`.
- Surfaces established: 12 cited, 0 inferred, 0 unplaceable. The tool marks every stored scope entry `inferred`, but each body names its paths.
- Intended first wave (oauth2-refresh, discovery-projection, repeated-query): no colliding pair.
  - The three write to different files.
  - `sha2` is already in `adapters/catalog/Cargo.toml:32`, as a dev-dependency. Moving it to `[dependencies]` changes no `Cargo.lock` entry, so the `openapiv3` dev-dependency is the only `Cargo.lock` change (inferred from the manifests).
- Provider chain: drive-reads → slides-reads → calendar-reads → gmail-reads → slides-writes → drive-writes → calendar-writes → gmail-draft-writes is fully linear, so no pair in it runs together.
- Acquisition sits on only one edge, `depends_on story:catalog-oauth2-refresh-profile`. It first meets the chain at slides-writes, so four read stories can run alongside it, all on the same file.

**What I could not establish.**
- Out of my lane, from the body of oauth2-refresh (design bullet on `sha2`): it says `sha2` is "added to `adapters/catalog/Cargo.toml`", but it is already there as a dev-dependency. The change is a move to `[dependencies]`, not an addition.
- Out of my lane, from `aep plan artifact waves`: the three intended first-wave stories land in store-wide wave 3, so they overlap something outside this set.
  - `story:catalog-parameters-declare-their-type` names `adapters/catalog/src/lib.rs`, which repeated-query also edits.
  - I did not assess items outside the set.

```findings
- file: .engineering/planning/story/cli-oauth-loopback-acquisition.md
  line: 67
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: this story and story:catalog-google-drive-reads both create docs/catalog-google.md, which does not exist yet (cited, both bodies); the story has no ordering path to drive-reads or to the slides-reads, calendar-reads and gmail-reads that follow it, and no body mentions the collision, so add an ordering edge recording the shared guide as its reason or split the guide so the OAuth section is its own file
```
