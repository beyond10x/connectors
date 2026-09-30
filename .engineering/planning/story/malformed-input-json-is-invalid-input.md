---
format: aep.planning-md/3
id: story:malformed-input-json-is-invalid-input
kind: story
status: archived
title: Malformed business JSON on invoke is invalid_input
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: apps/connectors-cli-contract
- confidence: cited
  path: crates/connectors-conformance/tests/cli_surface.rs
- confidence: cited
  path: crates/connectors-spec/toolchain.json
revision: 5
transitions:
- {from: "draft", to: "archived", at: "2026-09-30T13:06:06Z", actor: "human:timo", revision: 5}
---
## Observed
Found by the black-box CLI surface test of release 0.18.0 on 2026-09-30 (raw output under the tester's sandbox, outside the repository).
`operations invoke ... --input-json 'not json'` and `--input-stdin </dev/null` answer
`{"error":{"code":"cli_dynamic_input","data":{}}}`; a duplicate key answers `invalid_input`. scenarios.md:25 (C05)
requires `invalid_input` for both.
## Acceptance
- Unparsable, empty and duplicate-key business input all answer `invalid_input` with no input echoed.

## Decided (coordinator, 2026-09-30)

The code comes from the ESS CLI generator (`crates/generate/ess-cli-project/src/runtime.rs:666` at 0.45.0 and 0.48.0;
generated here as `apps/connectors-cli-contract/src/runtime.rs:661-669`), which answers before the adopter's validator
runs. Filed as https://github.com/beyond10x/ess/issues/274. This story waits for an ESS release that fixes it, then
moves the pin, regenerates the contract, and makes `crates/connectors-conformance/tests/cli_surface.rs:620-637`
assert the code (not only exit 2), with an empty-stdin case. No hand edit of the generated file.

## Withdrawn (coordinator, 2026-09-30, after the ESS design review of the CLI domain)

The contract contradicts itself: semantics.md:159-160 hands parser and input codes to `ess-cli/1`, which emits
`cli_parse`, `cli_input`, `cli_source` and `cli_dynamic_input` with empty data, while :430, :433 and C05 ask for
`invalid_input`. ESS 0.45 cannot remap these codes (`DynamicError` carries no code). Decision: the contract follows
the generator; story:cli-surface-minor-findings-0-18-0 corrects :430, :433 and C05. This story's acceptance no longer
holds. beyond10x/ess#274 stays open as an improvement.
