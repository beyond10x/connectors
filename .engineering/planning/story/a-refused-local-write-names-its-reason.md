---
format: aep.planning-md/3
id: story:a-refused-local-write-names-its-reason
kind: story
status: draft
title: A write refused at commit through the local CLI names its reason (draft)
summary: A refused local write answers a code alone; the engine's sentence would have named the upload defect in the first answer
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## State

Draft. Not scheduled.

## Gap

A write the provider child refuses at commit reaches the caller as a code alone. On 2026-11-24 a
`file.upload` through the local CLI of 0.78.0 answered `forbidden` at stage `dispatch` with no
reason, while the engine had refused it with the sentence "an upload is sent only with its slot
and byte ports". The cause was found by reading the commit path
(story:a-file-upload-through-the-local-cli-is-sent-with-its-ports); the sentence would have
named it in the first answer.

Read in the tree of 0.78.0:

- The commit's result frame of the private protocol holds a code only
  (`crates/connectors-host/src/local/runtime/writes.rs`).
- `mutation::Failure` has no reason member
  (`crates/connectors-host/src/local/owner/mutation.rs`).
- The CLI adds `service_reason` to a refused read (`apps/connectors/src/local.rs`); the answer of
  a refused write has no such member (`apps/connectors/src/local/operations.rs`).

A provider's own refusal of a write (`not_in_channel`) keeps its reason today through the
declared refusals of the selection; the engine's own refusals do not.

## Question

Which reasons may leave the provider child with a refused write? A reason the engine wrote is
fixed text. A reason read from a provider's answer is bounded and filtered for reads already
(`connectors_core::reason`). The contract of the write answer
(`contracts/service/local-mutations.md`) names the members a refusal holds, so a new member is a
contract change.

## Acceptance (draft)

- A write the engine refuses at commit answers with the engine's reason beside its code, bounded
  as a read's `service_reason` is.
- No reason carries a byte of the input, of a credential or of a provider's answer body beyond
  what a read's reason may carry.
- Red first: the test of a prepared upload sent without its ports, read through the CLI, names
  the reason.
