---
format: aep.planning-md/3
id: review-result:slack-complete-web-api-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1, epic:slack-complete-web-api
relations:
- reviews: story:slack-sdk-upstream-import
- reviews: story:slack-method-token-scope-table
- reviews: story:definitions-default-write-guard
- reviews: story:slack-four-profile-definition-generated
- reviews: story:admission-checks-token-scopes
- reviews: story:slack-user-writes-by-policy
- reviews: story:slack-socket-mode-stream
revision: 1
---
needs-revision

- story:slack-method-token-scope-table — its acceptance reads the projection `story:slack-sdk-upstream-import` produces ("Every method of … projection has a row") and has no `depends_on` edge to it, so one wave would hold both. The two also share `adapters/slack/upstream/` (cited, both Scope sections) and the `crates/connectors-build` importer binary (cited). Fix by adding the edge from this story to the import story, or by merging the two. — .engineering/planning/story/slack-method-token-scope-table.md:31, .engineering/planning/story/slack-sdk-upstream-import.md:37
- story:slack-sdk-upstream-import — its Scope names only `crates/connectors-build`, but the importer it extends is `connectors_catalog::import::import` in `crates/connectors-catalog/src/import.rs`. That is the crate `story:definitions-default-write-guard` changes (format and lowerer, `definition.rs`), and no edge or body line admits the overlap (inferred: the file is found from the call site, not named in either body). Either record an ordering edge with the shared crate as its reason, or split so the new source reader sits outside `connectors-catalog`. — .engineering/planning/story/slack-sdk-upstream-import.md:37, crates/connectors-build/src/main.rs:345
- story:admission-checks-token-scopes — "Operations carry their required scopes per profile from the catalog" does not say where those scopes are stored. Three places are possible: a new field in the `connectors-catalog` definition format, the generated operation records `story:slack-four-profile-definition-generated` writes, or a lookup in the table. Each choice collides with, or reads from, an item in the set that it has no edge to. If the scopes live on operation records, the acceptance reads a producer (story 4) with no edge; if in the format, it shares `definition.rs` with story 3 (inferred). Name the carrier in the body, then add the matching edge or Scope path. — .engineering/planning/story/admission-checks-token-scopes.md:25
- story:slack-socket-mode-stream — its Scope reads "Not assessed" and its acceptance is deferred. `aep plan artifact waves` prints it as `unassessed`, never placed. Its only edge is `depends_on` story 4, which would put it in the same wave as story 6. "Out of the first wave" is prose, not an edge or a scope. Either establish a surface, or keep it out of every concurrent set by a recorded ordering such as a `depends_on` on the last first-wave story. — .engineering/planning/story/slack-socket-mode-stream.md:35
- story:slack-sdk-upstream-import — the structured `scope` field is empty (Scope is prose only), so `aep plan artifact waves` lists it as `unassessed` and never places it, and its shared paths with story 2 and story 4 are not computed. Record the cited and inferred paths in `scope`. — `aep plan artifact waves` (output line "unassessed: story:slack-sdk-upstream-import")
- story:slack-method-token-scope-table — empty structured `scope`, printed as `unassessed` by `waves`. — `aep plan artifact waves` (output line "unassessed: story:slack-method-token-scope-table")
- story:definitions-default-write-guard — empty structured `scope`, printed as `unassessed` by `waves`. — `aep plan artifact waves` (output line "unassessed: story:definitions-default-write-guard")
- story:slack-four-profile-definition-generated — empty structured `scope`, printed as `unassessed` by `waves`. — `aep plan artifact waves` (output line "unassessed: story:slack-four-profile-definition-generated")
- story:admission-checks-token-scopes — empty structured `scope`, printed as `unassessed` by `waves`. — `aep plan artifact waves` (output line "unassessed: story:admission-checks-token-scopes")
- story:slack-user-writes-by-policy — empty structured `scope`, printed as `unassessed` by `waves`. — `aep plan artifact waves` (output line "unassessed: story:slack-user-writes-by-policy")

**What you read:** 8 artifacts (the 7 stories and the epic) with `aep plan artifact show`, plus `aep plan artifact graph`, `relations` and `waves`. I also read `crates/connectors-build/src/main.rs:345`, `crates/connectors-catalog/src`, `adapters/slack/upstream/`, `adapters/catalog/providers/slack/` and the catalog-slack-reads store record for comparison.

**Counts:**
- Surfaces by item: 6 items placed from prose Scope sections, all marked "Inferred" or partly cited; 0 of them recorded structurally; 1 (socket-mode) could not be placed.
- Of the 6 placed, 2 have bodies that cite paths (stories 1 and 2). The rest are inferred from named crates or directories.
- Acceptance reads traced to a producer in the set: 5.
- Recorded by `depends_on`: 4. Unrecorded: 1 (story 2 reading story 1). The fifth, story 5 reading scopes from the catalog, is ambiguous as written (see the third finding).
- Edges that held up: 4→1, 4→2, 4→3, 5→2, 6→4 and 7→4 are all present, and 6's `user-operations.json` is ordered behind 4's generator.

**What I could not establish:**
- The surface of story 7.
- Which file story 3 edits for the lowerer. I saw `definition.rs` and `import.rs` in `connectors-catalog`, but the body names neither.

Design and acceptance questions I saw but left alone as out of my lane:
- Story 5's `x-oauth-scopes` header is marked inferred.
- Story 4's "today's 23 operations keep their ids" depends on amendments surviving the new source.

```findings
- file: .engineering/planning/story/slack-method-token-scope-table.md
  line: 31
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance reads the projection story:slack-sdk-upstream-import produces and no depends_on edge runs from this story to it; both also land on adapters/slack/upstream/ (cited) and crates/connectors-build (cited); add the edge or merge the two"
- file: .engineering/planning/story/slack-sdk-upstream-import.md
  line: 37
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Inferred surface: the importer called from crates/connectors-build/src/main.rs:345 lives in crates/connectors-catalog/src/import.rs, the crate story:definitions-default-write-guard changes, and the Scope names only connectors-build; record an ordering edge with the shared crate as reason, or split the surface"
- file: .engineering/planning/story/admission-checks-token-scopes.md
  line: 25
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "body does not say where operations carry their scopes (definition format, generated operation records from story:slack-four-profile-definition-generated, or table lookup); on the first two it shares definition.rs with story:definitions-default-write-guard or reads story 4's output with no edge (inferred surface); name the carrier and add the matching edge or scope path"
- file: .engineering/planning/story/slack-socket-mode-stream.md
  line: 35
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Scope is 'Not assessed', so waves leaves it unassessed; its only edge (depends_on story 4) would place it beside story:slack-user-writes-by-policy, and 'out of the first wave' is prose, not a recorded ordering or scope"
- file: aep plan artifact waves
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:slack-sdk-upstream-import has an empty structured scope (prose Scope only), so waves prints it unassessed and never computes its shared paths; record cited and inferred paths in scope"
- file: aep plan artifact waves
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:slack-method-token-scope-table has an empty structured scope, so waves prints it unassessed"
- file: aep plan artifact waves
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:definitions-default-write-guard has an empty structured scope, so waves prints it unassessed"
- file: aep plan artifact waves
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:slack-four-profile-definition-generated has an empty structured scope, so waves prints it unassessed"
- file: aep plan artifact waves
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:admission-checks-token-scopes has an empty structured scope, so waves prints it unassessed"
- file: aep plan artifact waves
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:slack-user-writes-by-policy has an empty structured scope, so waves prints it unassessed"
```
