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
revision: 4
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

## D314: does 0.63.1 unblock a user-token entry addressed as `catalog/slack-user`

The orchestrator session, 2026-10-31, item D314: "as built, PR 227 does not unblock the consumer.
The blocked deployment invokes `catalog/slack-user`, not `catalog/slack`." It asked whether the
ADR admits a separate `slack-user` declaration, to be added to 0.63.1, with the acceptance "that
repro gets past legacy_entry_unmigrated", and held the merge of PR 227.

- The ADR admits it: `decision-blocker:slack-openrouter-pinned-source`, option A, keeps Slack as
  two connectors, `slack` and `slack-user`. No ADR change is needed.
- The target `catalog/<alias>` names a legacy catalog entry no connector is derived for, and is
  refused as `legacy_entry_unmigrated` whatever is declared
  (`contracts/cli/v1alpha1/semantics.md`, "A catalog entry no connector is derived for is no
  instance"; `apps/connectors/src/local/invoke.rs` `names_unmigrated`). Once a declaration names
  the entry's provider, the entry is named `<connector>/<alias>`. A `slack-user` declaration
  would name it `slack-user/slack-user`. Neither makes `catalog/slack-user` answer.
- Observed with this branch's binaries (head f65712d69) on a scratch configuration holding one
  guide-shaped legacy entry `[adapters.slack-user]` (provider `slack`, `user-operations.json`,
  profile `slack.user`), `~/.cache/connectors-d240/u1/repro.out`:
  `instances list` lists `slack/slack-user` and no unmigrated entry; `invoke catalog/slack-user
  conversations.history` is `not_found` at admission; `invoke slack/slack-user
  conversations.history` reaches `connection_unresolved` (no credential in the scratch state);
  `setup migrate --mode dry-run` migrates it to `slack/slack-user`.
- Decision of the implementing session: 0.63.1 stays as built. Its acceptance is met: the
  user-token entry is past `legacy_entry_unmigrated` under the target `slack/slack-user`. The
  deployment's target changes from `catalog/slack-user` to `slack/slack-user`. A `slack-user`
  declaration needs a second declaration in one catalog provider directory, which the declaration
  layout, the lowerer's output path and the declaration tests rule out; it stays
  `story:slack-user-connector-declaration`, and would rename the instance a second time.

D315, the orchestrator session, 2026-10-31: "go. PR 227 stays as built". The orchestrator tells the deployment to switch its target to `slack/slack-user`; `story:slack-user-connector-declaration` stays a follow-up story.

## D321: no fifth local full gate; CI's full gate decides 0.64.0

Local full gates 1 to 4 of wave D298 each failed on a generation or drift step, and each wrote
34 to 39 GB against the machine's write budget. The fixes after gate 4 (head `f737d9687`) were
checked by the gate's generation and drift steps alone (`drift.sh`, 14 steps, all exit 0): build
tool, `metadata-entities --check`, `ess-boundary`, `source-hashes`, `release-binaries`, `cli
--check`, `mcp-bindings --check`, `service-metadata --check`, `ess specify validate`, `cargo fmt
--check`, `tool-contracts generate --check`, `connectors-docs generate --check`, `cargo test -p
connectors-build --tests` and `aep plan artifact validate`.

Following the workspace rule for a full suite larger than the disk budget, the wave runs the full
tests of every crate its units touched (`connectors-sdk`, `connectors-host`, `connectors-core`,
`connectors-catalog-provider`, `connectors`) instead of a fifth local gate. The pull request's CI
full gate is the full gate of record for 0.64.0. If it fails, the implementing session reports the
failing test and does not start a local full gate.

## Approval

The orchestrator session, 2026-10-31 05:24Z, item D321: "no local gate 5. Record it in your store.
The workspace rule (delivery in waves) applies: where the full suite does not fit the disk budget,
run every touched crate's full tests plus the projection checks, and let CI's full gate run on the
PR."
