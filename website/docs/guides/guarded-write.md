---
title: A guarded write
sidebar_position: 11
description: The signing key, clock, policy, prepare, issue and invoke steps every write under approval takes, for every provider.
lede: A write runs only with a proof issued for the digest of its whole input; prepare names the digest, issue signs it and invoke sends it once.
source: docs/local-approvals.md, docs/local-clock.md, docs/local-gitlab-merge.md, contracts/cli/v1alpha1/semantics.md
---

# A guarded write

Every write is a required-approval mutation. A provider page gives its policy line, its example
input and what its guard refuses; the steps are the same for every provider. The examples use
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

Then publish a policy naming the writes you use. The page gives its policy line, which goes in
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
write whose answer is lost is `unknown` and is never sent again; repeating the same
`--idempotency-key` observes the original attempt. A write is never sent again after a `429`
either.
