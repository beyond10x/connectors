---
format: aep.planning-md/3
id: verification-report:connectors-v0772-release-scope
kind: verification-report
status: accepted
title: Scope of connectors 0.77.2
revision: 3
transitions:
- {from: "draft", to: "accepted", at: "2026-11-24T12:41:08Z", actor: "human:timo", revision: 3}
---
## Scope of connectors 0.77.2

Cut on 2026-11-24 under Timo's release order of 2026-11-23 03:59 CET (relayed by the host
operator): finished green work is integrated and released when a user sees something.

- story:an-approved-write-survives-one-refused-clock-acquisition: `approvals issue`, the three
  standing commands, a write through `invoke` and the recovery of a prepared write ask the
  clock sources a second time when the first attempt refused `below_quorum`, no source
  reported `unverified`, and the caller's deadline still holds one query deadline plus the
  reserve (1 s; the query deadline plus 1 s before the first of a write's two readings).
  `approvals clock-check` and the maintenance scan keep one attempt. Texts:
  `contracts/service/clock.md` (section "Second attempt"), `docs/local-clock.md`,
  `docs/design/approval-clock-quorum.md`.
- story:the-local-gate-lints-the-windows-target: `connectors-build gate` lints
  `connectors-host` and `connectors` for `x86_64-pc-windows-gnu` after the Linux lint and ends
  non-zero at its start when the Rust target or `x86_64-w64-mingw32-gcc` is missing. The
  forge's gate job installs both. Text: `docs/development.md`, section "The Windows lint".
- Patch version: no command, configuration member, answer member or error code is added,
  removed or changed. A refusal `unavailable` for a clock below its quorum arrives after up to
  4 s of network wait per acquisition (before: 2 s), and a write after up to 8 s (before: 4 s).

## What a consumer reads

- No member of an answer changes.
- A write that met one dropped clock exchange succeeds where 0.77.1 answered `unavailable`
  (`stage` `readiness` from `issue`, `not_attempted` at `stage` `admission` from `invoke`).
- A contributor who runs the local gate needs the Rust target `x86_64-pc-windows-gnu` on the
  pinned toolchain and the C compiler `x86_64-w64-mingw32-gcc`.

## Known at the release

- The forge's test step ran 621.8 s in pull request 291 against the folder budget of 120 s
  (story:the-gates-test-step-ends-within-120-s-on-the-forge, `draft`). This release does not
  change that time.
- The Windows lint does not link, does not cover the MSVC target and runs no test.

## Not in this release

- A grant that binds a target member to a pattern or a value set
  (story:standing-grant-binds-a-pattern-or-a-value-set, 0.78.0; its commit waits for the
  operator's pick on a signed test fixture).
- A clock refusal of `issue` and `invoke` that names its sources, and a write that acquires
  the clock once (two sibling drafts of the clock story).
- An approval signer an agent process of the operator's user cannot use
  (story:a-marked-instance-takes-only-approvals-a-human-signed, `draft`; the pick is the
  operator's).

## Verified after the release

- Pull request 292 merged 2026-11-24 11:08 CET as 7626141119f; tag `v0.77.2` is on that
  commit. Release page published 11:10 CET.
- The release page held no archive from 11:10 to 12:38 CET. In the release-binaries run
  45372260956 the build job for aarch64-apple-darwin started at 11:56 CET, 43 minutes after
  the provenance job ended; the upload job waits for all four builds and ran 12:37:58 to
  12:38:16 CET. While the page had no archive, the plugins repository's check of the newest
  CLI release answered 404.
- The run ended `success` in one attempt, all 7 jobs. The page holds four archives and
  `SHA256SUMS`; `sha256sum -c` passes on the four downloaded archives.
- The archive for x86_64-unknown-linux-gnu holds `connectors`, `connectors-catalog-provider`,
  `connectors-sql`, `connectors-loki`, `connectors-kubernetes`, `connectors-prometheus` and
  `connectors-grafana`; `connectors-sql --version` from it prints `connectors-sql 0.77.2`.
  This is the first released archive read to hold `connectors-sql`; 0.77.1 has none.
- The image `ghcr.io/beyond10x/connectors:0.77.2` (digest sha256:4e74c786c17d) is built for
  linux/amd64 and linux/arm64.
- From 0.78.0 on the release page is created as a pre-release and becomes the newest release
  only after its four archives pass `sha256sum -c`, so the newest release always has archives.
- No install was moved to 0.77.2; the workstation install goes from 0.77.0 to 0.78.0.
