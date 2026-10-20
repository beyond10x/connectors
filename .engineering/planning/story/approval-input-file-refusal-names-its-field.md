---
format: aep.planning-md/3
id: story:approval-input-file-refusal-names-its-field
kind: story
status: archived
title: A missing or unreadable approval input file is refused naming input_file
relations:
- serves: vision:independent-contract-adapters
- informed_by: review-result:wave-20261019-post0540-u1-adversary-1
scope:
- confidence: cited
  path: apps/connectors-cli-contract/src/runtime.rs
revision: 3
transitions:
- {from: "draft", to: "archived", at: "2026-10-20T00:45:33Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

`approvals issue`, `approvals prepare` and `approvals policy-set` given an `--input-file` that is
missing, a directory or unreadable answer `invalid_input` naming `input_file`, so a consumer reading
the refusal knows which argument to repair.

## Why now

Found by the adversary on wave 20261019-post0540 unit U1
(review-result:wave-20261019-post0540-u1-adversary-1, finding 5, pre-existing, INFEASIBLE).
U1 made every `invalid_input` the approval handlers raise name its argument (`Failure.field`), for
issue https://github.com/beyond10x/connectors/issues/180. A missing or unreadable `--input-file` is
refused earlier, by the ESS presentation runtime
(`apps/connectors-cli-contract/src/runtime.rs:638`), with no `field`. The contract exempts
presentation refusals, but the consumer of #180 still sees a field-less `invalid_input` on the issue
path. Measured by a probe, not by a test.

## Acceptance

- `approvals issue --input-file <missing path>` and `--input-file <a directory>` answer
  `invalid_input` with `field: "input_file"`, from fixtures.

## Scope

- **Files:** `apps/connectors-cli-contract/src/runtime.rs` (~:638) — cited (adversary report)
- **Also likely:** the ESS CLI runtime that generates the presentation layer — inferred; the change may belong upstream in ESS
- **Confidence:** low on where the fix lands
