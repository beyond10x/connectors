---
format: aep.planning-md/3
id: decision-blocker:wave-20261010a-scope
kind: decision-blocker
status: cleared
title: Which parity stories wave 20261010a delivers
relations:
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "open", to: "cleared", at: "2026-10-09T22:50:51Z", actor: "human:timo", revision: 2}
---
## Question

Which parity stories form wave 20261010a, after the MySQL wave, ordered by session call count?

## Decided (2026-10-10)

Option A: `story:parity-jira-transitions` (242 calls), `story:parity-gitlab-mr-writes` (159) and
`story:parity-gitlab-repository-reads` (125), then the ESS pin move from 0.56.0 to 0.57.0 as the
last unit. One tree, one integration branch, one pull request, release 0.41.0 when it is green.
An adversary reviews the two write units.

Not in this wave:

- `story:parity-jira-issue-writes` (326 calls) waits: other sessions' unmerged Jira issue-create
  and write-hardening work covers the same operations.
- `story:parity-slack-message-writes` (162) waits for the read-only Slack handler to be set up
  first.
