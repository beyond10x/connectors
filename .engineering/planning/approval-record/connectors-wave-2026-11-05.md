---
format: aep.planning-md/3
id: approval-record:connectors-wave-2026-11-05
kind: approval-record
status: approved
title: 'Wave 2026-11-05: the recorded Claude Code session and a Write-by-name refusal case'
relations:
- decides: story:claude-connects-to-the-mcp-server
- decides: story:mcp-write-operation-call-by-name-is-refused
revision: 3
transitions:
- {from: "draft", to: "approved", at: "2026-11-05T09:34:06Z", actor: "human:operator", revision: 2}
---
## Wave 2026-11-05 (connectors)

Skill: aep implementing, wave mode. Integration branch `wave/2026-11-05-connectors`, managed tree
`conn-w1105`, from `origin/main` b7240e27a (v0.66.0). Targeted tests per unit, one staged local full gate,
one bot pull request merged through `PUT /pulls/<n>/merge`, one release, v0.67.0.

Go: the directing session `b10x-chef-orchestrator`, 2026-11-05, after the operator's answer recorded in
approval-record:connectors-claude-recording-on-the-existing-login: one recorded `claude -p` run, then
"the recording and the story move ride a small connectors wave (with the two review drafts if they are
ready), gate, PR, merge, release on green".

Selection: `aep plan artifact waves --kind story --status active --format json` at this branch: wave 1
holds both units, no collisions. Unassessed and left out: story:macos-release-provider-starts-through-the-private-copy
and story:provider-programs-run-on-macos (the macOS kill waits on the adopter), story:sustained-live-read-loop
(credential-blocker).

## Units

| unit | story | work |
|---|---|---|
| U1 | story:claude-connects-to-the-mcp-server | the recording rig, the one recorded run (the coordinator runs it), the journey test and the claims |
| U2 | story:mcp-write-operation-call-by-name-is-refused | a runtime case: a Write operation called by name over MCP is refused with no provider request |

Review drafts: story:mcp-write-operation-call-by-name-is-refused is ready (a test only) and is U2.
story:mcp-mutating-tool-call-revision-rule needs an operator decision (which rule a mutating tool follows)
and stays draft.

## Release

v0.67.0: the Claude Code support claim moves from planned to shipped.

## Delivered (2026-11-05)

Both units are merged into `wave/2026-11-05-connectors`. The release commit is 925927b8f (`release: 0.67.0`).

| unit | merge | adversary | result |
|---|---|---|---|
| u1 Claude Code recording | 1894e5ab5 | red, 5 findings, all fixed in the unit; 8 cases | story:claude-connects-to-the-mcp-server implemented |
| u2 Write by name refused | 71bb4ef0b | holds, 1 should-fix fixed in the unit; 6 cases | story:mcp-write-operation-call-by-name-is-refused implemented |

The one `claude -p` run was made by the coordinator on the existing login, under approval-record:connectors-claude-recording-on-the-existing-login.

The local staged gate ran at 925927b8f, serially, with a cache discard between stages; each stage was pre-checked against disk and trees-budget. Results:
- S1: 317 passed.
- S2: 1525 passed, 47 ignored.
- S3: 559 passed, 13 ignored.
- S4a: 1200 passed, 108 ignored.
- S4b: 281 passed, 13 ignored.
- S5a-c: clippy `-D warnings` green.
- 0 failed; peak 12G.

Findings filed as drafts:
- story:owner-bounded-read-rechecks-semantic-effects (from the U2 adversary, tagged from-finding);
- story:claude-mcp-add-registration-is-recorded (needs the orchestrator's go and one recorded run).
