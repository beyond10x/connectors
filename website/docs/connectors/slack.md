---
title: Slack
sidebar_position: 11
description: Read Slack conversations, users and files with a bot token, post messages and delete files under approval, and search with a user token, as a derived or pinned bot instance and a pinned user instance.
lede: Slack runs as two instances of the slack connector, a bot instance that reads and posts under approval, derived from the shipped declaration or pinned, and a pinned user instance that reads and searches.
source: docs/catalog-slack.md, adapters/catalog/providers/slack/connector.json, adapters/catalog/providers/slack/operations.json, adapters/catalog/providers/slack/user-operations.json, adapters/catalog/tests/slack.rs, docs/local-catalog-provider.md
connectors: "slack"
---

# Slack

The `slack` connector of the provider Slack is served by the catalog provider from the pinned
Slack Web API document. The words connector, instance, connection and operation are those of the
[README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Slack guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-slack.md) has
every parameter, scope and refusal.

Since 0.63.1 the catalog provider ships a connector declaration for the bot selection (profile
`slack.bot`), so a bot instance can be derived from the connector name. A provider directory
holds one declaration, so the user selection has none, and the user instance pins its
executable, that executable's SHA-256 and its configuration revision. The
outputs below come from the guide's run against a local HTTPS fixture standing in for Slack
(synthetic token and ids); its commands are shown without `--config`, `--state-dir` and
`--output json`.

## What it reads and writes

| Token | Reads | Writes |
|---|---|---|
| bot (`slack.bot`) | `conversations.list`, `conversations.history`, `conversations.replies`, `users.list`, `users.info`, `users.lookupByEmail`, `team.info`, `emoji.list`, `auth.test`, `files.list`, `files.info`, `file.download` | `chat.postMessage`, `conversations.open`, `chat.update`, `chat.delete`, `files.delete`, each guarded: it reads `auth.test` first (`files.delete` also reads the file with `files.info`) and sends nothing unless that answers `"ok": true`, and the answer must carry `"ok": true` too |
| search (`slack.user`) | `search.messages`, `conversations.list`, `conversations.history`, `conversations.replies`, `users.list` | none |

A bot reads only the conversations it is a member of; a user token reads what that person can
read, and only a user token can search.

## Prerequisites at the provider

Install a Slack app in the workspace and keep the bot token (`xoxb-`) it issues. Grant the bot the
scopes of what you use: `channels:read`, `groups:read`, `im:read`, `mpim:read` for
`conversations.list`; the matching `:history` scopes for the message reads; `users:read`,
`users:read.email` for `users.lookupByEmail`, `team:read`, `emoji:read`, `files:read`; and `chat:write`, `im:write` and `files:write` for the writes. For
search, keep a user token (`xoxp-`) with `search:read` and the user read scopes. Slack returns
granted scopes only in a header the provider does not read, so a missing scope shows as
`missing_scope` on the read that needs it.

## Enable the connector and add an instance

A bot instance can be derived from the shipped declaration: its default endpoint is
`https://slack.com/api` and its one profile `slack.bot`.

```toml
[connectors.slack]
enabled = true

[connectors.slack.instances.work]
```

```console
$ connectors instances list | jq -c '[.. | objects | select(.instance_id) | {connector, instance, instance_id}]'
[{"connector":"slack","instance":"work","instance_id":"slack.work"}]
```

The user instance has no declaration, so it is pinned. The guide's run, which the rest of this
page shows, pins both instances, `bot` and `search`. Write each instance's native configuration
file, print its bootstrap and hash the provider, as the
[full guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-slack.md#configure-the-two-instances)
shows, then pin both instances. The bot instance permits the writes, so it selects private
protocol two:

```toml
[connectors.slack]
enabled = true

[connectors.slack.instances.bot]
alias = "slack"
instance_id = "slack"
adapter_id = "catalog"
configuration_revision = "REPLACE_FROM_BOT_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/2"
startup = "on-demand"
restart = "never"

[connectors.slack.instances.bot.permissions]
profiles = ["slack.bot"]
operations = ["conversations.list", "conversations.history", "conversations.replies", "users.list", "users.info", "users.lookupByEmail", "team.info", "emoji.list", "auth.test", "files.list", "files.info", "file.download", "chat.postMessage", "conversations.open", "chat.update", "chat.delete", "files.delete"]

[connectors.slack.instances.bot.executable]
path = "/absolute/path/connectors-catalog-provider"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/slack-bot.json"]
```

The `search` instance is the same table with `alias` and `instance_id` `slack-search`, protocol
`connectors-private/1`, profile `slack.user`, the five user reads and `slack-user.json`. The
connector name `slack` of a pinned instance is a name only; it is not checked against a
declaration.

```console
$ connectors instances list | jq -c '[.. | objects | select(.instance_id) | {connector, instance, instance_id}]'
[{"connector":"slack","instance":"bot","instance_id":"slack"},{"connector":"slack","instance":"search","instance_id":"slack-search"}]
```

## Connect

Connect each instance with its own token at the hidden prompt:

```sh
connectors connections connect --connector slack --instance bot --profile slack.bot --credential-prompt
connectors connections connect --connector slack --instance search --profile slack.user --credential-prompt
```

Connecting runs `auth.test`; its `user_id` is the connection's identity.

## A first read

The instance's one connection and its current schema and revision are used when none is named,
and the input is read from standard input. In the guide's run:

```console
$ printf '{}' | connectors invoke slack/bot conversations.list | jq -c '.result.result|{status,body}'
{"status":200,"body":{"channels":[{"id":"C0000000001","name":"general"}],"ok":true,"response_metadata":{"next_cursor":""}}}
```

An empty `response_metadata.next_cursor` ends a walk. With two Slack instances,
`connectors invoke slack conversations.list` names none and is refused as `instance_ambiguous`,
listing `slack/bot` and `slack/search`. `connectors invoke slack/bot conversations.list --describe`
prints the operation's input schema, schema and revision without calling Slack.

## A first guarded write

The bot instance needs a signing key, an `[approval_clock]` table
([the approval clock](https://github.com/beyond10x/connectors/blob/main/docs/local-clock.md)) and
a policy naming the writes. `~/private/slack-policy.json` holds
`{"operations":["chat.postMessage","conversations.open"]}`:

```console
$ connectors approvals key-init --connector slack --instance bot | jq -c '.result|{connector,instance,keys: [.issuer.keys[]|{key_id,state}]}'
{"connector":"slack","instance":"bot","keys":[{"key_id":"f0bfff44-1cde-46f3-8d2d-fca4522ef5d1","state":"active"}]}
$ connectors approvals policy-set --connector slack --instance bot --input-file ~/private/slack-policy.json | jq -c '.result.policy|{instance_id,operations,revision}'
{"instance_id":"slack-bot","operations":["chat.postMessage","conversations.open"],"revision":1}
```

Prepare the approval for `~/private/post.json`, which holds
`{"body":{"channel":"C0000000001","text":"migration check"}}`, check the clock, issue the proof
for the subject's digest and invoke with it. The approval names the connection, and the schema
and revision `connectors invoke slack/bot chat.postMessage --describe` prints:

```console
$ connectors approvals prepare --connector slack --instance bot --connection conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b --operation chat.postMessage --schema fd40561da56fd3a5b2f1cd9fe47106914c45c80dc7cc27dc4bdca4f9a45efae2 --revision 87a55803d552e77cbfd7ed338bf86133549150a2f65b4d9812ca1328df1acce6 --input-file ~/private/post.json | jq -c '.result.preparation|{subject_sha256, presentation}'
{"subject_sha256":"5da23cd1ba119d43b499bb9689ab76e210e6dfbeec7dbda0ac65e025d299bf5d","presentation":{"connection":"conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b","descriptor_revision":"87a55803d552e77cbfd7ed338bf86133549150a2f65b4d9812ca1328df1acce6","instance":"slack-bot","operation":"chat.postMessage"}}
$ connectors approvals clock-check --connector slack --instance bot | jq -c '.result.observation.sources|map({address,result})'
[{"address":"51.81.57.55:2002","result":"agreed"},{"address":"148.113.255.71:2002","result":"agreed"}]
$ connectors approvals issue --connector slack --instance bot --connection conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b --operation chat.postMessage --schema fd40561da56fd3a5b2f1cd9fe47106914c45c80dc7cc27dc4bdca4f9a45efae2 --revision 87a55803d552e77cbfd7ed338bf86133549150a2f65b4d9812ca1328df1acce6 --input-file ~/private/post.json --approve-subject 5da23cd1ba119d43b499bb9689ab76e210e6dfbeec7dbda0ac65e025d299bf5d --proof-output ~/private/proof-post-1.json | jq -c '.result|{connector,instance,disposition,subject_sha256}'
{"connector":"slack","instance":"bot","disposition":"published","subject_sha256":"5da23cd1ba119d43b499bb9689ab76e210e6dfbeec7dbda0ac65e025d299bf5d"}
$ connectors invoke slack/bot chat.postMessage --approval-file ~/private/proof-post-1.json --idempotency-key post-1 < ~/private/post.json | jq -c '.result|{classification: .mutation.classification, status: .result.status, body: .result.body}'
{"classification":"applied","status":200,"body":{"channel":"C0000000001","message":{"text":"migration check","ts":"1780000700.000700"},"ok":true,"ts":"1780000700.000700"}}
```

Nothing about a new message exists to compare before it is sent, so the same input approved again
posts a second message.

`files.delete` goes the same way once the policy names it, with `{"body":{"file":"F0000000001"}}`
as its input: it reads `auth.test` and then the file with `files.info`, and deletes nothing when
the file is not found. Slack's `file_not_found` and `file_deleted` are refused as `not_found`,
`cant_delete_file` as `forbidden`
([Files in the full guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-slack.md#files)).

## Reconnect when the connection is Stale

Slack answers a revoked token with `"ok": false` and `invalid_auth`; the connection turns `stale`,
revalidation cannot help, and `connections reconnect` publishes a new token under the same
connection. In the guide's run the fixture stopped accepting the bot token, then accepted it
again:

```console
$ connectors invoke slack/bot conversations.list --input-json '{}'
{"error":{"code":"failure","data":{"code":"service_failure","kind":"operational","next_action":"repair_connection","service_code":"unauthorized","service_reason":"invalid_auth","stage":"dispatch"}},"ok":false}
$ connectors connections list --connector slack --instance bot | jq -c '[.result.connections[]|{connection,revision,state}]'
[{"connection":"conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b","revision":"691cd61d-0a34-4ba9-8ab2-f499f90c71f6","state":"stale"}]
$ printf '{"token":"xoxb-fixture-token"}' | connectors connections reconnect --connector slack --instance bot --expected-revision 691cd61d-0a34-4ba9-8ab2-f499f90c71f6 --credential-stdin | jq -c '.result.connection.summary|{connector,instance,connection,revision,state}'
{"connector":"slack","instance":"bot","connection":"conn_352bbc77ccfb465fbef6d1648d68811a_f6678f6563e14eeab20fcf1c1a75ce2b","revision":"691cd61d-0a34-4ba9-8ab2-f499f90c71f6","state":"ready"}
```

## Limits

- Verified against a local fixture only (`adapters/catalog/tests/slack.rs`); no live workspace
  has been read or posted to.
- The shipped declaration covers the bot selection only: a user-token instance stays pinned,
  and a derived instance refuses the profile `slack.user`. `setup migrate` moves both legacy
  Slack entries to pinned instances of `slack` under their aliases.
- Each list returns one page per call; search pages by number (`page`, `count` 1 to 100).
- `file.download` reads at most 2 MiB, and only on a connection whose hosts admit
  `https://files.slack.com`. File upload is not offered; `files.delete` is not read back after
  the write.
- A Slack error arrives as `200` with `"ok": false`.
