---
format: aep.planning-md/3
id: review-result:slack-complete-web-api-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1, epic:slack-complete-web-api
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

- story:slack-user-writes-by-policy — its Outcome and Change repeat what story:slack-four-profile-definition-generated already claims ("The user profile `slack.user` carries the writes a user token may call", "The user selection gains the writes … gives it"). Both would be marked done for the same outcome, and story 4's acceptance ("Today's 23 operations keep … their existing tests pass unchanged") collides with the user test at `adapters/catalog/tests/adversary_credential_pass1.rs:290-296` that this story replaces. Narrow the Outcome and Change to the policy only: the read-only user instance, the replaced test, and the docs. Leave the writes to story 4, and state which of the two stories changes that assertion. — `aep plan artifact show story:slack-user-writes-by-policy` (## Outcome, ## Change); `aep plan artifact show story:slack-four-profile-definition-generated` (## Outcome, ## Acceptance)

What I read: 8 artifacts, the epic and the 7 stories, with `aep plan artifact show` on each, plus `aep plan artifact graph` and `adapters/catalog/tests/adversary_credential_pass1.rs:280-300`. I extracted 9 promises from the epic and traced all 9 to a story:
- the four profiles → story 4
- generated from pinned sources → stories 1, 2, 4
- the per-method description → story 4
- policy over write limits → story 6
- the scope check → story 5
- the allow-list and the approval for writes already exist, per the epic's "Today" section
- Socket Mode as its own story → story 7
- xoxe rules marked unverified → story 2
- the exclusions (raw pass-through, other Slack APIs, releases) → nothing in the set reaches into them

What I could not establish:
- Out of my lane (design or parallel-safety): stories 4 and 6 edit the same user selection and the same test.
- Out of my lane (design or parallel-safety): the older draft stories `story:slack-reactions-and-channel-info` (`reactions.add`, `conversations.info`) and `story:parity-slack-files` (`slack.file.upload`) sit under other epics. They overlap what story 4 generates, and story 4's acceptance does not mention them. They are outside the parent, so they do not set my verdict.
- The epic file has no `path:line`, so citations are `aep plan artifact show` commands.

```findings
- file: "aep plan artifact show story:slack-user-writes-by-policy"
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Outcome and Change claim that the user profile carries the writes, which story:slack-four-profile-definition-generated already claims, and story 4's 'existing tests pass unchanged' collides with the user-writes test this story replaces; narrow this story to the policy test, the read-only user instance and the docs, and state which story changes the assertion at adversary_credential_pass1.rs:290-296"
```
