---
format: aep.planning-md/1
id: review-result:adversary-categorize-pass-2
kind: review-result
status: active
title: Adversary pass 2 against the rewritten categorize shim
relations:
- reviews: story:cli-first-level-groups
revision: 1
---
# Adversary pass 2 — `story:cli-first-level-groups`

Worktree `wt-e9b482b00009`, uncommitted over `967062d`. `aep-drive:adversary`, model fable.
164,619 tokens, 45 tool uses, 942 s.

## Header, as returned

```
unit: story:cli-first-level-groups, legacy shim in crates/connectors-cli/src/lib.rs — worktree wt-e9b482b00009, branch impl/cli-first-level-groups, uncommitted over 967062d
verdict: red (NEEDS-CHANGE)
cases: executed 66→70, red 4
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 paths (both under ~/.cache/claude-tmp/adv-shim-pass2/)
needs-coordinator: the `serve` + global-only finding is a fork between CHANGELOG.md:24-28 and lib.rs:467-470; which document wins is the unit's call, not mine
```

## 1. Diff — proof of what it touched

`git --no-pager diff --stat` unchanged from hand-over: `36 files changed, 1283 insertions(+), 530
deletions(-)`, all the unit's. Its only write is one untracked test file:
`?? crates/connectors-cli/tests/adversary_shim_pass5.rs`. No non-test path touched.

## 2. Cases added

`crates/connectors-cli/tests/adversary_shim_pass5.rs`, each run alone before the suite, all red:

| case | asserts | red output (verbatim, run alone) |
|---|---|---|
| `the_double_dash_escape_is_not_a_word_that_moved` | `moved()` returns `None` for a word behind `--`; binary `-- doctor --help` exits 2 with no note | `` `connectors -- doctor` was rewritten to `connectors inspect doctor` `` … `` `connectors -- serve --config x` was rewritten to `connectors serve local --config x` `` … `` `connectors -- help doctor` was rewritten to `connectors help inspect doctor` `` (5 argvs) |
| `serve_with_only_the_global_format_flag_serves_or_names_where_the_server_went` | for 4 spellings × 2 positions `moved()` lands on `serve local`; binary `serve -o json` serves or names `connectors serve local` | `` `connectors -o json serve` → None `` … `` `connectors serve -ojson` → None `` (8 argvs) |
| `an_argument_neither_the_group_nor_the_leaf_declares_is_not_the_old_leaf` | `moved()` returns `None` for `serve --bogus/-V/--version/--hlep`; `serve --hlep` stderr equals `setup --hlep` stderr with the group name swapped | `` `connectors serve --bogus` was rewritten to `connectors serve local --bogus` `` … `` `connectors serve --hlep` was rewritten to `connectors serve local --hlep` `` (4 argvs) |
| `a_positional_value_spelled_help_is_a_value_not_a_help_request` | `login help` / `connect help` hand on `help` as the positional | `` `connectors login help` is handed on as `connectors help session login`, not `connectors session login help` `` (2 argvs) |

## 3. Suite run

After part 2: `cd crates/connectors-cli && cargo test --no-fail-fast` → `error: 1 target failed`,
non-zero. `test result:` lines, all 14 binaries:

| binary | result |
|---|---|
| unittests src/lib.rs | ok. 5 passed |
| unittests src/main.rs | ok. 0 passed |
| adversary_cli_cap_pass3 | ok. 1 passed |
| adversary_fence_probe | ok. 6 passed |
| adversary_fence_probe_pass2 | ok. 3 passed |
| adversary_shim_pass3 | ok. 6 passed |
| adversary_shim_pass4 | ok. 3 passed |
| **adversary_shim_pass5** | **FAILED. 0 passed; 4 failed** |
| cli_surface | ok. 20 passed |
| cli_surface_drift | ok. 10 passed |
| cli_surface_pass_two | ok. 6 passed |
| first_level_groups | ok. 5 passed |
| moved_paths_are_not_taught | ok. 1 passed |
| doc-tests | ok. 0 passed |

## 4. Findings

"before" = `~/.cargo/bin/connectors` 0.5.11, the pre-release 16-command build, run with HOME/XDG
pointed at scratch.

| # | file:line | what was measured | what reaches it | verdict / origin |
|---|---|---|---|---|
| 1 | `crates/connectors-cli/src/lib.rs:519` (`allow_external_subcommands(true)`, no positional) | `connectors -- doctor`: before → exit 2 "unexpected argument 'doctor' found / tip: remove the '--'"; now → note + doctor runs (exit 1, unhealthy). `-- serve --config x --help` → `Usage: connectors serve local`. `--` is dropped from the rebuilt argv | nothing found — the release before refused it, so no caller exists | CONFIRMED / introduced |
| 2 | `crates/connectors-cli/src/lib.rs:606` (two-kind filter) vs `CHANGELOG.md:24-28` | before: `serve -o json`, `-o json serve`, `serve --output=json` each printed the readiness JSON and served (killed at 3 s, exit 124); now: `serve -o json` → "'connectors serve' requires a subcommand" exit 2, `-o json serve` → group help exit 2, no note either way. Real parse fails with `MissingSubcommand` / `DisplayHelpOnMissingArgumentOrSubcommand`, which stage 2 does not take | `connectors serve` with no config was the personal posture (`docs/design/02-architecture.md:113` at base); `-o json` was the documented way to get the readiness document as JSON | NEEDS-CHANGE / introduced |
| 3 | `crates/connectors-cli/src/lib.rs:655` (guard only checks `next` is the group's) | `serve --hlep`: before → "unexpected argument '--hlep' … Usage: connectors serve --help"; now → note "`connectors serve` is now `connectors serve local`" + "Usage: connectors serve local --help". Same for `--bogus`, `-V`, `--version`. Control `setup --hlep` gets clap alone | anyone mistyping a flag at the `serve` group | CONFIRMED / introduced |
| 4 | `crates/connectors-cli/src/lib.rs:636` (`help` taken at any walked position) | `login help` before → login to base `help` (exit 1 at discovery); now → `help session login`, prints help exit 0. `connect help` likewise | nothing found beyond a person typing it | CONFIRMED / introduced |

Fixes named, not applied: (1) refuse the rewrite when the reader consumed `--` — check
`words.iter().any(|w| w == "--")` before the external word, or declare a positional; (2) either take
`MissingSubcommand` at a `MOVED` group whose only arguments were globals, or amend CHANGELOG.md:24-28
to list `serve` + global-only as the group (both resolutions leave the changelog and the code
agreeing, which they do not now); (3) after the walk, require the first tail token to be a long/short
the *new leaf* declares before firing the `serve` row; (4) do not take `help` after a walked word
whose new leaf declares a positional.

## 5. Attacked and held

| class | driven | result |
|---|---|---|
| `LEGACY_WINDOW = 8` | repeated `-o` ×2/×3/×4, mixed spellings | not reachable: before-binary refuses any repeat for a real run ("cannot be used multiple times"); only `--help` variants printed help, because clap exits on help before validating |
| stage 2 third error kinds | `--output doctor`, `--config doctor`, `-o \xff doctor`, `serve-hosted` w/o `--config`, `serve --config` w/o value | clap answers as before, no note |
| current path with `UnknownArgument` | `inspect providers --query doctor --bogus`, `serve local --bogus`, `serve --bogus local` | not rewritten (except the `serve --bogus` class, finding 3) |
| `serve` matrix | bare, `--help`, `-h`, `help`, `help serve`, `local --help`, `--config x`, `--config=x`, `--state-root x`, `-o json --config x`, `--config x -o json`, `--config x local` | group where the group was meant, leaf where `--config`/`--state-root` present |
| tail rebuild | globals both sides (`-o json doctor -o yaml`), `doctor -- --config x`, non-UTF-8 `--config $'\xff\xfe' --state-root $'\xfe'` | acceptance example holds byte-for-byte: stdout identical, stderr identical after the note |
| `help` shapes | `help auth`, `help auth status`, `auth help status`, `auth -o json help`, `help -o json auth status`, `help help doctor` | all land on `inspect auth` / clap's own refusal |
| `pub fn moved` vs `run_from` | same function, same `command()`; zwirn (`agent-app/src/main.rs:195`) builds `argv[0]="connectors"` | no disagreement; zwirn unaffected at pin `1e0eb9f` |
| `moved_paths_are_not_taught` scan | grep of everything it does not read | one comment only: `scripts/check-local-identity-refused.sh:4` names `connectors serve-hosted`; `.github`, non-.rs assets under `src`, Dockerfile shell-form: nothing |
| citation fence | preposition-phrased citations ("`X` in `path:line`"), `components.yaml:109` → `lib.rs:55-94` | none exist; the one checked lands on `enum Command` |

## 6. Paths written outside the worktree

- `~/.cache/claude-tmp/adv-shim-pass2/xdg/` — HOME/XDG for both binaries (contains
  `b10x/connectors/connectors.sock` from the pre-release server runs, `l.out`, `l.err`, `c.out`,
  `c.err`)
- `~/.cache/claude-tmp/adv-shim-pass2/probe/` — scratch cargo project replicating the reader, with
  its `target/` (61 MB total for the directory)

## 7. Findings, verbatim

```findings
- file: crates/connectors-cli/src/lib.rs
  line: 519
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "`connectors -- doctor` was refused before (exit 2, clap tip to remove `--`) and now runs `inspect doctor` with the `--` escape dropped from the rebuilt argv; `-- serve --config x` reaches `serve local`."
- file: crates/connectors-cli/src/lib.rs
  line: 606
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "`connectors serve -o json`, `-o json serve` and `serve --output=json` served with JSON readiness before and now exit 2 with no note naming `connectors serve local`, while CHANGELOG.md:24-28 promises every old path works with the global in front in any spelling."
- file: crates/connectors-cli/src/lib.rs
  line: 655
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "`connectors serve --hlep` (also `--bogus`, `-V`, `--version`) is rewritten to `serve local` and prints a note for a path not typed plus `Usage: connectors serve local --help`, where `setup --hlep` gets clap's group refusal alone."
- file: crates/connectors-cli/src/lib.rs
  line: 636
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "`connectors login help` and `connectors connect help` were a login to base `help` and a connect to provider `help` before, and are now rebuilt as `help session login` / `help setup connect` and print help, exit 0."
```
