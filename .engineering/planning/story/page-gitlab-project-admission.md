---
format: aep.planning-md/1
id: story:page-gitlab-project-admission
kind: story
status: implemented
title: Page GitLab project admission
summary: Keep every listed GitLab repository admissible across project-bound datasource checks.
relations:
- derived_from: epic:enforced-authority
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/integration-gitlab/Cargo.toml
- confidence: cited
  path: crates/integration-gitlab/src/backend.rs
- confidence: cited
  path: crates/integration-gitlab/src/backend_tests.rs
- confidence: cited
  path: crates/integration-gitlab/src/project_scan.rs
revision: 8
---
## Outcome

Every GitLab repository returned by the paginated project datasource remains admissible for project-bound reads, even when it is not among the first 100 recently active memberships.

## Context

`gitlab.projects` pages through the provider collection, but project-bound binding discovery and binding redemption each inspect only the first provider page. Workspace can therefore list a repository and then receive `NotGranted` while opening it.

## Acceptance

- A deterministic integration test places the selected repository after the first 100 projects and proves branch binding discovery succeeds.
- Redeeming that binding walks bounded provider pages and resolves the same project without widening the user's GitLab authority.
- Pagination is bounded, malformed continuation headers fail closed, and default-branch selection remains provider-declared.
- The released Connectors runtime restores the authenticated Devcenter project-open journey.

## Scope

- `crates/integration-gitlab/src/backend.rs`
- `crates/integration-gitlab/src/backend_tests.rs`
- Connectors release metadata and Devcenter deployment evidence.
