---
title: Jira
sidebar_position: 7
description: Read Jira Cloud issues, comments, changelogs, transitions and users, and run one transition under approval.
lede: One Jira instance per Atlassian site reads issues by JQL and runs a transition only from the status you read to the status you approved.
source: docs/catalog-jira.md, adapters/catalog/providers/jira/connector.json, adapters/catalog/providers/jira/operations.json, adapters/catalog/tests/jira.rs, adapters/catalog/tests/jira_transition_run.rs
connectors: "jira"
---

# Jira

The `jira` connector of the provider Atlassian is served by the catalog provider from the pinned
Jira Cloud platform REST v3 document. The words connector, instance, connection and operation are
those of the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Jira guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-jira.md) has
every parameter, paging rule and refusal.

Commands use `connectors` on your `PATH` with your own configuration. Outputs were produced in
the repository's tree against a scratch configuration, whose `--config` and `--state-dir` are left
out here. A command that needs an Atlassian account is shown without output.

## What it reads and writes

| Reads | Writes |
|---|---|
| `issues.search`, `issue.get`, `issue.comments`, `issue.changelog`, `issue.transitions`, `issue.create_meta`, `users.search`, `attachment.content` | `issue.transition.run`, guarded: it reads the issue and the transition first and sends nothing unless the issue is in the status you named and the transition leads to the status you named; it reads the issue again afterwards |

## Prerequisites at the provider

Create an Atlassian API token for the account the reads and the transition should run as. Jira
and Confluence share one Atlassian credential and the profile `atlassian.basic`: the account's
email and the token, sent as HTTP basic. Atlassian API tokens expose no scope list, so
connecting checks the identity, not scopes.

A service-account token or an OAuth access token works only through the Atlassian API gateway,
which a derived instance cannot express; the
[full guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-jira.md#through-the-api-gateway)
shows that pinned form and why.

## Enable the connector and add an instance

The connector has no default endpoint: the instance names its site's REST v3 root.

```toml
[connectors.jira]
enabled = true

[connectors.jira.instances.work]
endpoint = "https://your-domain.atlassian.net/rest/api/3"
```

```sh
connectors instances list
```

```json
{"adapters":[{"adapter":"jira.work","adapter_id":"catalog","connector":"jira","instance":"work","instance_id":"jira.work","restart":"never","startup":"on-demand"}],"source":"configuration"}
```

## Connect

Put `{"account": "<email>", "token": "<API token>"}` in an owner-only file, then:

```sh
connectors connections connect --connector jira --instance work \
  --profile atlassian.basic --credential-file ~/private/atlassian.json
```

The same file connects the [Confluence](./confluence.md) instance as the same Atlassian account.

## A first read

```sh
connection=$(connectors --output json connections list --connector jira --instance work \
  | jq -r '.result.connections[0].connection')
schema=$(connectors --output json operations describe --connector jira --instance work \
  --operation issues.search | jq -r .result.schema)
revision=$(connectors --output json operations describe --connector jira --instance work \
  --operation issues.search | jq -r .result.revision)
connectors operations invoke --connector jira --instance work --connection "$connection" \
  --operation issues.search --schema "$schema" --revision "$revision" \
  --input-json '{"jql":"project = FIX AND updated >= \"2026-09-01 00:00\" ORDER BY updated ASC","maxResults":50}'
```

Follow `nextPageToken` until a page has none. Validation lasts 60 seconds after connect; later,
`connections revalidate` renews it.

## A first guarded write

Once per instance, create the signing key (`connectors approvals key-init --connector jira
--instance work`) and add an `[approval_clock]` table to the configuration
([the approval clock](https://github.com/beyond10x/connectors/blob/main/docs/local-clock.md)).
Publish a policy naming the write:

```sh
printf '{"operations":["issue.transition.run"]}\n' > policy.json
connectors approvals policy-set --connector jira --instance work --input-file policy.json
```

Read the issue's status id (`fields.status.id` of `issue.get`) and the transition's id and target
(`issue.transitions`), then pin them in `transition.json`:

```json
{"issueIdOrKey":"FIX-1","current_status":"3","target_status":"10002","body":{"transition":{"id":"31"}}}
```

Take the schema and revision of `issue.transition.run` from `operations describe`, then prepare,
issue and invoke:

```sh
connectors --output json approvals prepare --connector jira --instance work \
  --connection "$connection" --operation issue.transition.run \
  --schema "$schema" --revision "$revision" --input-file transition.json
connectors approvals clock-check --connector jira --instance work
connectors approvals issue --connector jira --instance work \
  --connection "$connection" --operation issue.transition.run \
  --schema "$schema" --revision "$revision" --input-file transition.json \
  --approve-subject "$subject_sha256" --proof-output "$HOME/private/proof-fix-1.json"
connectors operations invoke --connector jira --instance work --connection "$connection" \
  --operation issue.transition.run --schema "$schema" --revision "$revision" \
  --input-file transition.json --approval-file "$HOME/private/proof-fix-1.json" \
  --idempotency-key fix-1-31
```

An issue in another status, or a transition not open on it, is refused with the POST unsent.
When the read afterwards finds another status, the outcome is `unknown` and nothing is sent again.

## Reconnect when the connection is Stale

A token Atlassian refuses (revoked, expired) makes the connection `stale`, and an invoke answers
`next_action: repair_connection`. Publish a new token for the same account on the same
connection:

```sh
connectors --output json connections list --connector jira --instance work \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
connectors connections reconnect --connector jira --instance work \
  --expected-revision "<revision from the list>" --credential-file ~/private/atlassian.json
```

## Limits

- Live, only the gateway form with a service-account token has run (`issues.search` returned
  issues on 2026-09-30); the site form, the other reads and the transition are checked against a
  local fixture in the pinned document's shapes.
- The provider does not walk pages or bound `maxResults`; a read answered `429` is
  `rate_limited` with `retry_after_seconds` when the retry does not fit.
- `attachment.content` reads at most 2 MiB.
- The OAuth gateway form is not verified live, and the provider does not refresh an OAuth token.
