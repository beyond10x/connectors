---
format: aep.planning-md/3
id: approval-record:wave-d210-capacity-probe
kind: approval-record
status: approved
title: 'Wave D210: the local CLI capacity probe flake, accepted by the orchestrator'
relations:
- decides: story:local-cli-capacity-probe-flake
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-10-29T13:54:18Z", actor: "human:timo", revision: 2}
---
## Approval

The directing session `b10x-chef-orchestrator` approved item D210 on 2026-10-29, under its grant: "a connectors wave for story:local-cli-capacity-probe-flake and its release", "I accept the story (approval-record citing D210). Reproduce first (loop the test until it fails; record the rate), find the cause (race in the probe or in the test), fix the cause, not the assertion; never add #[ignore] or retries. The test must pass 50 of 50 runs after the fix." It also allowed "up to 3 other accepted, unblocked, independent connectors stories only if they exist (wave cap 4); no live credentials", and ordered under D198: "do NOT move the ESS pin (stays 0.75.0)".

## Selection

- story:local-cli-capacity-probe-flake: accepted by this record.
- No other unit: `aep plan artifact list --kind story --status accepted` lists no story on 2026-10-29. Active stories are not accepted work offered to this wave.
- ESS pin stays at 0.75.0 (D198).
