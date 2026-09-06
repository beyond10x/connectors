---
format: aep.planning-md/1
id: decision-blocker:cli-credentials-final-review
kind: decision-blocker
status: open
title: Credential unit needs an operator decision after two attacks
relations:
- blocks: story:one-placement-several-credentials
withholds: test_result
revision: 1
---
The second complete adversarial pass of story:one-placement-several-credentials returned a confirmed blocker. The immutable report is review-result:cli-credentials-adversary-2-20260906; source under review is 9f15c108fbd18cf6f079f0b04c0a9bb746b1ac0d. It measured 234 executions: 233 passed and one failed, with strict clippy and formatting passing.

A denied read-only Slack user credential suppresses description of the independently callable writable bot. The new available_connections helper reads every configured binding before restricting the result to the requested operation's admitted Connections. The coordinator classifies the finding as introduced: base 76f3fef9ce53a92d54d5e1c8147c5943315d423f described operations without this global custody scan; the failing scan and selected-binding model arrive with this unit. This classification is based on the whole-unit source comparison, not a claimed execution on base. The adversary's original undecided classification remains unchanged in its immutable report.

The aep-drive wave skill0.8.0 says: "red after two full attacks" -> "stop attacking. It goes to a person". The unit leaves this wave's merge set. No third attack, further implementation correction or integration is authorized by that workflow rule alone. The operator must decide whether to authorize another correction/review cycle or leave this unit for a later wave. All source and added tests remain in managed tree wt-03d120cf8736, branch impl/one-placement-several-credentials; the final two test files remain uncommitted. This blocker withholds completion evidence; it is not a claim that the existing installed Slack discovery fix is reverted.

Independent one-shot work does not need this branch: its test incorrectly placed two bot identities in the same unnamed legacy credential slot. Giving each fixture an explicit instance preserves distinct Connection references, exact credential selection and grant checks using existing production semantics. The original failing fixture output remains in scratch and is not attributed to one-shot production code.
