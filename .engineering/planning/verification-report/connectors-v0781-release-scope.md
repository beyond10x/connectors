---
format: aep.planning-md/3
id: verification-report:connectors-v0781-release-scope
kind: verification-report
status: accepted
title: Scope of connectors 0.78.1
summary: What 0.78.1 holds, what a consumer reads and what is known open at the release
relations:
- serves: vision:independent-contract-adapters
revision: 2
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
