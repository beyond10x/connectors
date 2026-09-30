---
format: aep.planning-md/3
id: story:expired-evidence-invoke-advises-revalidate
kind: story
status: draft
title: An invoke on a connection whose evidence expired advises revalidate, not repair
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Observed

2026-09-30, live Jira through the Atlassian gateway (service account, basic auth, connectors 0.18.0 plus the gateway
unit): a connection that answered `ready` at connect was `pending` a few minutes later; `operations invoke` then
answered `not_granted` / `repair_connection` at `admission`. `connections revalidate` (no credential re-entry) brought it
back and the same invoke returned 10 issues. The knowledge-ingest consumer reported the same on 0.15.1 ("an invoke
returns not_granted / repair_connection rather than prompting a revalidate, while connections list says pending").

## Acceptance

- An invoke on a connection whose evidence expired, but whose credential is intact, answers with next action
  `retry_status` or a dedicated revalidate action as the contract names, never `repair_connection` (which asks for a
  new credential).
- Or: invoke revalidates transparently when only the evidence expired, if the contract allows it; the story decides
  which and records it in semantics.md.
- How long evidence stays valid for catalog providers is stated in the docs.
