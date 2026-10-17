---
title: GitLab
sidebar_position: 6
description: Read GitLab projects, merge requests, pipelines and repositories, and write merge requests, tags, releases, branches and pipelines under approval.
lede: One GitLab instance per host and account reads 30 operations and writes 21 under approval, the merge pinned to the head commit and the pipeline you read.
source: docs/local-catalog-provider.md, docs/local-gitlab-merge.md, docs/local-approvals.md, adapters/catalog/providers/gitlab/connector.json, adapters/catalog/providers/gitlab/operations.json
connectors: "gitlab"
---

# GitLab

The `gitlab` connector of the provider GitLab is served by the catalog provider from the pinned
GitLab API v4 OpenAPI document. The words connector, instance, connection and operation are those
of the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full GitLab guide](https://github.com/beyond10x/connectors/blob/main/docs/local-catalog-provider.md)
and the [guarded merge guide](https://github.com/beyond10x/connectors/blob/main/docs/local-gitlab-merge.md)
have every operation, guard and refusal.

Commands use `connectors` on your `PATH` with your own configuration. Outputs were produced in
the repository's tree against a scratch configuration, whose `--config` and `--state-dir` are left
out here. A command that needs a GitLab account is shown without output.

## What it reads and writes

| | Operations |
|---|---|
| Reads (30) | `projects.list`, `project.get`, `project.events`, `issues.list`, `file.get`, `repository.tree`, `branch.get`, `branches.list`, `tags.list`, `tag.get`, `releases.list`, `release.get`, `release.links`, `commits.list`, `commit.get`, `commit.diff`, `repository.compare`, `search.blobs`, `deployments.list`, `environments.list`, `pipelines.list`, `pipeline.get`, `pipeline.jobs`, `job.get`, `job.trace`, `merge_requests.list`, `merge_request.get`, `merge_request.diffs`, `merge_request.discussions`, `merge_request.discussion.get` |
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

```sh
connectors instances list
```

```json
{"adapters":[{"adapter":"gitlab.work","adapter_id":"catalog","connector":"gitlab","instance":"work","instance_id":"gitlab.work","restart":"never","startup":"on-demand"}],"source":"configuration"}
```

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
connection=$(connectors --output json connections list --connector gitlab --instance work \
  | jq -r '.result.connections[0].connection')
schema=$(connectors --output json operations describe --connector gitlab --instance work \
  --operation merge_request.get | jq -r .result.schema)
revision=$(connectors --output json operations describe --connector gitlab --instance work \
  --operation merge_request.get | jq -r .result.revision)
connectors operations invoke --connector gitlab --instance work --connection "$connection" \
  --operation merge_request.get --schema "$schema" --revision "$revision" \
  --input-json '{"id":"group/project","merge_request_iid":9}'
```

Validation lasts 60 seconds; after that a read is refused with `next_action:
revalidate_connection` until `connections revalidate` renews it, with no token entered.

## A first guarded write

Once per instance, create the signing key (`connectors approvals key-init --connector gitlab
--instance work`) and add an `[approval_clock]` table to the configuration
([the approval clock](https://github.com/beyond10x/connectors/blob/main/docs/local-clock.md)).
Publish a policy naming the writes you use:

```sh
printf '{"operations":["merge_request.merge","merge_request.update"]}\n' > policy.json
connectors approvals policy-set --connector gitlab --instance work --input-file policy.json
```

Pin the merge to the head commit and the successful pipeline the read above showed, in
`merge.json`:

```json
{"id":"group/project","merge_request_iid":17,"pipeline_id":123,"body":{"sha":"0123456789abcdef0123456789abcdef01234567"}}
```

Take the schema and revision of `merge_request.merge` from `operations describe`, then prepare,
issue and invoke:

```sh
connectors --output json approvals prepare --connector gitlab --instance work \
  --connection "$connection" --operation merge_request.merge \
  --schema "$schema" --revision "$revision" --input-file merge.json
connectors approvals clock-check --connector gitlab --instance work
connectors approvals issue --connector gitlab --instance work \
  --connection "$connection" --operation merge_request.merge \
  --schema "$schema" --revision "$revision" --input-file merge.json \
  --approve-subject "$subject_sha256" --proof-output "$HOME/private/proof-merge-17.json"
connectors operations invoke --connector gitlab --instance work --connection "$connection" \
  --operation merge_request.merge --schema "$schema" --revision "$revision" \
  --input-file merge.json --approval-file "$HOME/private/proof-merge-17.json" \
  --idempotency-key merge-17
```

`mutation.classification` in the answer is `not_attempted`, `refused`, `applied` or `unknown`. A
lost answer is `unknown` and never triggers another PUT; repeating the same `--idempotency-key`
observes the original attempt.

## Reconnect when the connection is Stale

A token GitLab refuses (revoked, expired) makes the connection `stale`; an invoke answers
`next_action: repair_connection`, and revalidation cannot help. Publish a new token on the same
connection:

```sh
connectors --output json connections list --connector gitlab --instance work \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
connectors connections reconnect --connector gitlab --instance work \
  --expected-revision "<revision from the list>" --credential-prompt
```

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
