---
title: GitLab
sidebar_position: 1
description: GitLab served from its pinned OpenAPI document through the catalog provider, with guarded merge-request writes and a merge-request feed.
---

# GitLab

Served from GitLab's own pinned OpenAPI document through the
[catalog provider](../../../concepts/catalog-provider.md), with no adapter code per endpoint. The
native GitLab adapter that source release v0.2.0 packaged was retired in v0.11.0, once every
operation it carried ran through the catalog with the same or stricter guarantees.

[Follow a request from your laptop to GitLab](/docs/examples/follow-a-request) to see federation,
the separate authentication boundaries and the issue response.

## What runs

The repository ships a reviewed selection set for GitLab. Each entry exposes one operation of the
pinned source under a local id with a declared effect:

| id | effect | what it binds |
|---|---|---|
| `project.get`, `issues.list`, `file.get`, `branch.get` | read | one GET each on the project, its issues, a repository file, a branch |
| `projects.list`, `project.events` | read | visible projects, one project's events |
| `tags.list`, `releases.list`, `commits.list`, `repository.compare` | read | repository tags, releases, commits for a ref, comparison of two refs |
| `deployments.list` | read | a project's deployments with their environment |
| `merge_requests.list`, `merge_request.get` | read | the project's merge requests, one merge request by IID |
| `pipelines.list`, `pipeline.get`, `pipeline.jobs`, `job.get` | read | pipelines, one pipeline, its jobs, one job |
| `job.trace` | read | a job's log, answered as text |
| `issue.create` | write | one POST opening an issue, unguarded |
| `merge_request.create` | write | one POST, guarded on the pinned source head |
| `merge_request.update` | write | one PUT, guarded on the pinned source head |
| `merge_request.merge` | write | one PUT, guarded on five checks |
| `merge_request.discussion.get` | read | one discussion of a merge request by id |
| `merge_request.note.create`, `merge_request.discussion.reply` | write | one POST adding a note or a discussion reply, unguarded |
| `merge_request.discussion.resolve` | write | one PUT resolving or unresolving a discussion, guarded on it being the requested, resolvable discussion and on the answered id and state |
| `feed.containers`, `feed.items` | read | the merge-request feed: member projects, then one project's merge requests changed since a watermark |

Input is one property per declared path or query parameter, named as GitLab names it, plus a
`body` object where the operation takes one. Output is the provider's status, its body unchanged,
and provenance naming the instance, the resource path and the pinned source's SHA-256. A paged
read returns one page as GitLab answers it, and every list read bounds `per_page` to 1 through 100.

## The merge-request feed

GitLab binds the shared [feed family](../../contracts/data/feed.md) as profile
`gitlab-merge-requests/1` ([GitLab feed profile](../catalog/contracts/feed.md)).
`feed.containers` lists the projects the token's user is a member of, oldest project id first,
continued after the last id of a full page. `feed.items` reads one project's merge requests in
every state, oldest change first, and resumes from the watermark the previous page returned. The
revision is the merge request's `updated_at`. Every project is listed `private`, because a public
project may keep its merge requests to its members; `url` is null, because a project rename
changes a merge request's link without changing its revision; deleted merge requests are not
observed. The binding is checked against recorded GitLab answers and a stand-in provider; it has
not been read from a running GitLab.

## Writes and the guard

A write is a required-approval mutation under private protocol two, with the host's approval,
audit and attempt controls ([the local runtime](../../../concepts/local-runtime.md)). The approval
policy must name each write before an approval is issued for it, and the approval binds the whole
input by digest. The guard is data in the selection: before the one request it reads the merge
request and refuses unless every check holds; after it, it compares the response and leaves the
outcome uncertain, never refused, when a check fails, because GitLab may already have applied the
write. No corrective request is ever issued.

The merge guard requires the merge request to be open and mergeable, at the pinned `body.sha`,
with the pinned `pipeline_id` as its head pipeline in status `success`; afterwards it requires
state `merged` at the pinned head. GitLab also checks `sha` atomically on the merge. Update has no
such precondition at GitLab, so a head that moves between the preflight read and the PUT leaves
the outcome uncertain.

`issue.create` carries no guard: before a create nothing exists to compare, so the one POST is
sent as approved. The same input approved and sent again opens a second issue. GitLab requires
`body.title`; every other body member, such as `assignee_ids` or `confidential`, is sent as
supplied, so read the whole input before approving.

`merge_request.note.create` and `merge_request.discussion.reply` are creates too, and unguarded
for the same reason: the same input approved and sent again adds a second note. Their bodies are
closed to the note text (`body`, plus `internal` for a merge-request note). GitLab runs quick
actions written in a note, such as `/close` or `/approve`, so read the text before approving.
The text `body` is required and must be a JSON string, and `internal` a JSON boolean; a body
without its text, or with any other key such as `created_at` or `merge_request_diff_head_sha`, is
refused before any request.
`merge_request.discussion.resolve` takes `resolved` as a JSON boolean only (`"true"` or `1` is
refused before any request), reads the discussion first and refuses unless it is the requested
discussion and resolvable, sends exactly `resolved`, and requires GitLab's answer to carry the
requested discussion id and state.
These four are checked against a fixture in the pinned document's shapes, not against a running
GitLab. A merge-when-pipeline-succeeds variant of the merge and a reopen variant of the update are
not selected: each needs a selection to fix a body member to a value, which the engine cannot
declare.

## Against a live GitLab

The 2026-10-02 replay returned HTTP 200 for all eighteen current reads and reused saved
credentials after an owner restart. Tags, releases and deployments returned empty lists;
`job.trace` returned text. One initial revalidation returned `unavailable` before a later
explicit retry succeeded, and that failure remains recorded and unexplained. This replay
performed no provider writes.

The earlier dedicated sandbox run answered the eleven reads then selected and the job trace as
text; refused a merge with a stale pinned head and a merge naming the wrong pipeline before any
request; merged one merge request with exactly one PUT; and refused an update of the merged
request on the state check. Earlier in that run, a create opened a merge request at its pinned
head, a duplicate was refused by GitLab's 409, and a create whose branch moved between the
preflight read and the POST was classified uncertain.

## Use saved credentials

The local CLI requires Linux x86_64 and the qualified GNOME Keyring Secret Service binding. The
operator builds the optimized CLI and catalog provider, writes an owner-only provider
configuration naming the bundle, the API base, the token header and the identity and scope reads,
and references the shipped selection set. The adapter entry in the CLI's TOML configuration names
`adapter_id = "catalog"`, the provider's configuration revision, the executable's SHA-256 and the
selection ids it may serve. `connections connect` then reads the token at a hidden prompt, runs
the identity and scope reads and refuses a token below the required scopes.

The repository's [catalog provider guide](https://github.com/beyond10x/connectors/blob/main/docs/local-catalog-provider.md)
has the complete configuration and commands, and the
[guarded merge guide](https://github.com/beyond10x/connectors/blob/main/docs/local-gitlab-merge.md)
the approval steps for a write. Validation evidence lasts 60 seconds; `connections revalidate`
renews it without re-entry, `connections repair` replaces an invalid credential and refuses a
changed identity, and `connections revoke` is local revocation that does not revoke the token at
GitLab.

## Limits

- The bundle carries parameters, media types and response statuses, not request or response
  schemas: a `body` is passed through and validated only by GitLab.
- Operations that need a header or cookie parameter, a multipart or form body, or that answer
  binary content are not served; a selection naming one is refused at load or fails at request
  time with a safe error.
- One token-header authentication profile. OAuth is not offered.
- A guard compares scalars for equality; it cannot express ranges or absence.
