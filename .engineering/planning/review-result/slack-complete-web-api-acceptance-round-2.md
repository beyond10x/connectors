---
format: aep.planning-md/3
id: review-result:slack-complete-web-api-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2, epic:slack-complete-web-api
relations:
- reviews: story:slack-sdk-upstream-import
- reviews: story:slack-method-token-scope-table
- reviews: story:definitions-default-write-guard
- reviews: story:slack-four-profile-definition-generated
- reviews: story:admission-checks-token-scopes
- reviews: story:slack-user-writes-by-policy
- reviews: story:slack-socket-mode-stream
revision: 2
---
needs-revision

- story:slack-four-profile-definition-generated — the Outcome's sentence "Each operation carries its required scopes per profile as data" has no acceptance bullet. The four bullets test placement per profile, the unknown row, the 23 old operations and the budget, so the story can be marked done with no scopes in the definition or the lowered records. Add a bullet with a before and an after, for example: a lowered `conversations.replies` under `slack.user` has an empty scope list before the work and `channels:history`, `groups:history`, `im:history`, `mpim:history` after. — .engineering/planning/story/slack-four-profile-definition-generated.md:29

Round 1's three findings are fixed:
- **`story:admission-checks-token-scopes`:** a bullet now records scopes at `connect` and `revalidate`, with an empty store before and the header's scopes after.
- **`story:slack-method-token-scope-table`:** all ten rows now state token types, scopes and effect.
- **`story:slack-four-profile-definition-generated`:** an `unknown` row now appears under no profile, with a fixture case. The one new finding above concerns a different sentence of the same story.

What I read: 7 of the 7 ids, with `aep plan artifact show story:&lt;id&gt;` on each. I also read `review-result:slack-complete-web-api-acceptance-round-1`, `aep plan artifact kinds`, `aep plan artifact lifecycle story`, and the line numbers in the four-profile story file.

What I could not establish:
- `story:slack-socket-mode-stream` still has the placeholder acceptance "Written once the open items are decided." I did not count it, as in round 1: the story is deferred and in draft. Someone must still write a checkable acceptance before it leaves draft.
- Whether `node-slack-sdk` at the pinned commit really binds 303 methods. The source is not in the tree, so I could not check it. The count bullet is observable once the commit is fixed.
- `story:admission-checks-token-scopes` names a "named code" in its Outcome that no bullet or other text names. I judged "is refused" to cover it, so I did not count it.
- Out of my lane: whether `x-oauth-scopes` is the real source of a token's scopes. That is a design question.

```findings
- file: .engineering/planning/story/slack-four-profile-definition-generated.md
  line: 29
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: undecided
  message: "the Outcome says each operation carries its required scopes per profile as data, but no acceptance bullet checks the scopes in the definition or the lowered operation records, so the story can be marked done with no scopes carried"
```
