---
title: A guarded write
sidebar_position: 11
description: The signing key, clock, policy, prepare, issue and invoke steps every write under approval takes, for every provider.
lede: A write runs only with a proof issued for the digest of its whole input; prepare names the digest, issue signs it and invoke sends it once.
source: docs/local-approvals.md, docs/local-clock.md, docs/local-gitlab-merge.md, contracts/cli/v1alpha1/semantics.md
---

# A guarded write

Every write is a required-approval mutation. It runs with a proof issued for its exact input, or,
on an instance that names a grant file, [under a standing grant](#under-a-standing-grant) its
operator signed once. A provider page gives its policy line, its example
input and what its guard refuses. The steps are the same for every provider. The examples use
shell variables, so set them first:

```sh
connector=gitlab; instance=work; operation=merge_request.merge; input=merge.json
```

## Once per instance

Create the instance's signing key and add an `[approval_clock]` table to the configuration
([the approval clock](https://github.com/beyond10x/connectors/blob/main/docs/local-clock.md)):

```sh
connectors approvals key-init --connector "$connector" --instance "$instance"
```

Then publish a policy that lists the writes you use. The page gives its policy line, which goes in
`policy.json`:

```sh
connectors approvals policy-set --connector "$connector" --instance "$instance" --input-file policy.json
```

## Prepare, issue and invoke

Take the connection, and the schema and revision of the operation from `invoke --describe`.
Prepare the approval and read the subject it names: `$subject_sha256` is the `subject_sha256` of
the prepare answer. The approval binds the whole input by digest, so read the input before you
approve it. Then check the clock, issue the proof and invoke with it:

```sh
connection=$(connectors --output json connections list --connector "$connector" --instance "$instance" \
  | jq -r '.result.connections[0].connection')
schema=$(connectors --output json invoke "$connector/$instance" "$operation" --describe \
  | jq -r .result.schema)
revision=$(connectors --output json invoke "$connector/$instance" "$operation" --describe \
  | jq -r .result.revision)
subject_sha256=$(connectors --output json approvals prepare --connector "$connector" --instance "$instance" \
  --connection "$connection" --operation "$operation" \
  --schema "$schema" --revision "$revision" --input-file "$input" \
  | jq -r .result.preparation.subject_sha256)
connectors approvals clock-check --connector "$connector" --instance "$instance"
connectors approvals issue --connector "$connector" --instance "$instance" \
  --connection "$connection" --operation "$operation" \
  --schema "$schema" --revision "$revision" --input-file "$input" \
  --approve-subject "$subject_sha256" --proof-output "$HOME/private/proof.json"
connectors invoke "$connector/$instance" "$operation" \
  --input-file "$input" --approval-file "$HOME/private/proof.json" \
  --idempotency-key <a key of your own>
```

`--input-json '<input>'` takes the input in place of `--input-file` in all three commands.

`mutation.classification` in the answer is `not_attempted`, `refused`, `applied` or `unknown`. A
write whose answer is lost is `unknown` and is never sent again. Repeating the same
`--idempotency-key` observes the original attempt. A write is never sent again after a `429`
either.

## Under a standing grant

A standing grant admits every keyed write it covers, with no proof per write: one operation, the
exact value of each target member, the input members a caller may set beside them, an expiry and a
cap of writes per hour. The key, the clock and the policy above stay prerequisites. Name the
instance's grant file in its entry, by an absolute path:

```toml
[connectors.slack.instances.bot]
standing_grants = "/absolute/private/slack-bot.grants"
```

Write the grant to `grant.json`, here posts to one Slack channel, and sign it:

```json
{
  "operation": "chat.postMessage",
  "target": [{"pointer": "/body/channel", "value": "C0000000001"}],
  "input_keys": ["/body/text", "/body/thread_ts"],
  "not_after_unix_ms": 1798761600000,
  "max_per_hour": 20
}
```

```sh
connectors approvals grant-standing --connector "$connector" --instance "$instance" --input-file grant.json
connectors approvals list-standing --connector "$connector" --instance "$instance"
```

Then invoke with a key and no `--approval-file`:

```sh
connectors --output json invoke "$connector/$instance" "$operation" \
  --input-stdin --idempotency-key <a key of your own> < "$input"
```

`mutation.standing.grant_id` in the answer names the grant. A write the grant does not cover is
`approval_required`, with no provider request and no reason in the answer; a write over the cap is
`rate_limited` with `retry_after_seconds`. `approvals revoke-standing <grant id>` removes a grant.
One grant admits up to 10,000 writes an hour until it expires, so grant the narrowest target and
input that serve the caller
([standing grants](https://github.com/beyond10x/connectors/blob/main/docs/local-approvals.md#standing-grants)).
