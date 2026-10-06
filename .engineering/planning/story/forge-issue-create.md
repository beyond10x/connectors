---
format: aep.planning-md/3
id: story:forge-issue-create
kind: story
status: implemented
title: GitLab issue creation through the forge selection as an approved write
relations:
- decomposes: epic:connector-probe-20261006
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: README.md
- confidence: cited
  path: adapters/catalog/providers/gitlab/operations.json
- confidence: inferred
  path: adapters/catalog/tests/gitlab_issue_create.rs
- confidence: inferred
  path: adapters/catalog/tests/shipped.rs
- confidence: inferred
  path: docs/local-catalog-provider.md
- confidence: inferred
  path: website/docs/adapters/gitlab.mdx
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T10:51:11Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-06T10:51:11Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-10-06T10:51:13Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Source

beyond10x/connectors#81, open: the forge (GitLab) catalog selection admits no issue creation
operation, so a caller that reads issues cannot file one through the same connection. The issue
asks for exact input, effect and schema, connection admission and approval requirements, and
response evidence.

## Acceptance

- `adapters/catalog/providers/gitlab/operations.json` admits `issue.create`
  (`postApiV4ProjectsIdIssues`, `POST /api/v4/projects/{id}/issues`, already in the committed
  bundle) as `effect: write`. It carries no guard: a guard needs a GET preflight with checks, and a
  create has nothing pinned to check, as the shipped `files.create`, `presentations.create` and
  `events.insert` creates do. It is admitted only under an approval policy naming it, as every
  write is.
- A fixture test sends the declared request with an approval and refuses it without one;
  `adapters/catalog/tests/shipped.rs` pins the new id list.
- The GitLab guide (`website/docs/adapters/gitlab.mdx`) and `docs/local-catalog-provider.md` list
  the write; #81 is closed by the PR.

## Scope

Derived 2026-10-06 by `aep:story-scoper`. **Cited** = read from the story or the tree; **inferred** =
a reading that could be wrong.

- **Files:** `adapters/catalog/providers/gitlab/operations.json:53-79`, beside the three
  `merge_request.*` writes; `postApiV4ProjectsIdIssues` at `adapters/gitlab/upstream/openapi_v3.yaml:50752`
  — cited
- **Also likely:** `adapters/catalog/tests/shipped.rs:33-58` (asserts the exact GitLab id list and
  `declared.len() == 21`), a new `adapters/catalog/tests/gitlab_issue_create.rs` modelled on the
  approvals fixture in `adapters/catalog/tests/google_drive.rs:1409-1415`,
  `website/docs/adapters/gitlab.mdx:23-32`, `docs/local-catalog-provider.md:59-69`, `README.md:110`,
  `CHANGELOG.md` — inferred
- **Confidence:** high for the selection file; medium for tests and docs
- **Would collide with:** any unit touching the GitLab selection, `adapters/catalog/tests/shipped.rs`,
  the GitLab docs or `CHANGELOG.md`
- **Safety fact:** no regenerated bundle and no host code: the operation is in the committed bundle
  with path `id` and a JSON body, and approval keys on the write effect, not on a list of operation
  names — unproven
