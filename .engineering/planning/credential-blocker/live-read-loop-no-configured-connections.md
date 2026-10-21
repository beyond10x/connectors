---
format: aep.planning-md/3
id: credential-blocker:live-read-loop-no-configured-connections
kind: credential-blocker
status: open
title: The operator's host has no live connection for the 10-minute read loop
relations:
- blocks: story:sustained-live-read-loop
withholds: test_result
revision: 1
---
# The operator's host has no live connection for the 10-minute read loop

`story:sustained-live-read-loop` accepts on a 10-minute run against live Slack, Jira, GitLab,
Kubernetes and SQL connections on the operator's host. Its runner is already in the tree
(`crates/connectors-build/src/live_read_loop.rs`, commit `b6aa95a23c`); what remains is the run.

Observed 2026-10-21 with connectors 0.54.0 on the host the wave ran on:

- `connectors setup check` reports configured adapter entries for confluence, jira, slack-bot,
  slack-search and others, all `ready`.
- `connectors connections list --adapter <alias> --output json` answered `[]` for confluence,
  jira, slack-bot and slack-search: no saved connection.
- No GitLab, Kubernetes or SQL adapter entry is configured.

The wave of 2026-10-21 may read live only on connections already configured on that host, so
the run cannot happen there. Two `depends_on` stories are also still draft:
`story:upgrade-moves-catalog-adapters` and `story:refusals-name-the-field`.

Clears when an operator host carries ready connections for the five providers (or the operator
narrows the provider list) and the two dependencies are implemented. Related:
`credential-blocker:sandbox-accounts-jira-confluence-hubspot`.
