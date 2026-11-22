---
title: GitLab
sidebar_position: 6
description: Read GitLab projects, merge requests, pipelines and repositories, and write merge requests, tags, releases, branches and pipelines under approval.
lede: One GitLab instance per host and account reads 32 operations and writes 21 under approval, the merge pinned to the head commit and the pipeline you read.
source: docs/local-catalog-provider.md, docs/local-gitlab-merge.md, docs/local-approvals.md, adapters/catalog/providers/gitlab/connector.json, adapters/catalog/providers/gitlab/operations.json
connectors: "gitlab"
---

# GitLab

The `gitlab` connector of the provider GitLab is served by the catalog provider from the pinned
GitLab API v4 OpenAPI document. Terms follow the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full GitLab guide](https://github.com/beyond10x/connectors/blob/main/docs/local-catalog-provider.md)
and the [guarded merge guide](https://github.com/beyond10x/connectors/blob/main/docs/local-gitlab-merge.md)
have every operation, guard and refusal.

[How these commands were run](../guides/first-read.md#examples).

## What it reads and writes

| | Operations |
|---|---|
| Reads (32) | `projects.list`, `project.get`, `project.events`, `project.languages`, `issues.list`, `file.get`, `file.raw`, `repository.tree`, `branch.get`, `branches.list`, `tags.list`, `tag.get`, `releases.list`, `release.get`, `release.links`, `commits.list`, `commit.get`, `commit.diff`, `repository.compare`, `search.blobs`, `deployments.list`, `environments.list`, `pipelines.list`, `pipeline.get`, `pipeline.jobs`, `job.get`, `job.trace`, `merge_requests.list`, `merge_request.get`, `merge_request.diffs`, `merge_request.discussions`, `merge_request.discussion.get` |
| Writes (21) | `issue.create`, `merge_request.create`, `merge_request.update`, `merge_request.merge`, `merge_request.auto_merge`, `merge_request.reopen`, `merge_request.note.create`, `merge_request.discussion.reply`, `merge_request.discussion.resolve`, `tag.create`, `tag.delete`, `release.create`, `release.update`, `pipeline.retry`, `pipeline.cancel`, `pipeline.create`, `commit.create`, `file.update`, `branch.create`, `branch.delete`, `project.create` |

Most writes are guarded: the merge is refused before any request unless the merge request is
open, mergeable, at the pinned head commit and its pinned pipeline succeeded. Issue creation,
notes and replies are unguarded.

## Prerequisites at the provider

Create a personal access token on your GitLab host with the `api` scope; the connector declares
`api` as its minimum, and connecting reads the token's scopes and refuses one below it. The
token's user needs at least the Developer role for the writes, and the right to merge into a
protected target branch for a merge: GitLab answers a merge it forbids with `401`, so a permission
problem reads as an authentication one.

## Enable the connector and add an instance

The instance names the GitLab host; without `endpoint` it uses `https://gitlab.com`.

```toml
[connectors.gitlab]
enabled = true

[connectors.gitlab.instances.work]
endpoint = "https://gitlab.example.com"
```

`endpoint` is the host, `https://<host>`, a trailing `/` allowed, or `https://<host>/<path>` for
a GitLab served under a relative root. The connector adds `/api/v4` itself, so an endpoint that
already ends in `/api/v4` is refused as `invalid_input` naming `endpoint`.
`connections connect gitlab/work --profile gitlab.pat --endpoint https://gitlab.example.com`
writes the same instance before it asks for the token.

The instance is derived by the `connectors-catalog-provider` in the directory of `connectors`;
keep the two binaries together. `connectors setup check` lists it as the `installed_provider`
prerequisite, `failed` with the reason when it does not run. A connect is then refused as
`unavailable` at `installed_provider`, naming the provider's path and the reason, instead of
blaming the endpoint, and so is a connect whose provider prints its bootstrap but is killed or
exits when the host starts it for the instance. The macOS binaries are not signed: on an archive
downloaded through a browser, remove the quarantine attribute from the extracted directory first, with
`xattr -dr com.apple.quarantine <dir>`.

A second host or account is a second instance. A GitLab behind a private CA needs a `ca_file`,
which a derived instance cannot state: use the pinned form of the full guide.

## Connect

Enter the token at the hidden prompt:

```sh
connectors connections connect --connector gitlab --instance work \
  --profile gitlab.pat --credential-prompt
```

For automation, `--credential-file` with an owner-only `{"token":"..."}` document, or
`--credential-stdin`, carries it instead.

## A first read

```sh
connectors invoke gitlab/work merge_request.get \
  --input-json '{"id":"group/project","merge_request_iid":9}'
```

A read after the 60-second validation is refused with `next_action: revalidate_connection`; see [First read](../guides/first-read.md#validation).

## A first guarded write

Set up the [signing key and clock](../guides/guarded-write.md#once-per-instance), then publish this policy with `approvals policy-set`:

```sh
printf '{"operations":["merge_request.merge","merge_request.update"]}\n' > policy.json
```

Pin the merge to the head commit and the successful pipeline the read above showed, in
`merge.json`:

```json
{"id":"group/project","merge_request_iid":17,"pipeline_id":123,"body":{"sha":"0123456789abcdef0123456789abcdef01234567"}}
```

[Prepare, issue and invoke](../guides/guarded-write.md#prepare-issue-and-invoke) `merge_request.merge` on `gitlab/work` with `merge.json`, `--idempotency-key merge-17`.

A lost answer is `unknown` and never triggers another PUT; repeating the same `--idempotency-key`
observes the original attempt.

## Reconnect when the connection is Stale

A token GitLab refuses (revoked, expired) makes the connection `stale`; revalidation cannot help. Publish a new token on the same
connection. [Reconnect](../guides/reconnect-a-connection.md) `gitlab/work` with `--credential-prompt`.

A token of another GitLab user is refused as an identity mismatch.

## Limits

- Live GitLab evidence covers the reads, merge-request create and update and a guarded merge
  (recorded in the [full GitLab guide](https://github.com/beyond10x/connectors/blob/main/docs/local-catalog-provider.md#what-the-sandbox-showed));
  the newer writes are checked against fixtures in the pinned document's shapes.
- One page per call; the provider does not walk pages or follow GitLab's `Link` header.
- A binary answer is read whole, at most 2 MiB; uploads are not served.
- The update guard and the discussion resolve are not atomic: GitLab offers no precondition on
  their PUT.
- `connections revoke` ends local use and does not revoke the token at GitLab.
