---
format: aep.planning-md/3
id: verification-report:connectors-v0770-release-scope
kind: verification-report
status: draft
title: Scope of connectors 0.77.0
revision: 1
---
## Scope of connectors 0.77.0

Cut on 2026-11-24 under Timo's release order of 2026-11-23 03:59 CET and his order of 15:11 CET
(both relayed by host-operator): a write of an unattended run goes without a human approval per
write, inside a grant a human signed.

- story:local-invoke-admits-a-keyed-write-under-a-standing-grant: `connectors invoke
  <connector>/<instance> <operation> --idempotency-key <key>` with no `--approval-file` is
  admitted when a grant signed with the instance's approval key covers it. Three commands
  (`approvals grant-standing`, `approvals revoke-standing`, `approvals list-standing`), one
  optional configuration member `standing_grants`, the answer member `mutation.standing.grant_id`,
  the refusal `rate_limited` with `retry_after_seconds`. A grant names one instance, one
  operation, exact target values, an optional closed input, `not_after_unix_ms` and
  `max_per_hour`.
- story:standing-grant-closes-the-input-to-listed-members: the member `input_keys` of a grant,
  on the served host too.
- Two reviews before the cut, both `needs-change`, both answered on the branch:
  review-result:standing-grant-local-adversary-1 and review-result:standing-grant-local-security-1.
- Generated files regenerated once on the tree that holds 0.76.2, with ess 0.97.4.
- Minor version: three new commands, one new configuration member, one new answer member, one
  new error code. A binary older than this release refuses a configuration that carries
  `standing_grants`; an instance without the member is unchanged.

## What a consumer reads

- `result.mutation` gains the optional member `standing` (`mutation.standing.grant_id`).
- `error.data.code` gains the value `rate_limited`.
- `error.data.retry_after_seconds` is new and appears only with `rate_limited`.
- `result.result.status` and `error.data.mutation.classification` are unchanged.

## Known at the release

- The five journeys of `local_runtime::slack_standing` did not hold on the loaded workstation:
  at host load 53 to 63 four of five passed, at load 74 to 84 two of five; the failures are
  `outcome_unknown` and `not_granted`. A write through the owner took 11 to 22 s in those runs;
  the CLI's write deadline is 20 s. story:a-write-through-the-owner-takes-seconds-under-load
  carries it.
- `final_audit_recovery_preserves_every_live_business_result` is killed at 60 s on the loaded
  workstation (story:store-tests-end-within-60s-under-load).
- A process of the owner's UID that replaces or edits rows of
  `<state>/standing-grants.sqlite3` can free the hourly cap. The contract says so.
- The signed grant file carries no sequence: an older signed file put back at the path admits
  again until its grants' `not_after` or until the approval key is rotated.
- No grant exists on any install at the release. The consumer's grants wait for 0.78.0.

## Not in this release

- A grant that binds a target member to a pattern or a value set
  (story:standing-grant-binds-a-pattern-or-a-value-set, 0.78.0).
- `connections list` and `describe` through a running owner (branch
  `story/connections-through-owner`).
- `connectors-sql` as a released binary and the MySQL replica and Loki proxy tests
  (story:released-install-reads-mysql-and-loki-per-environment).
