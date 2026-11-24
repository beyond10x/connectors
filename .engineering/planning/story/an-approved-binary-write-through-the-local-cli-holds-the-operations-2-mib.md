---
format: aep.planning-md/3
id: story:an-approved-binary-write-through-the-local-cli-holds-the-operations-2-mib
kind: story
status: draft
title: An approved binary write through the local CLI holds the operation's 2 MiB (draft, waits for the owner's pick)
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## State

Draft only. It changes what an approval binds, so the pick is Timo's (the operator, 2026-11-24
12:39 CET: "B: draft the story only. It changes what an approval binds, so it is Timo's pick
(security); I queue it for him, no implementation before his word."). No implementation and no
design merge before that word.

## Gap

`file.upload` of the Slack catalog takes a file of at most 2 MiB (`binary.max_bytes` 2,097,152 in
`adapters/catalog/providers/slack/operations.json`). Through the local CLI with a signed approval
per write the bound is lower, and `docs/catalog-slack.md` says so: "The local CLI keeps its limits,
an input of at most 1 MiB and an approval request of at most 272 KiB, both counting the base64
text, so an upload through it is bounded by them; the served host reads the whole 2 MiB."

- `crates/connectors-host/src/local/runtime.rs`: `INPUT_LIMIT` is 1,048,576.
- `crates/connectors-host/src/local/owner/approval_issuance.rs`: `REQUEST_LIMIT` is 278,528
  (272 KiB).
- Base64 adds one third: a request of 272 KiB holds a file of about 200 KiB.

Met on 2026-11-24: a screen recording of 0.96 MB for a team channel, ordered by the owner, could
not be uploaded by the workstation's bot install, which approves each write through the local CLI.
The operator re-encodes it under 200 KiB for this one upload.

## Question for the owner

What does an approval of a binary write bind?

- A (today): the whole input, with the content's base64 text. A file is bounded by the approval
  request limit.
- B: the input without the content's text, and the content's `sha256` and length, which the input
  already carries (`content.sha256`) and which the provider checks against the bytes before any
  request. The local CLI's input limit for an operation with `binary.max_bytes` follows that
  bound. The approval then names the file by its digest and no longer holds its bytes.
- C: raise `REQUEST_LIMIT` and `INPUT_LIMIT` to hold 2 MiB of base64 (about 2.8 MiB). The approval
  binds what it binds today; every prepare, issue and invoke reads and hashes that much.

Not decided here. B changes the approval subject; C changes two limits that bound every write.

## Acceptance (for B, if picked)

- A `file.upload` of a 2 MiB file through the local CLI is prepared, approved and applied; the
  approval subject holds the content's `sha256` and length and not its base64 text.
- An approved upload sent with other bytes of the same length is refused before any request, with
  nothing sent.
- A proof issued before the change verifies or is refused by name; which of the two is stated in
  `contracts/service/local-mutations.md`.
- Red first: the failing test of the 2 MiB upload is committed before the change.

## Limits

- No limit is raised and no subject is changed before the owner's pick.
- A served host is not part of this story; it reads 2 MiB today.
