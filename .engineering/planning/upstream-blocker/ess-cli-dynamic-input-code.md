---
format: aep.planning-md/3
id: upstream-blocker:ess-cli-dynamic-input-code
kind: upstream-blocker
status: open
title: ESS CLI generator answers unparsable dynamic input before the adopter's validator
relations:
- blocks: story:malformed-input-json-is-invalid-input
withholds: test_result
revision: 1
---
Cleared by an ESS release that fixes https://github.com/beyond10x/ess/issues/274 (the generated runtime returns
`cli_dynamic_input` for unparsable or empty dynamic input before the adopter's `DynamicValidator` runs;
`crates/generate/ess-cli-project/src/runtime.rs:666`, present at 0.45.0 and 0.48.0).
