---
format: aep.planning-md/3
id: decision-blocker:wave-20261010b-scope
kind: decision-blocker
status: cleared
title: Which parity units wave 20261010b delivers
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-10T02:00:23Z", actor: "human:timo", revision: 3}
---
## Question

Which parity units form wave 20261010b, ordered by session call count, now that the remaining
GitLab and Jira gaps wait on catalog engine changes rather than on selections?

## Decided (2026-10-10)

Option A, three units, about 246 calls:

1. `story:catalog-guard-postflight-read` with the Jira `doTransition` selection
   (`jira.issue.transition.run`, 109 calls).
2. `story:catalog-selection-fixed-body-value` with the GitLab merge-when-pipeline-succeeds and
   reopen variants (`gitlab.mr.merge` 77 and `gitlab.mr.update` 60, partial to covered).
3. `story:parity-gitlab-mr-reads`, selection only (`gitlab.mr.changes` 58 partial,
   `gitlab.mr.discussion.list` 43 missing).

Units 1 and 2 change the same engine and selection-format files and run as one serial lane; an
adversary reviews both. Spec first for both. One tree builds at a time, at most 8G on disk:
the lane first, unit 3 after it (or beside it once `/` is at or over 35G free). One integration
branch, one pull request, release 0.42.0 when it is green.

Not chosen:

- B: also `story:catalog-path-correction-and-value-bound` with GitLab code search (75), a third
  engine change.
- C: unit 3 alone.

Not in this wave:

- `story:parity-jira-issue-writes` (326 calls) waits on other sessions' unmerged Jira write work.
- `story:parity-slack-message-writes` (162) waits for the read-only Slack handler.
