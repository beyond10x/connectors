---
format: aep.planning-md/3
id: review-result:slack-complete-web-api-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1, epic:slack-complete-web-api
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

- story:admission-checks-token-scopes — the Outcome's second sentence, "The token's scopes are recorded at connect and on revalidate", has no acceptance bullet. Both bullets start from "a recorded answer" and test only the refusal, so the work can be marked done with nothing recording scopes at connect or revalidate. Add a bullet with a before and an after, for example a connection whose stored scopes are empty before connect or revalidate and hold the header's scopes after. — .engineering/planning/story/admission-checks-token-scopes.md:32
- story:slack-method-token-scope-table — "Ten named rows match their pages" gives expected values for only four of the ten rows (`search.messages` user only, `apps.connections.open` app-level only, `apps.manifest.create` configuration only, and `conversations.replies` and the rest nothing). The Change section rules out network access in tests, so "match their pages" is a manual comparison that nobody can repeat identically. State the expected token types and scopes for each named row, so a test or a reader gets the same answer twice. — .engineering/planning/story/slack-method-token-scope-table.md:33
- story:slack-four-profile-definition-generated — "Every method of the table appears under each profile its row allows, and under no other" does not say which profiles a method gets when its row is `unknown`, which `story:slack-method-token-scope-table` allows for token type and scope. Two readers can pass or fail the same generated definition, and the count comparison cannot tell them apart. Name the rule: either an `unknown` row appears under no profile, or it appears under every profile, and check it with a counted case. — .engineering/planning/story/slack-four-profile-definition-generated.md:33

What I read: 7 of the 7 ids you gave, with `aep plan artifact show &lt;id&gt;` on each, plus `aep plan artifact kinds`, `aep plan artifact lifecycle story`, the parent epic's body, and the story files. I also ran `git grep`-style checks of the tree for the Slack upstream directory, the catalog provider files and `adversary_credential_pass1.rs:290-296`.

What I could not establish:
- `story:slack-socket-mode-stream` has the placeholder acceptance "Written once the open items are decided." I did not count it as a finding because the story is deliberately deferred and its open design questions block any honest check. Someone must still write a checkable acceptance before it leaves draft.
- Whether `x-oauth-scopes` is the real source of a token's scopes. The story marks this as inferred, and it is a design question, not an acceptance one.
- Whether `story:slack-user-writes-by-policy` should split its first bullet in two, since it joins the refusal test and the approval-proof test with a semicolon. I judged each part checkable and did not count it.
- Out of my lane: the scope lines in every story say "Inferred", and I did not check them. Whether `story:slack-four-profile-definition-generated` should depend on the default write guard in both its Change and its depends_on is for the design and scope critics.

```findings
- file: .engineering/planning/story/admission-checks-token-scopes.md
  line: 32
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Outcome says the token's scopes are recorded at connect and on revalidate, but no acceptance bullet checks the recording, so that outcome can be marked done with nothing stored"
- file: .engineering/planning/story/slack-method-token-scope-table.md
  line: 33
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'Ten named rows match their pages' states expected token types and scopes for only four of the ten rows, and tests make no network request, so the check is a manual comparison that is not repeatable"
- file: .engineering/planning/story/slack-four-profile-definition-generated.md
  line: 33
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'appears under each profile its row allows, and under no other' does not say where a method with an unknown token type goes, so two readers can disagree on the same generated definition"
```
