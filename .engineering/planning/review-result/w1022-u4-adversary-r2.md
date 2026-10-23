---
format: aep.planning-md/3
id: review-result:w1022-u4-adversary-r2
kind: review-result
status: active
title: 'Adversary pass 2, wave 20261022 U4: connectors list and Taskfile'
summary: 'break: 1 blocker, 1 warning (deferred aliases unknown to lookups, entry limit counted them); fixed in the unit commit'
relations:
- reviews: story:cli-lists-available-connectors
revision: 1
---
unit: U4 round 2, no-derive mode in the uncommitted working tree of conn-w1022-u4 (base 8432903e2, fix pass 1)
verdict: NEEDS-CHANGE
cases: executed 7→9, red 3 (both new tests fail, one of them on 2 sub-cases)
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: none new (the assigned TMPDIR from round 1)
needs-coordinator: none

1. **Diff stat.** I only touched the untracked test file `apps/connectors/tests/adversary_list_connectors.rs`, adding 2 tests to it. I changed no implementation file. I also rebuilt `target/debug/connectors-catalog-provider` with `cargo build -p connectors-catalog-provider`; the new tests copy that real provider next to the CLI.

2. **Cases added.** Both were red on their first run, alone. Their preconditions held: `instances list` exited 0 with the real provider.

| Test | What it asserts | Red output (verbatim) |
|---|---|---|
| `list_accepts_every_configuration_instances_list_accepts` | `list` accepts any file `instances list` accepts | `consumer permitted the derived alias: instances list exit 0, list exit Some(2): {…"code":"invalid_configuration"…}` and `default_adapter naming the derived alias: instances list exit 0, list exit Some(2): …` |
| `an_instance_awaiting_a_profile_is_not_counted_against_the_entry_limit` | 64 legacy entries plus a zendesk instance with no profile are accepted by both commands | `list refused a file instances list accepts: {…"invalid_configuration"…}` with `left: Some(2)`, `right: Some(0)` |

3. **Suite run:** `cargo test --locked --no-fail-fast -p connectors --test list_connectors --test adversary_list_connectors`
   - `list_connectors`: 4 passed.
   - `adversary_list_connectors`: 3 passed (the round-1 cases, now green), 2 failed.
   - Exit 101.

4. **Findings**

| # | file:line | Verdict / origin | What was measured | What reaches it |
|---|---|---|---|---|
| R2-1 | `crates/connectors-host/src/local/config.rs:745` | NEEDS-CHANGE / introduced | A deferred instance gets no entry in `adapters`. So every rule that looks up the alias `<connector>.<instance>` there refuses the file in `list`, while every other command accepts it. Two such rules are confirmed: `consumers.*.permissions.connections` and `default_adapter`. A legacy entry's `permissions.custody` target naming a derived alias should hit the same lookup; I read the code but did not run it. | A `/5` file with a derived instance and a consumer allowed to use it. `list` then refuses a file that works everywhere else. |
| R2-2 | `crates/connectors-host/src/local/config.rs:1601` | NEEDS-CHANGE / introduced | `adapters.len() + deferred().count() > 64` counts an instance that is awaiting a profile. A real derivation gives that instance no entry, so `Config::read` does not count it. | 64 legacy entries plus one profile-less instance of a connector with several profiles (zendesk). Narrow. |

Suggested fix for both: give a deferred instance a placeholder that the alias lookups and the count can see. Alternatively, skip the consumer, `default_adapter` and custody alias checks for deferred aliases, and leave them out of the count. Either way an awaiting-profile instance cannot be told apart without the provider, which the contract already says.

5. **Attacked and not broken**
   - **Can `Config::read` now admit something it refused before?** No, by reading the code (no runtime test). In the normal mode `deferred()` is always empty: a derivation either inserts an entry or marks the instance awaiting a profile, and without an installation an unpinned instance still fails through `pinned()`. So the changed uniqueness, `default_instance` and 64-limit lines behave exactly as before for every other command.
   - **Name uniqueness in `list`:** a derived alias that collides with a legacy or pinned entry is refused, through `admit_instances:745` and the alias set in `check_connectors`.
   - **Does the no-derive mode reach anything but `list`?** No. Its only caller is `listed_instances` (`config.rs:1484`), and its only caller is `list.rs:14`. `Paths::resolve` keeps its config checks through `Paths::config`.
   - **Documented gap, not raised as a finding:** a `default_instance` naming an instance that awaits a profile, or an undeclared connector, is accepted by `list` and refused elsewhere. The new semantics.md text says this is what is left unchecked when only the provider can answer.

6. **Paths written outside the worktree:** none new. The assigned TMPDIR `<wave scratch>/u4/tmp` is from round 1, and the lease `adversary-w1022-u4-r2` was taken and released.

```findings
- file: crates/connectors-host/src/local/config.rs
  line: 745
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a deferred instance has no adapters entry, so list refuses /5 files every other command accepts when a consumer permission or default_adapter names the derived alias (list_accepts_every_configuration_instances_list_accepts)
- file: crates/connectors-host/src/local/config.rs
  line: 1601
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the 64-entry limit counts a deferred instance that a real derivation leaves awaiting a profile without an entry, so list refuses 64 legacy entries plus one such instance that instances list accepts (an_instance_awaiting_a_profile_is_not_counted_against_the_entry_limit)
```
