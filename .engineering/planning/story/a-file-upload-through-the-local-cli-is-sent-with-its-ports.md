---
format: aep.planning-md/3
id: story:a-file-upload-through-the-local-cli-is-sent-with-its-ports
kind: story
status: implemented
title: A file upload through the local CLI is sent with its slot and byte ports
summary: The local commit dropped an upload's ports, so every local file.upload was refused as forbidden; found on the first live use of 0.78.0
relations:
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T13:41:18Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T13:41:18Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T17:35:50Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Gap

`file.upload` of the Slack catalog shipped in 0.74.0 and is refused on every call through the
local CLI. Found on the first live use, 2026-11-24 14:20 CET, with connectors 0.78.0 on the
workstation's bot install: a file of 3,967 bytes, described, prepared and approved, was answered
`forbidden` at stage `dispatch`, classification `refused`, with no reason, and nothing was shared.
The same three requests sent by hand passed: the slot (`files.getUploadURLExternal`) answered
`ok`, and the byte POST to the issued URL answered HTTP 200.

## Cause

Read in the tree of 0.78.0:

- The local owner commits an approved write through the provider child, and the child's commit
  always calls `PreparedWrite::execute_created`
  (`crates/connectors-host/src/local/runtime/writes.rs`, `tokio_timeout`).
- The catalog's `execute_created` (`adapters/catalog/src/local.rs`, `impl runtime::PreparedWrite
  for Write`) drops the upload's slot and byte ports (`upload: _`) and calls
  `Prepared::execute_holding`.
- `execute_holding` reaches `dispatch_zeroizing` (`adapters/catalog/src/lib.rs`), which refuses a
  prepared upload: "an upload is sent only with its slot and byte ports".
- Only `execute` sends an upload (`Prepared::execute_upload`), and the local commit never calls
  it. The served host does, which is where every upload test ran
  (`adapters/catalog/tests/served.rs`).

`docs/catalog-slack.md` says of the upload: "Verified against the local fixture … not against a
live workspace". No test sent an upload through the provider child.

## Acceptance

1. A `file.upload` prepared and committed through the provider child (`Child::prepare_write`, then
   `commit`) on a connection that admits the upload host is `applied`: the fixture records the
   slot request with the file's name and length, the byte POST with the content's length and
   SHA-256 under its media type, and the completing write with the issued file id, the title, the
   channel, the thread and the comment. Three POSTs, in that order.
2. The test of 1 is committed red before the change: on the tree of 0.78.0 it ends `refused`
   with only `auth.test` sent.
3. A write that creates a credential is still held (`execute_created` keeps its custody path), and
   a prepared upload is still refused when it is sent without its ports.
4. On the workstation's bot install, with the released binary, one `file.upload` of a test file
   into the operator's test thread is `applied` and the file is read back in the thread.
   (Written as "a test image" until 2026-11-24; the one live upload was a text file, see
   "Verified".)

## Verified

- 1 and 2: `adapters/catalog/tests/slack.rs`,
  `an_approved_upload_through_the_child_sends_its_slot_its_bytes_and_the_share`; red on the tree
  of 0.78.0 in `0f0dd58ae`, green with `95fb4f974`.
- 3: `adapters/catalog/tests/upload_child_adversary.rs`,
  `a_prepared_upload_sent_without_its_ports_is_refused_with_nothing_sent`, and the
  `created_credential` tests. The adversary review is
  review-result:local-upload-adversary-round-1 (holds, five findings, no blocker).
- 4: 2026-11-24 18:31 CET, with the binaries of release 0.78.1 from its checked archive
  (`connectors 0.78.1`), through the workstation's bot install: one `file.upload` of a text
  file of 82 bytes with a title and a comment into the operator's test thread, answered `ok`
  with classification `applied` and a file id. Read back through the read install with
  `conversations.replies`: the thread holds the message with that file id, the file's name,
  title, size 82 and `text/plain`, and the comment as a `rich_text` block with bold, code and
  italic, so `initial_comment` takes mrkdwn. Not verified live: an image or other bytes that
  are not text; the fixture cases above compare the SHA-256 of such bytes.

## Scope

- `adapters/catalog/src/local.rs`: `execute_created` sends an upload with its ports.
- `adapters/catalog/tests/slack.rs`: the fixture issues an upload slot, takes the bytes and
  shares the file; one test.
- `docs/catalog-slack.md`: the sentence on what the upload was verified against.

## Not in this story

- A refused local write carries no reason: the commit's result frame holds a code only, so the
  CLI printed `forbidden` without the engine's sentence. The cause above took a code read to
  find. That is a story of its own.
- The bound of an approved upload through the local CLI (an input of 262,144 bytes):
  story:an-approved-binary-write-through-the-local-cli-holds-the-operations-2-mib.
