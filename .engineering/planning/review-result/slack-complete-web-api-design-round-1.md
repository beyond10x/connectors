---
format: aep.planning-md/3
id: review-result:slack-complete-web-api-design-round-1
kind: review-result
status: active
title: Design critic, round 1, epic:slack-complete-web-api
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

Findings:
story:slack-method-token-scope-table — its acceptance reads the projection that story:slack-sdk-upstream-import creates ("every method of the projection has a row"), and no depends_on edge runs from the table to the import, so the two can be scheduled together and the row-coverage test has nothing to read. Add `depends_on story:slack-sdk-upstream-import` — .engineering/planning/story/slack-method-token-scope-table.md:31, .engineering/planning/story/slack-sdk-upstream-import.md (Outcome, "The imported document lists every method")

story:slack-four-profile-definition-generated — its outcome puts user-token writes in `slack.user` and its acceptance keeps all existing tests passing unchanged, but the test `adapters/catalog/tests/adversary_credential_pass1.rs:290-296` asserts the user connection has "no write included" and fails once those writes are generated. The fix for that test sits in story:slack-user-writes-by-policy, so neither story can land green alone. Move the user-write generation and the test replacement into one story, or make the four-profile acceptance carve out that test and add a `depends_on` edge for the order — .engineering/planning/story/slack-four-profile-definition-generated.md:18,35 and .engineering/planning/story/slack-user-writes-by-policy.md:20,24

story:admission-checks-token-scopes — it says "Operations carry their required scopes per profile from the catalog", and its acceptance (`conversations.history` refused without `channels:history`) reads that per-operation scope data. No story in the set writes scopes into operations as data: four-profile only puts them in operation descriptions (line 29), and the table is a separate file. So the carrier is unowned. Either the body says admission reads the table directly, or story:slack-four-profile-definition-generated emits scopes as data and admission gets a `depends_on` edge to it — .engineering/planning/story/admission-checks-token-scopes.md:25 and .engineering/planning/story/slack-four-profile-definition-generated.md:29

**What you read**
- 8 artifacts: the epic and the 7 stories, through `aep plan artifact show`.
- `aep plan artifact relations`, `aep plan artifact graph` and `aep plan artifact validate`.
- The test file `adapters/catalog/tests/adversary_credential_pass1.rs` and `adapters/catalog/providers/slack/*.json`.

**Edges and acceptance reads**
- I walked 11 edges inside the set: 7 `decomposes` to the epic and 6 `depends_on`. I walked outside the set only to the epic and `vision:independent-contract-adapters`. I found no cycle and no serialising chain.
- I traced 8 acceptance reads to a producer in the set. 5 have an edge: four-profile to import, table, and guard; admission to table; socket-mode and user-writes to four-profile. 3 have no edge or no named carrier, and they are the findings above.

**What you could not establish**
- Whether `conversations.history` appears in the generated or the hand-written catalog at the admission acceptance. It is in the existing hand-written 18 bot operations, so that test may not need the generator.
- Out of my lane (acceptance): the `x-oauth-scopes` header claim is marked "verify" but has no gating acceptance line. The socket-mode acceptance is "written once the open items are decided".
- `aep plan artifact validate` printed `valid`, with only the existing review-outcome warnings on other artifacts.

```findings
- file: .engineering/planning/story/slack-method-token-scope-table.md
  line: 31
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance reads the projection that story:slack-sdk-upstream-import creates, and no depends_on edge runs from the table story to the import story; add depends_on story:slack-sdk-upstream-import"
- file: .engineering/planning/story/slack-four-profile-definition-generated.md
  line: 35
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its outcome generates user-token writes while its acceptance keeps existing tests unchanged, but adapters/catalog/tests/adversary_credential_pass1.rs:290-296 asserts no user writes and is only replaced in story:slack-user-writes-by-policy, so the half-abstraction split leaves neither story green alone; merge the user-write generation and the test replacement or carve the test out and record the order with a depends_on edge"
- file: .engineering/planning/story/admission-checks-token-scopes.md
  line: 25
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "it reads per-operation required scopes 'from the catalog' but no story in the set writes scopes into operations as data (four-profile only writes them into descriptions), so the carrier is unowned; say admission reads the table directly or have four-profile emit scopes as data and add a depends_on edge from admission to it"
```
