---
format: aep.planning-md/3
id: review-result:slack-complete-web-api-design-round-2
kind: review-result
status: active
title: Design critic, round 2, epic:slack-complete-web-api
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

What I read: 8 artifacts (the epic, the 7 stories) and `review-result:slack-complete-web-api-design-round-1`, through `aep plan artifact show`. I also ran `aep plan artifact relations`, `graph` and `validate`. `validate` printed `valid`, with only review-outcome warnings on other artifacts.

Round 1's three blockers are fixed in the store:
- **Table depends on import.** `story:slack-method-token-scope-table` now has `depends_on story:slack-sdk-upstream-import`.
- **User-write test.** `story:slack-four-profile-definition-generated` now owns the replacement of the `adversary_credential_pass1.rs:290-296` assertion. Its acceptance says so, and `story:slack-user-writes-by-policy` is reduced to policy and approval tests and docs over it, with a `depends_on` edge.
- **Scopes as data.** Four-profile now adds required scopes as a field of the operation in the definition format and the lowered record. Admission reads that field and has a `depends_on` edge to four-profile.

Edges and acceptance reads:
- **Edges walked:** 18 inside the set (7 `decomposes` and 11 `depends_on`). I also walked outside it, to `epic:slack-complete-web-api` and `vision:independent-contract-adapters`. There is no cycle. The `depends_on` graph has real branching (import fans out to the table and the guard, which join at four-profile), so it is not a queue.
- **Acceptance reads:** I traced 6 to a producer in the set, and an edge records all 6.
  - Table reads import's projection.
  - Four-profile reads the projection, the table and the guard default.
  - Admission reads the scopes field that four-profile adds.
  - User-writes reads the user-profile writes that four-profile generates.
- **Shared files:** the shared-file orderings (`crates/connectors-build` and `definition.rs`) each have an edge.

What I could not establish, all outside my lane:
- **Acceptance:** four-profile's acceptance has no line that checks the scopes field is present on the lowered operation records. Admission is the only test that reads it.
- **Acceptance:** `story:slack-socket-mode-stream` has no acceptance yet, by design. Its edges to admission and user-writes name no read.
- **Scope:** `story:definitions-default-write-guard` has an edge to import because both touch `connectors-catalog`. The files differ (`import.rs` and `definition.rs`), so the edge may be stronger than needed. It serialises nothing.

```findings
[]
```
