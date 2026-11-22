---
format: aep.planning-md/3
id: story:github-definition-repository-activity-reads
kind: story
status: draft
title: The GitHub definition reads a user's repositories and stars, and a repository's branches, commits, pull requests and languages
summary: Six reads added to connectors/github.yaml under the existing token and app profiles
relations:
- serves: vision:independent-contract-adapters
revision: 2
---
## Outcome

The GitHub definition `connectors/github.yaml` in `beyond10x/connectors-definitions` (release
0.6.0: 14 reads, 2 guarded writes) gains six reads, so a caller can survey a user's repositories
and a repository's recent activity:

| id | upstream |
|---|---|
| `user.repos` | `GET /user/repos` |
| `user.starred` | `GET /user/starred` |
| `branches.list` | `GET /repos/{owner}/{repo}/branches` |
| `commits.list` | `GET /repos/{owner}/{repo}/commits` |
| `pull_requests.list` | `GET /repos/{owner}/{repo}/pulls` |
| `repo.languages` | `GET /repos/{owner}/{repo}/languages` |

The ids are proposals. They follow the existing naming of `repos.list`, `releases.list` and
`check_runs.list`.

## Acceptance

- The definition carries the six reads under the same profiles, `github.token` and `github.app`,
  each `effect: read`, each list bounded by `per_page` 1..100. Each description states its end
  condition: a page shorter than `per_page`.
- `commits.list` accepts `sha`, `since` and `until`. `pull_requests.list` accepts `state`, `base`
  and `head`. No other filter widens what a call reads.
- A served instance with `catalog.repositories` set runs `branches.list`, `commits.list`,
  `pull_requests.list` and `repo.languages` only for a listed repository. It leaves `user.repos`
  and `user.starred` out of its description, because they have no `owner` and `repo`, as
  `docs/running-services.md` states for every other such read.
- A recorded-fixture test per read asserts the exact request sent. A two-page walk stops at the
  documented end condition.
- The release that carries the change is pinned here, and the GitHub page lists the six reads.

## Notes

- `GET /user/repos` and `GET /user/starred` read the authenticated user. Under `github.app` (an
  installation token) GitHub answers them for the installation, not a person. The description
  says which profile gives which answer.
