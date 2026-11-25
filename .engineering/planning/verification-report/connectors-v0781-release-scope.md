---
format: aep.planning-md/3
id: verification-report:connectors-v0781-release-scope
kind: verification-report
status: accepted
title: Scope of connectors 0.78.1
summary: What 0.78.1 holds, what a consumer reads and what is known open at the release
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "accepted", at: "2026-11-24T15:40:53Z", actor: "human:timo", revision: 2}
---
## Scope of connectors 0.78.1

Cut on 2026-11-24 under Timo's release order of 2026-11-23 03:59 CET (relayed by the host
operator): finished green work is integrated and released when a user sees something. The user
here is the workstation's bot install, whose first `file.upload` was refused.

- story:a-file-upload-through-the-local-cli-is-sent-with-its-ports: the provider child sends an
  upload with its slot and byte ports. Since 0.74.0 every `file.upload` through the local CLI was
  answered `forbidden` at stage `dispatch` with only `auth.test` sent. Text:
  `docs/catalog-slack.md`, section "Files".
- story:the-forges-gate-job-compiles-once-and-prints-its-time-per-step, its first measurement and
  one change: `connectors-build` starts cargo without the `CARGO_MANIFEST_*` and `CARGO_PKG_*`
  variables of `cargo run`, so `ring` and what depends on it are not compiled again at each
  change of cargo's caller. The story stays `active`: the printed time per step and the other
  measurements are open.
- Patch version: no command, configuration member, answer member or error code is added,
  removed or changed.

## What a consumer reads

- A `file.upload` that is prepared, approved and sent with `connectors invoke` is sent: three
  requests (slot, bytes, share), answered `applied` with `ok` and `files`.
- Its bound through the local CLI is unchanged: an approval input of at most 256 KiB counting
  the base64 text (measured: a file of 196,243 bytes passes, one of 196,475 bytes is refused as
  `invalid_input`).
- The served host is unchanged.

## Known at the release

- A write refused at commit answers a code without the engine's reason
  (story:a-refused-local-write-names-its-reason, `draft`).
- An approved upload through the local CLI does not hold the operation's 2 MiB
  (story:an-approved-binary-write-through-the-local-cli-holds-the-operations-2-mib, `draft`; the
  pick is the owner's).
- The forge's test step is over its budget of 120 s
  (story:the-gates-test-step-ends-within-120-s-on-the-forge). This release does not change the
  tests.
- The upload is verified against the fixture through the provider child, not yet against a live
  workspace; the live upload through the workstation's bot install follows the release and is
  recorded below.

## Verified after the release (2026-11-24)

- Pull request 294 merged 17:38 CET as `cff90705b`; tag `v0.78.1` on that commit; the release
  page was a pre-release until its four archives matched `SHA256SUMS`, then the newest release
  (18:30 CET). The container image of the tag lists `linux/amd64` and `linux/arm64`.
- One job of the pull request, "release-built providers start through the host on
  macos-latest", ended `cancelled` with no step: "The job was not acquired by Runner of type
  hosted even after multiple attempts". It was run again once and passed.
- The live upload: 18:31 CET, binaries of the release archive (`connectors 0.78.1`), the
  workstation's bot install, one `file.upload` of a text file of 82 bytes into the operator's
  test thread: `applied`, read back in the thread with its name, title and size, the comment
  parsed as mrkdwn. story:a-file-upload-through-the-local-cli-is-sent-with-its-ports holds the
  record. Not verified live: bytes that are not text.
- On the workstation only the bot install moved to 0.78.1; the main install and the other three
  side installs stay on 0.78.0, for which 0.78.1 changes nothing.
- The forge's gate job of the pull request: 51 min 18 s. The step of the ignored tests took 6 s
  (617 s on pull request 293, 752 s on 292) and `ring` was compiled 3 times (5 before). The
  gate's second build did not get shorter: 250 s (209 s on 293). The changelog entry of 0.78.1
  named those 209 s as a cost of `ring`; that was wrong, and the entry says so since the next
  change. story:the-forges-gate-job-compiles-once-and-prints-its-time-per-step holds the lines.
