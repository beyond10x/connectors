---
format: aep.planning-md/3
id: review-result:google-plan-parallel-round-2
kind: review-result
status: active
title: Plan critic (parallel), round 2, epic:google-workspace-reads
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

story:catalog-google-slides-writes — five of the write operations (`presentations.create`, `files.create`, `files.copy`, `events.insert`, `users.drafts.create`) are specified as a "postflight `id` present" guard with no preflight. Today's guard cannot express that, so someone has to edit `adapters/catalog/src/lib.rs` (inferred, no body names it). Slides-writes, drive-writes, calendar-writes and gmail-draft-writes are unordered against each other and against `story:catalog-repeated-query-parameters`, which also edits that file, and none of the bodies says so. Either name the guard-engine change as its own story that all four writes depend on (an ordering edge whose reason is `adapters/catalog/src/lib.rs`), or state in each body that the create guards are dropped or restated within today's model. — .engineering/planning/story/catalog-google-slides-writes.md:30

Round-1 finding: fixed. `story:cli-oauth-loopback-acquisition` now owns `docs/catalog-google-oauth.md` alone. Its body says "its own file, so no provider story shares it". The provider guides only link to it. No other story in the set touches `adapters/catalog/tests/local_runtime/cli_journey.rs`.

**What I read.** 12 stories plus `review-result:google-plan-parallel-round-1`. I ran `aep plan artifact show` on each and `aep plan artifact waves --format json`, and filtered its collisions to the set. I also read `adapters/catalog/src/lib.rs`, `bundle_drift.rs`, `docs/local-catalog-provider.md:135-175` and the manifests. Surfaces established: 12 cited from bodies, 0 inferred, 0 unplaceable. The tool marks every stored scope entry `inferred`. The one finding rests on a surface the writes stories do not name, so it is marked inferred above.

Findings that turn out to be non-findings, checked:
- **First wave** (oauth2-refresh, discovery-projection, repeated-query): no colliding pair, and the tool reports none within the set.
  - `Cargo.lock` is discovery's alone. Moving `sha2` to `[dependencies]` in `adapters/catalog/Cargo.toml` changes no lock entry, and `base64` and `zeroize` are already regular dependencies. `openapiv3` appears in no manifest or lockfile yet.
  - `connectors-catalog/src/lib.rs` is discovery's and `inventory.rs`/`template.rs` are repeated-query's, so no file is shared.
  - `adapters/catalog/src/lib.rs` is repeated-query's and `local.rs` is oauth2's.
- **Providers**: the four reads are strictly ordered on `bundle_drift.rs` (`SOURCES` is a fixed-size array) and `index.json`.
  - Bundles derive from the source alone, not from the selection, so the writes stories need not touch them.
  - Each provider's writes edit only that provider's own three files, and each depends on that provider's reads.

**What I could not establish.**
- Out of my lane (acceptance/design): the create guards are not merely unplaced but not expressible. `Guard.preflight` must carry at least one check (`adapters/catalog/src/lib.rs:308-313`), and `Expectation` is only `Input` or `Literal` (`:34-38`), so nothing means "present". This probably needs its own story. Whether these stories should exist in this form is for the design and acceptance critics.
- Out of my lane: the oauth2-refresh body says `sha2` is "moves from `[dev-dependencies]`", which is right. My round-1 note about "added" is obsolete.
- Where repeated-query's `engine_*` tests land is not stated. I infer `adapters/catalog/tests/engine.rs`, which nothing else in the set touches. It collides with `story:catalog-parameters-declare-their-type`, which is outside the set, so I did not assess it.

```findings
- file: .engineering/planning/story/catalog-google-slides-writes.md
  line: 30
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the create guards of this story and of story:catalog-google-drive-writes, story:catalog-google-calendar-writes and story:catalog-google-gmail-draft-writes are 'postflight id present' with no preflight, which today's Guard (a mandatory non-empty preflight, equality-only Expectation) cannot express, so each would have to edit adapters/catalog/src/lib.rs (inferred, no body names it), where the four are unordered against each other and against story:catalog-repeated-query-parameters; add an ordering edge recording that file as its reason (for example through one story that owns the guard-engine change) or state in each body how its guards stay within the existing model"
```
