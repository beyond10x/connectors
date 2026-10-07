---
format: aep.planning-md/3
id: task:gitlab-feed-sandbox-replay
kind: task
status: draft
title: Read the GitLab feed binding from the sandbox through the released CLI
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:gitlab-feed-binding
revision: 1
---
## Outcome

The released GitLab feed binding is read from the dedicated GitLab sandbox
(`connectors-gitlab-20260912`, `docs/evidence/gitlab-sandbox-20260913/`) through the released CLI
and a saved connection, the way `task:gitlab-catalog-current-sandbox-read-replay` read the
other GitLab operations on 2026-10-02 (`docs/evidence/gitlab-current-20261002/`): containers, a
first items page and a resumed page, with the protected credential read by the CLI from its
existing owner-only file and no secret in argv or a published log.

## Why separate

Acceptance item 6 of `story:gitlab-feed-binding`: on 2026-10-07 the sandbox was running but the
unit's session had no credential for it, so the binding shipped checked against recorded
provider shapes and the stand-in provider only. The replay needs private config and state,
custody daemons and the released binary, which is a procedure of its own.

## Acceptance

- A recorded transcript of the three reads under `docs/evidence/`, with response hashes, no
  token and no response body that is not the sandbox's own seed data.
- Whether GitLab answers 403 on the merge request list of a project the token can see is
  observed, not assumed (adversary finding F1 of wave 20261007a).
- `adapters/catalog/contracts/feed/v1alpha1/gitlab.md` states the binding as read from GitLab.
