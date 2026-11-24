---
format: aep.planning-md/3
id: verification-report:connectors-v0780-release-scope
kind: verification-report
status: draft
title: 'connectors 0.78.0: what the release holds'
revision: 3
---
## Scope of connectors 0.78.0

Cut on 2026-11-24 under Timo's release order of 2026-11-23 03:59 CET (relayed by the host
operator): gaps of the runbook consumer go first. Its release review posts to Slack and
updates Jira tickets under one long-lived grant per operation (the operator's pick of
2026-11-23: exact-value grants in 0.77.0, a pattern grant and a value set in 0.78.0).

- story:standing-grant-binds-a-pattern-or-a-value-set: a target member of a standing grant
  is an exact value, a `pattern` (a regular expression of the Rust `regex` crate that matches
  the whole string, written with `^` and `$`) or `one_of` (a list of values). A file holds at
  most 64 pattern members. Each pattern gets a bound `max_value_bytes` from one work bound
  (22,055 bytes for `^DEV-[0-9]+$`); a longer value is covered by no grant.
  `approvals grant-standing` refuses a pattern whose bound is under 64 bytes, and
  `approvals list-standing` shows `max_value_bytes` on a pattern member.
- Minor version: the file format `connectors.standing-grants/1` gains two member forms. A
  binary older than 0.78.0 does not read a file that holds one: the served host answers
  every write `unavailable`, the local `invoke` answers `approval_required` and the
  `approvals` commands `invalid_configuration`. A file written by 0.77.0 verifies under
  0.78.0 and is reproduced byte for byte.
- Generated files: the CLI contract fixture, the operation types, the Entity Runtime
  definitions, the metadata emissions and the website reference pages were written by their
  generators with ess 0.97.4; the seven `--check` forms exit 0.
- The release workflow: the job that uploads the archives to the release page takes the
  archive artifacts only (`pattern` on its download step). In the run of 0.77.1 it took the
  two build records of the image build too and stopped at its count of four files, so 0.77.1
  has no archives. Two tests in `crates/connectors-build/tests/release_artifact_selection.rs`.
- This release holds everything of 0.77.1 and 0.77.2. Its archives are the first that hold
  `connectors-sql`.

## What a consumer reads

- `approvals list-standing`: a pattern member of `target` holds `max_value_bytes`; an exact
  member and a value-set member hold no such key.
- A grant with `{"pointer": "/issueIdOrKey", "pattern": "^DEV-[0-9]+$"}` and
  `{"pointer": "/target_status", "one_of": [...]}` covers every ticket of one project and a
  closed set of status ids under one `max_per_hour` and one `not_after_unix_ms`.
- Upgrade every reader of a grants file before a file with a new member form is mounted or
  signed.

## Known at the release

- The figures of the contract are measured, not proven ceilings: one read of the costliest
  file found holds 3,950,582 bytes, the peak of one read and one checked write is 4,393,287
  bytes (the text states "no more than 5 MiB at any moment"), and one read of 64 members of
  the costliest pattern to compile takes about 360 ms of CPU in the test profile.
- A counted repetition of a Unicode class meets the compiled-size limit early: `^\w{2}$` and
  `^\pL{2}$` are refused; `\d` is taken up to 12 repetitions, `[A-Za-z0-9_]` up to 680.
- The fixture `crates/connectors-host/tests/fixtures/standing_grants_0_77_0.json` holds a
  signature made with a test seed. The gates' `secrets` rule flags its member `signed`; the
  policy holds an exception for this path, line and digest (the operator's pick of
  2026-11-24).
- No standing grant, approval key or policy is created on any install by this release.
- The forge's test step ran 842.2 s in pull request 292 against the folder budget of 120 s.
  This release adds one test binary of 69 tests to it.
  story:the-gates-test-step-ends-within-120-s-on-the-forge and
  test-plan:where-the-gates-test-step-spends-its-time hold the measurements and twelve
  stories; the stories reach an estimated 382 s, not 120 s.
- The new `pattern` of the upload job is first run by the release-binaries run of this
  release's tag: on a pull request the workflow skips the upload job. The image job of the
  same workflow uses a `pattern` with `merge-multiple` on its download step.

## Not in this release

- An approval signer an agent process of the operator's user cannot use
  (story:a-marked-instance-takes-only-approvals-a-human-signed, `draft`; the pick is the
  operator's).
