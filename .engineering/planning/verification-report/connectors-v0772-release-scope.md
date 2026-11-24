---
format: aep.planning-md/3
id: verification-report:connectors-v0772-release-scope
kind: verification-report
status: draft
title: Scope of connectors 0.77.2
revision: 1
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
