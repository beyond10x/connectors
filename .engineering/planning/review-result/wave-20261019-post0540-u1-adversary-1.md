---
format: aep.planning-md/3
id: review-result:wave-20261019-post0540-u1-adversary-1
kind: review-result
status: active
title: 'U1 adversary pass 1: approval-issue-slack-bot-post'
relations:
- reviews: story:approval-issue-slack-bot-post
revision: 1
---
unit: story:approval-issue-slack-bot-post
verdict: red
cases: executed 890→894, red 3
origin: introduced 4, pre-existing 1, undecided 0
wrote-outside-worktree: ~/.cache/connectors-wave-20261019b/u1/adversary-1/ (probe/, gen-*.log, suite-*.log); ~/.cache/sccache/connectors (shared compile cache, by its env vars)
needs-coordinator: yes. Finding 1 has two possible fixes. One is a contract line in a file the unit owns. The other is code in `apps/connectors/src/local/select.rs`, which is in neither the unit's "yours" nor its "not yours" list.

Findings cover commit 6d4884191 plus the adversary's test files, uncommitted in the unit worktree.

**1. Diff stat** (tracked files, plus the new untracked ones)
```
 crates/connectors-host/src/local/owner/approval_issuance/tests.rs | 3 +++   (mod line only: #[path = "tests/adversary_field.rs"] mod adversary_field;)
?? apps/connectors/tests/approval_field_adversary.rs
?? crates/connectors-host/src/local/owner/approval_issuance/tests/adversary_field.rs   (new #[cfg(test)] child module, needed to reach the private fixtures)
```
No implementation file changed.

**2. Cases added**, each run alone first

| Case | Asserts | Now |
|---|---|---|
| `approval_field_adversary::approvals_issue_refusing_its_instance_selection_names_adapter` | `approvals issue` with `--connector slack` only, with `--adapter` and `--connector`, and with no selector: `field == "adapter"`, as semantics.md:1138 says | red |
| `approval_field_adversary::a_write_invoke_refusal_carries_no_approval_field` | `operations invoke` of a write with `--connection "not an id"`: no `field` | red |
| `approval_issuance::tests::adversary_field::issue_admits_a_proof_output_in_a_private_directory_named_through_a_symlink` | a 0700 directory named through a symlink gets the same answer as the same directory named directly | red |
| `approval_field_adversary::approval_commands_name_the_refused_argument_on_the_cli` | from the CLI: issue gives `connection`, prepare gives `input`, policy-set gives `input_file` | green |

Red output, verbatim:
```
left: [("--connector slack", String("invalid_input"), Null), ("--adapter slack --connector slack", String("invalid_input"), Null), ("", String("invalid_input"), Null)]
right: [(... String("adapter")), (... String("adapter")), (... String("adapter"))]
---
assertion `left == right` failed: {"code":"invalid_input","field":"connection","kind":"usage","next_action":"none","stage":"arguments"}
  left: Some(String("connection"))   right: None
---
adversary_field.rs:49:5  left: (InvalidInput, Some("proof_output"))  right: (Unavailable, None)
```

**3. Suite**
- `cargo test --locked -j 8 --no-fail-fast -p connectors-host`: exit 101, 634 passed, 1 failed (the symlink case), 34 ignored.
- `cargo test --locked -j 8 --no-fail-fast -p connectors`: exit 101, 257 passed, 2 failed, 10 ignored.

**4. Findings**

| # | file:line | Verdict / origin | What was measured | What reaches it |
|---|---|---|---|---|
| 1 | select.rs:84 against semantics.md:1138 | NEEDS-CHANGE / introduced | A refused instance selection on an approval command is `invalid_input` with no `field`; `select::resolve` raises it before `approvals::execute`. `Argument::Adapter` is effectively unreachable. | The Slack guide's `--connector/--instance` selectors with one missing or with `--adapter` added; #180's command. |
| 2 | local.rs:317 | NEEDS-CHANGE / introduced | `operations invoke` of a write now emits `field` via `issuance::resolve`; the same refusal on a read carries none. | Every write invoke with an approval file. |
| 3 | semantics.md:1139 | CONFIRMED / introduced (the text) | The contract lists the `proof_output` conditions as complete, but a path through a symlink is also refused (`O_NOFOLLOW`). | Paths under macOS `TMPDIR` and home directories reached through a symlink. |
| 4 | local.rs:315-318, approvals.rs:16-35 | CONFIRMED / introduced | Before the green case, nothing outside the ignored tests read `Failure.field` from the CLI. | Any change to `owner_failure` or `approvals.rs`. |
| 5 | apps/connectors-cli-contract/src/runtime.rs:638 | INFEASIBLE / pre-existing | `approvals issue --input-file <missing or a directory>` answers `invalid_input` with no `field` from the presentation runtime. | The Slack guide's `--input-file ~/private/post.json`; needs a change in the ESS presentation runtime. |

**5. Attacked and could not break:** generators (`connectors-build cli --check`, `metadata-entities --check`, `ess-boundary`, `connectors-docs generate --check`) exit 0; `Argument` is a closed enum and no caller text reaches `field`; no store refusal maps to a wrong `field`; cross-build wire refused by `same_build` (transport.rs:670); `:293`/`:311` answering `unavailable` is unobservable; #180's issue-only `invalid_input` sites are the four `proof_output` checks plus findings 1 and 5; the unit's tests assert literal values. Not run: the ignored Slack journey.

```findings
[
  {"file": "apps/connectors/src/local/select.rs", "line": 84, "category": "contract-drift", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "approvals_issue_refusing_its_instance_selection_names_adapter: a refused --connector/--instance/--adapter selection on approvals issue is invalid_input with no field, though semantics.md:1138 promises `adapter` for the instance selection and the handler, not the presentation, raises it"},
  {"file": "apps/connectors/src/local.rs", "line": 317, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "a_write_invoke_refusal_carries_no_approval_field: operations invoke of a write now emits Failure.field (via issuance::resolve) while the contract gives field to approval commands only and the read path emits none"},
  {"file": "contracts/cli/v1alpha1/semantics.md", "line": 1139, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "issue_admits_a_proof_output_in_a_private_directory_named_through_a_symlink: the closed list of proof_output conditions omits that a symlinked component is refused (O_NOFOLLOW), so a path meeting every listed condition is refused naming proof_output"},
  {"file": "apps/connectors/src/local.rs", "line": 315, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "approval_commands_name_the_refused_argument_on_the_cli (added, green): before it, only the ignored Secret Service journey read Failure.field from the CLI, so removing the emission in owner_failure or approvals.rs left the gate green"},
  {"file": "apps/connectors-cli-contract/src/runtime.rs", "line": 638, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "pre-existing", "message": "probe (no test): approvals issue with a missing or unreadable --input-file answers presentation invalid_input with no field; the contract exempts it but #180's consumer still gets a field-less invalid_input on the issue path, and fixing it needs the ESS presentation runtime"}
]
```
