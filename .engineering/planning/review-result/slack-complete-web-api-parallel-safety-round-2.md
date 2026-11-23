---
format: aep.planning-md/3
id: review-result:slack-complete-web-api-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, round 2, epic:slack-complete-web-api
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
approve

**What you read:** 7 stories and the round-1 record, with `aep plan artifact show` on each and on `review-result:slack-complete-web-api-parallel-safety-round-1`. I also ran `aep plan artifact graph` and `aep plan artifact waves`. In the tree I read `crates/connectors-catalog/src` (`pub fn lower` is at `definition.rs:4227`) and `adapters/catalog/tests`.

Round 1's ten findings are all closed:
- **Missing edges:** `depends_on` now runs from the scope table, the write guard and the generated definition to the import story. It also runs from the admission story to the generated definition.
- **Shared crate:** the import story's body now names `connectors-catalog/src/import.rs` and its ordering before the write guard.
- **Scope carrier:** the admission story's body says admission reads the field the generated definition adds to the lowered operation record.
- **Socket Mode:** it now depends on stories 4, 5 and 6, so it comes after every other item in the set.
- **Empty `scope`:** all seven stories have a structured `scope`. `waves` lists none of them as unassessed.

**Counts:**
- Surfaces placed from paths the bodies cite: 6.
- Surfaces inferred: 1, the socket-mode story's `adapters/catalog/src`.
- Items not placed at all: 0.
- Acceptance reads traced to a producer in the set: 7. They are 2 and 4 reading 1's projection, 4 reading 2's table, 4 reading 3's guard, 5 and 7 reading 4's operation records, and 6 reading the writes 4 adds.
- Recorded by `depends_on`: 7 of 7.
- `waves` collisions between members of the set: 6, all already ordered by an edge.
  - 1 and 2 share `adapters/slack/upstream` and `crates/connectors-build`.
  - 1 and 4 share `crates/connectors-build`.
  - 2 and 4 share `crates/connectors-build`.
  - 3 and 4 share `definition.rs`.
  - 5 and 7 share `adapters/catalog/src`.
- The pairs with no edge between them do not share a file. They are 2 and 3, 3 and 6, and 5 and 6.

**What you could not establish:**
- Whether 6's acceptance "No engine code gains a rule naming `slack.user`" can be broken by 5, which has no edge to 6. 5's body names no such rule, so this is an unease, not a finding.
- 5 and 6 can be worked at once, and 6's approval-test fixture runs through the admission path 5 changes. 5's "unknown scopes are not refused" rule keeps a fixture without recorded scopes passing.
- The socket-mode story's body still reads "Scope: Not assessed" while its structured `scope` holds `adapters/catalog/src`. It is ordered after everything else in the set, so this does not affect concurrency. Out of my lane, as body consistency.

```findings
[]
```
