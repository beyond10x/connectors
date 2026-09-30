---
format: aep.planning-md/3
id: story:catalog-gitlab-commit-reads
kind: story
status: draft
title: 'GitLab catalog: list a ref''s commits and compare two refs'
relations:
- informed_by: story:catalog-gitlab-repository-reads
revision: 1
---
# GitLab catalog: list a ref's commits and compare two refs (read-only)

## Ask (from a consumer, 2026-09-30)

A consumer links each merged merge request to the tag that shipped it. It does so today from
`merge_requests.list` (`merge_commit_sha`), `project.events` (branch pushes, `commit_from` →
`commit_to`) and `tags.list` (`commit.id`): a merge belongs to the first tag whose commit is a pushed
branch head at or after the merge's push. That works for about 85% of tags in the consumer's data.
The rest point at a commit that no push event names, typically a release commit that a CI job
creates and tags in one step. Placing those tags needs the commit graph, which no catalog operation
reads.

## Acceptance

- Two read-only operations on the GitLab catalog provider, same permission model as `tags.list`:
  - `commits.list`: `GET /projects/:id/repository/commits` with `ref_name`, `since`, `until`,
    `first_parent`, `per_page`, `page`; returns `id`, `parent_ids`, `created_at`, `committed_date`,
    `title`, `author_name`, `author_email` as the provider returns them.
  - `repository.compare`: `GET /projects/:id/repository/compare` with `from`, `to`, `straight`;
    returns `commits` (ids and `parent_ids`) and `compare_timeout`.
- Both appear in `operations describe` with their schemas and in the shipped selection file.
- A fixture test for each pins paging (`commits.list`) and a `compare_timeout: true` answer
  (`repository.compare`), which the consumer must be able to tell from an empty compare.
- Minimum scope `read_api`, as the other repository reads.
