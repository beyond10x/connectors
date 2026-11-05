---
format: aep.planning-md/3
id: approval-record:connectors-wave-2026-11-05
kind: approval-record
status: approved
title: 'Wave 2026-11-05: the recorded Claude Code session and a Write-by-name refusal case'
relations:
- decides: story:claude-connects-to-the-mcp-server
- decides: story:mcp-write-operation-call-by-name-is-refused
revision: 2
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
