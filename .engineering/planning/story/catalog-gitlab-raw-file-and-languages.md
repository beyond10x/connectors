---
format: aep.planning-md/3
id: story:catalog-gitlab-raw-file-and-languages
kind: story
status: draft
title: 'GitLab catalog: read a repository file as text and a project''s languages'
revision: 1
---
## Outcome
A GitLab connection through the catalog provider reads a repository file's text as text, and a
project's language breakdown, so an indexer can read documents without decoding base64 out of
JSON and can classify a project from one read. Requested by a downstream consumer that indexes
about 280 projects and 1,800-2,300 documents per run; until this ships it indexes project
metadata only.

## Operations

| id | endpoint (pinned `adapters/gitlab/upstream/openapi_v3.yaml`) | effect |
|---|---|---|
| `file.raw` | `GET /api/v4/projects/{id}/repository/files/{file_path}/raw` (`ref`, `lfs`), line 61418 | read |
| `project.languages` | `GET /api/v4/projects/{id}/languages`, line 36037 | read |

`file.get` (base64 `content` inside JSON) stays as it is.

## Open questions the specification answers first
- `file.raw` answers `application/octet-stream` (`type: string, format: binary`); the catalog
  engine sends and reads JSON bodies. The shared model states how a non-JSON body is returned:
  UTF-8 text as a string member when it decodes, and a named refusal (or a base64 member) when
  it does not, with the provider response bound (4 MiB) applied to the raw bytes.
- `project.languages` declares no `200` schema; the answer is an object of language name to
  percentage. A cited amendment (`openapi_v3.amendments.json`, as for earlier GitLab
  parameters) supplies the schema; the pinned bytes and digest stay unchanged.

## Spec first
Model the non-JSON text answer in the shared ESS (a generic member, no GitLab vocabulary) and
validate with the repository's pinned `ess` before any engine change.

## Acceptance
- Against the local GitLab fixture, `file.raw` on a UTF-8 file returns its exact text (bytes
  compared), with `ref` honoured; a non-UTF-8 file gets the modelled outcome by name; a file
  over the bound is refused `capacity`.
- `project.languages` returns the fixture's language map with its percentages.
- `docs/catalog-gitlab.md` (or the GitLab catalog page) shows both reads from a run in the tree.
