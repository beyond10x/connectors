---
format: aep.planning-md/3
id: upstream-blocker:definitions-lack-agent-projections
kind: upstream-blocker
status: open
title: connectors-definitions has no agent projection definitions
summary: jira-agent.yaml, slack-agent.yaml and the agent rule are missing at 536945d
relations:
- blocks: story:agent-tool-projection
revision: 1
---
# connectors-definitions has no agent projection definitions

story:agent-tool-projection lowers two definitions that `beyond10x/connectors-definitions` does not
carry yet, and enforces a rule its schema does not state.

Needed on connectors-definitions `main`:
- `connectors/jira-agent.yaml` (`catalog: jira`): `issues.search` (withholding `properties`,
  `fieldsByKeys`, `failFast`, `reconcileIssues`), `issue.get`, `issue.comments`.
- `connectors/slack-agent.yaml` (`catalog: slack`): `conversations.list`,
  `conversations.history`, `conversations.replies`, `users.info`.
- `spec/domains/definition.yaml`: an `agent` flag on a definition; a definition marked
  `agent: true` gives every operation a `description` of at most 200 bytes and selects at most 8
  operations.

Observed 2026-10-22: connectors-definitions `main` `536945d` has neither file
(`connectors/README.md:3-6` lists the suffixed selections, `slack-user` among them) and no `agent`
field in `definition.yaml`.

Clears when a connectors-definitions commit on `main` carries all three and its checks pass; that
commit is recorded here as evidence.
