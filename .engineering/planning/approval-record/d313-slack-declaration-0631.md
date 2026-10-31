---
format: aep.planning-md/3
id: approval-record:d313-slack-declaration-0631
kind: approval-record
status: approved
title: 'D313: Slack connector declaration ships alone as 0.63.1; wave D298 as 0.64.0'
summary: U1 slack-connector-declaration as v0.63.1 (D169); U2, U3 and the release-plan move as 0.64.0
relations:
- decides: story:slack-connector-declaration
- decides: story:migrate-writes-the-short-instance-form
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-10-31T00:29:11Z", actor: "human:timo", revision: 2, executor: "agent:conn-d298-u1", correlation: "wave-d298-u1"}
---
## D313: Slack connector declaration as 0.63.1, then wave D298 as 0.64.0

Unit U1, `story:slack-connector-declaration`, ships alone as v0.63.1 under D169: a fix a consumer
is blocked on today, a downstream deployment that ingests Slack. Tree `conn-d298-u1`, branch
`impl/slack-connector-declaration`, from `main` at `023093ecd` (v0.63.0). Targeted tests, then one
full local gate, both through the shared build slot; one bot pull request; release v0.63.1.

U2 (the rest of `story:migrate-writes-the-short-instance-form`), U3
(`story:served-merge-refusal-answers-refused`) and moving
`release-plan:connectors-v0621-rehearsal-findings` to implemented form wave D298, released as
0.64.0. U3 covers only the test-double refusal if no recording of GitHub's real answer exists, and
the live record becomes a follow-up story.

## Approval

The orchestrator session, 2026-10-31 00:18Z, item D313, under the grant of the workspace
instructions (class C: a wave, its commits, pull request, merge and release): "your D298 proposal
is approved as written. U1 ships alone as 0.63.1 under D169, the consumer-blocked rule. U2, U3 and
the release-plan move form wave D298, released as 0.64.0."

## Deviation from the proposal, decided by the implementing session

The proposal named one `connector.json` with two profiles, bot on `operations.json` and user on
`user-operations.json`. A declaration names one selection, and ADR connector-vocabulary decision 5
makes every operation of a connector available on every instance, so a per-profile selection is an
ADR change. `decision-blocker:slack-openrouter-pinned-source` already decided Slack is two
connectors. U1 therefore declares the bot selection as `slack`, lowered from
`connectors/slack.yaml`; the user selection stays pinned, and `slack-user` is
`story:slack-user-connector-declaration`. Both legacy Slack entries migrate (to pinned instances
of `slack`), which is what the blocked deployment needs.
