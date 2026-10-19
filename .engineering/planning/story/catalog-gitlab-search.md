---
format: aep.planning-md/3
id: story:catalog-gitlab-search
kind: story
status: draft
title: GitLab search over projects, merge requests and issues, each result keeping its project id
relations:
- depends_on: story:catalog-zendesk-ticket-writes
- depends_on: story:catalog-github-anonymous-reads
revision: 1
---
## Outcome

A consumer searches GitLab through the catalog provider: `search` (GET /api/v4/search) over the
scopes `projects`, `merge_requests` and `issues`, and project code search. Each result keeps its
project id, so a caller can follow it with the existing project, merge request and issue reads.

## Spec first

The new selection entries are declared in the catalog selection (adapters/catalog/providers/gitlab
/operations.json) against the pinned GitLab OpenAPI document, with their bounds, and checked by the
catalog selection model before any provider code changes.

## Acceptance

- `search` is a read of `getApiV4Search` with `scope` held to `projects`, `merge_requests` and
  `issues`, `search` required, `per_page` bounded (1..100) and one page per call; every other
  scope is refused by name at the selection.
- Project code search: `search.blobs` already ships (`getApiV4ProjectsIdDashSearch`, scope held to
  blobs). The consumer's `project.search` is served by it; this story adds no duplicate operation,
  and documents the name to call.
- Each result in every scope carries its project id (`project_id`, or `id` for a project), pinned
  in a fixture test per scope.
- A result-count bound: a call asks for at most the bound and the answer never exceeds it; tested
  with a fixture that returns more.
- docs/catalog-gitlab.md lists both operations with an example call run in the tree.
