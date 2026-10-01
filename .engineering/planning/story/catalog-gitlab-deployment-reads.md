---
format: aep.planning-md/3
id: story:catalog-gitlab-deployment-reads
kind: story
status: active
title: 'GitLab catalog: list a project''s deployments with their environment'
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:catalog-gitlab-commit-reads
scope:
- confidence: cited
  path: adapters/catalog/providers/gitlab/operations.json
- confidence: cited
  path: adapters/catalog/tests/gateway_prefix.rs
- confidence: inferred
  path: adapters/catalog/tests/gitlab_deployment_reads.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime.rs
- confidence: cited
  path: adapters/catalog/tests/shipped.rs
- confidence: cited
  path: docs/local-catalog-provider.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T18:10:57Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-01T18:10:57Z", actor: "human:timo", revision: 5}
---
## Source

2026-10-01, a consumer session (cross-session request): connectors 0.22.0's GitLab selection has no deployment read.
The job entity `pipeline.jobs` returns (`APIEntitiesCiJob`) has no environment field; the environment is only on
`APIEntitiesDeployment` (`environment.name`, with `deployable` the job). Without it a consumer guesses the
environment from the deploy job's name and misses deploy jobs not named after their environment.

## Outcome

`deployments.list` maps `GET /api/v4/projects/{id}/deployments` (`getApiV4ProjectsIdDeployments`,
`adapters/gitlab/upstream/openapi_v3.yaml:63259`, already in the committed bundle). The upstream body is returned
unchanged, as the other list operations do. Inputs as the pinned document declares them: `id` (path, as
`pipelines.list`), `page`, `per_page` (bounded 1–100 like the other GitLab lists), `order_by`, `sort`,
`updated_after`, `updated_before`, `finished_after`, `finished_before`, `environment`, `status`. `read_api` scope.

## Acceptance

- `operations list --adapter <gitlab>` lists `deployments.list`; `operations describe` gives the inputs above with
  their pinned types.
- A fixture invoke returns records carrying `environment.name` and `deployable.id`, and pages by `page`/`per_page`.
- A project the token cannot read answers the same refusal code the other GitLab list operations answer.
- The GitLab guide table lists the operation.

Precedent: story:catalog-gitlab-commit-reads (operations.json, shipped.rs, local_runtime.rs, a story test file,
the guide table, and the bootstrap digest pin in `adapters/catalog/tests/gateway_prefix.rs`).
