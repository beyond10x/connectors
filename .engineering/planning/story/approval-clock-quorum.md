---
format: aep.planning-md/3
id: story:approval-clock-quorum
kind: story
status: implemented
title: The approval clock answers from a quorum of Roughtime sources
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: adapters/catalog/tests/local_runtime/background_recovery.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime/guarded_merge.rs
- confidence: cited
  path: apps/connectors-cli-contract
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: cited
  path: apps/connectors/tests/local_cli.rs
- confidence: cited
  path: contracts/cli/v1alpha1/fixtures/values.json
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: contracts/service/clock.md
- confidence: cited
  path: contracts/service/delegation.md
- confidence: cited
  path: crates/connectors-build/src/ignored.rs
- confidence: cited
  path: crates/connectors-docs/src/status.rs
- confidence: cited
  path: crates/connectors-host/src/local/approvals/proof.rs
- confidence: cited
  path: crates/connectors-host/src/local/approvals/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/clock.rs
- confidence: cited
  path: crates/connectors-host/src/local/clock/protocol.rs
- confidence: inferred
  path: crates/connectors-host/src/local/clock/protocol_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/clock/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/config.rs
- confidence: cited
  path: crates/connectors-host/src/local/mutations.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/approval_issuance.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/approval_issuance/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/maintenance.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/mutation/execution.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/mutation/recovery.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/mutation/tests.rs
- confidence: cited
  path: crates/connectors-host/tests/fixtures/clock/README.md
- confidence: cited
  path: crates/connectors-host/tests/fixtures/clock/server.rs
- confidence: cited
  path: docs/catalog-slack.md
- confidence: cited
  path: docs/local-approval-binding.md
- confidence: cited
  path: docs/local-clock.md
- confidence: cited
  path: docs/local-mutation-ledger.md
- confidence: cited
  path: ess/domains/cli.yaml
- confidence: cited
  path: ess/domains/clock.yaml
- confidence: cited
  path: ess/domains/mutations.yaml
- confidence: cited
  path: website/docs/concepts/local-runtime.md
- confidence: inferred
  path: website/docs/reference/contracts/cli.md
- confidence: inferred
  path: website/docs/reference/contracts/clock.md
- confidence: inferred
  path: website/docs/reference/contracts/model/domains/connectors-cli.md
- confidence: cited
  path: website/docs/reference/contracts/model/domains/connectors-clock.md
- confidence: inferred
  path: website/docs/reference/contracts/model/domains/connectors-mutations.md
- confidence: inferred
  path: website/docs/status.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T18:07:38Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-14T18:07:38Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-10-16T02:05:07Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1}}}
---
# Story: the approval clock answers from a quorum of Roughtime sources

## Outcome

`approvals clock-check`, approval issuance and the mutation ledger take their bounded
UTC interval from two to eight configured Roughtime sources, combined by a quorum,
instead of one trusted source. One lying source among honest ones cannot move the
answer off true time; one unreachable source does not stop the clock while a quorum
remains. A refusal names its combined cause and every source's result.

## Decision

The approval clock moves from one Roughtime source to several with a quorum, and the
width cap moves from 4,000 ms to 8,000 ms. The single-source check is replaced, not
kept beside it. Combination rule: the answer is the hull of the instants that at
least `quorum` counted sources' intervals contain (`docs/design/approval-clock-quorum.md`,
"Quorum rule"). A `roughtime-clock/1` table is refused for every clock use with an
upgrade message and never read as a one-source quorum; it no longer fails the rest of
the configuration. Sources are queried in parallel under one shared 2,000 ms deadline,
no retries, one fixed wire version per source, no negotiation.

## Acceptance (named conformance scenarios)

Host, `crates/connectors-host/src/local/clock/tests.rs`:
- `quorum_answer_is_hull_of_quorum_coverage`
- `quorum_contains_truth_with_one_disjoint_liar`
- `quorum_overlapping_liar_cannot_exclude_truth`
- `quorum_below_quorum_names_each_source`
- `quorum_split_refuses`
- `quorum_combined_width_over_cap_refuses`
- `quorum_wide_source_counts_and_only_the_answer_is_capped`
- `quorum_width_cap_boundary`
- `quorum_shared_deadline`
- `quorum_version_mismatch_is_unverified`
- `quorum_configuration_refusals`
- `quorum_counted_set_is_fixed`
- `legacy_single_source_format_refuses_only_the_clock`

CLI, `apps/connectors/tests/local_cli.rs`:
- `clock_check_reports_each_source`
- `clock_check_split_refusal`
- `clock_check_below_quorum_refusal`
- `clock_check_legacy_format_refusal`

Width consistency:
- `proof_clock_admits_8000_ms` (`crates/connectors-host/src/local/approvals/tests.rs`)
- `ledger_clock_admits_8000_ms` (`crates/connectors-host/src/local/owner/mutation/tests.rs`)

Live, ignored and explicit: `independent_live_quorum` replaces `independent_live_source`.

The ESS model (`ess/domains/clock.yaml`) validates and compiles with the pinned ESS;
the regenerated CLI contract and documentation pages show no drift under the gate.

## Scope

Depends on `impl/clock-refusal-names-its-cause` (commit `0ec7ccd1a`) being on `main`:
this story extends its `clock_refusal` and replaces its `clock_radius_ms`.

Specification (first):
- `ess/domains/clock.yaml` (done in the design: `Configuration`, `Source`,
  `RoughtimeVersion`, `SourceResult`, `SourceReport`, `Refusal`, `Observation.sources`)
- `ess/domains/cli.yaml`: `Failure.clock_refusal` becomes
  `Optional<connectors.clock.Refusal>`, add `clock_sources:
  Optional<List<connectors.clock.SourceReport>>`, drop `clock_radius_ms` and
  `connectors.cli.ClockRefusal`
- `ess/domains/mutations.yaml`: the four-second comment on `SettlementClockInterval`

Generated (regenerate, never hand-edit):
- `apps/connectors-cli-contract/` (`ess generate cli`), `contracts/cli/v1alpha1/fixtures/values.json`
- `website/docs/reference/contracts/model/domains/connectors-clock.md` and the other
  `connectors-docs generate` pages

Rust, crate `connectors-host`:
- `src/local/clock.rs`: `Configuration`, `Source`, legacy shape, parallel acquisition,
  quorum combination, `MAX_WIDTH_MS = 8000`, per-source and combined causes
- `src/local/clock/protocol.rs`: version passed per source instead of the `VERSION` constant
- `src/local/clock/tests.rs`, `tests/fixtures/clock/server.rs` (several fixture servers, liar and wide-radius fixtures)
- `src/local/config.rs`: legacy table loads; clock refusal scoped to clock uses
- `src/local/approvals/proof.rs:94`, `src/local/mutations.rs:469`: width 8,000
- `src/local/owner/approval_issuance.rs`, `owner/maintenance.rs`,
  `owner/mutation/execution.rs`, `owner/mutation/recovery.rs`: legacy-format refusal
- tests constructing `clock::Configuration`: `owner/approval_issuance/tests.rs`,
  `approvals/tests.rs`, `owner/mutation/tests.rs`

Rust, crate `connectors` (CLI): `apps/connectors/src/local.rs` (`clock_failure`),
`apps/connectors/tests/local_cli.rs`.

Rust, crate `connectors-catalog` tests: `adapters/catalog/tests/local_runtime/guarded_merge.rs:519`,
`background_recovery.rs:94` (fixture configuration shape).

Prose: `contracts/service/clock.md`, `contracts/service/delegation.md:145` (eight
seconds, ±4 s), `contracts/cli/v1alpha1/semantics.md`, `docs/local-clock.md`,
`docs/local-approval-binding.md:56-57`, `docs/local-mutation-ledger.md:75-76`,
`docs/catalog-slack.md:497-498`, `crates/connectors-docs/src/status.rs:169`, `CHANGELOG.md`.

Out of scope: a second Roughtime wire version, server-list discovery, malfeasance
reporting, any change to the proof profile's `nbf`/`exp` offsets.

## Scope corrections (2026-10-14, scoped on wave 20261014c)

- `crates/connectors-build/src/ignored.rs` names `local::clock::tests::independent_live_source`;
  the rename to `independent_live_quorum` lands there too, or the ignored-test inventory fails.
- `website/docs/concepts/local-runtime.md` (hand-written) shows a `roughtime-clock/1`
  `[approval_clock]` table; it moves to the new shape.
- `src/local/clock/protocol_tests.rs` reads `protocol::VERSION`; it follows the per-source version.
- Only `owner/approval_issuance/tests.rs` constructs `clock::Configuration`;
  `approvals/tests.rs` and `owner/mutation/tests.rs` are only the homes of the two
  `*_admits_8000_ms` tests.
- The clock status items are `crates/connectors-docs/src/status.rs:161-163` and `:196-198`, not `:169`.
- The width cap is re-checked separately in `approvals/proof.rs:94` and `mutations.rs:469`;
  leaving either at 4,000 ms is the regression this story must not ship.
- The base now carries `0ec7ccd1a` (the clock refusal causes): this branch merged wave 20261014c.

## Decided 2026-10-14: a wide source counts; only the answer is capped

The live check of 2026-10-14 found Cloudflare and roughtime.se unreachable, int08h verified with
a 5,000 ms radius (over the per-source cap) and time.txryan.com verified with a 3,000 ms radius:
one counted source, `below_quorum`. Decided: a verified source counts toward the quorum whatever
its own radius; the combined answer's width stays capped at 8,000 ms. The guarantee (if `quorum`
counted intervals contain true time, the answer does) needs no per-source bound. A verified
source is never `too_imprecise` on its own; `too_imprecise` names only an answer over the cap.
`quorum_source_over_cap_is_not_counted` becomes
`quorum_wide_source_counts_and_only_the_answer_is_capped`. Until a third source answers, the
configured two reachable sources at quorum 2 tolerate no outage and no lying source; the release
note says so.

`roughtime-1` is removed from the model and the host: no source answers it and no reply can be
recorded, and a wire version ships only with its verifier and a recorded fixture.
`roughtime_1_recorded_reply_verifies` is dropped. Every configured source uses
`roughtime-0x8000000c`.
