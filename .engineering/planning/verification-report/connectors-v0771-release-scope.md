---
format: aep.planning-md/3
id: verification-report:connectors-v0771-release-scope
kind: verification-report
status: draft
title: Scope of connectors 0.77.1
revision: 1
---
## Scope of connectors 0.77.1

Cut on 2026-11-24 under Timo's release order of 2026-11-23 03:59 CET (relayed by the host
operator): gaps of the dev-operator consumer go first. Upstream ask A15 of that repository: a
released install reads `mysql/<env>` and `loki/<env>` without a build from source.

- story:released-install-reads-mysql-and-loki-per-environment: the four release archives and
  the container image hold `connectors-sql` (seven programs, six before); `connectors-sql
  --version` prints its version; the MySQL guide has the section "A managed MySQL replica" and
  the paging rules of `query.read`; the Loki guide names the route through Grafana for a
  consumer with no Loki token. Tests of the replica (`adapters/sql/tests/mysql_replica.rs`,
  ignored, live), of the wire statements (`mysql_protocol.rs`) and of a metric query through a
  Grafana proxy (`adapters/loki/tests/local_runtime.rs`).
- story:one-pinned-rust-toolchain-for-the-local-gate-and-the-forge: `rust-toolchain.toml` pins
  1.98.1; the 11 install steps of 7 workflow files install from the file; the release workflow
  refuses a tag whose commit has no such file.
- story:host-tests-that-count-replays-act-as-their-own-process: test code only. A test that
  counts fresh opens or complete replays acts under process ids no other test of the binary
  uses.
- The clock example names `time.txryan.com` at its address of 2026-11-24
  (`docs/local-clock.md`, `docs/catalog-slack.md`).
- Patch version: no command, configuration member, answer member or error code is added,
  removed or changed. `connectors-sql` gains `--version`.
- Generated files: the seven checks (`metadata-conformance emissions`, `metadata-entities`,
  `cli`, `service-metadata`, `mcp-bindings`, `connectors-docs generate`, `ess-boundary`) found
  every file current after the merge of 0.77.0; nothing was regenerated.

## What a consumer reads

- No member of an answer changes.
- The archive of each target holds one more file, `connectors-sql`.
- A build from source needs Rust 1.98.1; rustup installs it from `rust-toolchain.toml`.

## Known at the release

- No instance of kind `mysql` or `loki` is configured on any install by this release. An
  instance that reaches a production database or a production Loki is set up only after
  Timo's yes for that connection, through the host operator.
- `release-binaries.yml` no longer rebuilds the archives of 0.77.0 or an earlier tag: those
  commits have no `rust-toolchain.toml`.
- `both_selectors_reach_one_instance_and_only_the_alias_is_deprecated` took 38.0 s on the
  loaded build server on 2026-11-24 (bound: 60 s).
- Rust 1.88.0 is not installed on the workstation, and no workflow runs `gate --msrv`.

## Not in this release

- A grant that binds a target member to a pattern or a value set
  (story:standing-grant-binds-a-pattern-or-a-value-set, 0.78.0).
- The Windows-target lint in the local gate (story:the-local-gate-lints-the-windows-target,
  `proposed`).
- An approval signer an agent process of the operator's user cannot use
  (story:a-marked-instance-takes-only-approvals-a-human-signed, `draft`; the pick is the
  operator's).
- `connections list` and `describe` through a running owner (branch
  `story/connections-through-owner`).
