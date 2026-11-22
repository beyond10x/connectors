---
format: aep.planning-md/3
id: review-result:slack-complete-web-api-scope-round-2
kind: review-result
status: active
title: Scope critic, round 2, epic:slack-complete-web-api
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

What I read: 8 artifacts (the epic and the 7 stories) with `aep plan artifact show` on each, plus `aep plan artifact show review-result:slack-complete-web-api-scope-round-1` and the head of `aep plan artifact graph`. I wrote down 9 promises from the epic before reading the stories and traced all 9 to an item:
- The catalog describes each method as Slack documents it: story 4, with its descriptions.
- It covers the four profiles `slack.user`, `slack.bot`, `slack.app` and `slack.config`: story 4.
- The operation lists are generated from pinned sources: stories 1, 2 and 4.
- The allow-list and write approvals are per-instance policy: they exist already, and story 6 tests them.
- Admission checks each operation's scopes against the token's scopes: story 5.
- "No user writes" becomes per-instance policy: the writes arrive with story 4 and the policy is story 6.
- Socket Mode is its own story, out of the first wave: story 7.
- The xoxe token-rotation rules are marked unverified: story 2.
- The `## Noun` item (granted scopes as a new fact about a connection, modeled first): story 5.

None of the exclusions (raw pass-through, other Slack APIs, releases) is reached by any item.

Round 1's finding is fixed. In `story:slack-user-writes-by-policy`, the Outcome now says the writes "arrive with `story:slack-four-profile-definition-generated`, which also replaces the old catalog assertion". Its Change is down to a policy test, an approval test and docs. Story 4's Change names the replaced assertion at `adversary_credential_pass1.rs:290-296`, and its acceptance exempts that line from "existing tests pass unchanged". Each outcome is now claimed once.

What I could not establish:
- Out of my lane (design): story 6's title still reads "The Slack user profile carries writes". The body disclaims it, so I did not count it as a finding.
- Out of my lane (design or parallel-safety): `story:slack-reactions-and-channel-info` and `story:parity-slack-files` sit under other epics and overlap what story 4 generates. This is unchanged from round 1.
- Out of my lane (acceptance): story 2's ten named rows are "expected from … known on 2026-11-23" and confirmed at generation.
- The epic file has no `path:line`, so citations would have been `aep plan artifact show` commands. There are no findings.

```findings
[]
```
