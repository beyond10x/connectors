---
format: aep.planning-md/3
id: upstream-blocker:ess-cli-dynamic-input-code
kind: upstream-blocker
status: cleared
title: ESS CLI generator answers unparsable dynamic input before the adopter's validator
relations:
- blocks: story:malformed-input-json-is-invalid-input
withholds: test_result
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-09-30T13:06:07Z", actor: "human:timo", revision: 3}
---
Cleared by an ESS release that fixes https://github.com/beyond10x/ess/issues/274 (the generated runtime returns
`cli_dynamic_input` for unparsable or empty dynamic input before the adopter's `DynamicValidator` runs;
`crates/generate/ess-cli-project/src/runtime.rs:666`, present at 0.45.0 and 0.48.0).

## Cleared (coordinator, 2026-09-30)

Not needed: the contract now follows the generator's codes (see story:malformed-input-json-is-invalid-input).
