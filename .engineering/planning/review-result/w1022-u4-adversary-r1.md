---
format: aep.planning-md/3
id: review-result:w1022-u4-adversary-r1
kind: review-result
status: active
title: 'Adversary pass 1, wave 20261022 U4: connectors list and Taskfile'
summary: 'break: 1 blocker, 2 warnings (list skipped Config::read checks, unescaped names, state dir dropped instances); fixed in the unit commit'
relations:
- reviews: story:cli-lists-available-connectors
revision: 1
---
unit: U4 story:cli-lists-available-connectors, uncommitted working tree of conn-w1022-u4 on base 8432903e2
verdict: NEEDS-CHANGE
cases: executed 4→7, red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths (part 6)
needs-coordinator: none

1. **Diff stat.** My only change is one new untracked test file, `apps/connectors/tests/adversary_list_connectors.rs`, so it does not show in `git diff --stat`. I changed no implementation file. The tracked diff (47 files) and the other untracked files are the implementor's.

2. **Cases added** (`apps/connectors/tests/adversary_list_connectors.rs`). All 3 were red on their first run, with that file alone.

| Test | What it asserts | Red output (verbatim, cut short) |
|---|---|---|
| `list_refuses_every_configuration_instances_list_refuses` | For 4 configurations that `instances list` refuses with exit 2, `list` refuses too | `unsupported format: instances list exit Some(2), list exit 0`; `[connectors] outside connectors-local/5: … list exit 0`; `disabled connector with an instance: … list exit 0 with … "gitlab","instances":["work"]`; `instance name that is no selector: … "instances":["Work Space"]` |
| `list_never_writes_a_control_character_from_the_configuration` | No ESC byte reaches stdout from an instance name | `exit Some(0), stdout "…gitlab         gitlab.pat                    w\u{1b}[2Jork\n…"` |
| `an_unresolvable_state_directory_does_not_hide_the_configured_instances` | With an absolute `--config` and `--state-dir relative/state`, `list` either refuses or lists `work` | `the configured instance was dropped: {"ok":true,…"id":"gitlab","instances":[]…}` with `left: Array []`, `right: Array [String("work")]` |

3. **Suite run**, after the cases existed: `cargo test --locked --no-fail-fast -p connectors --test list_connectors --test adversary_list_connectors`
   - `list_connectors`: 4 passed.
   - `adversary_list_connectors`: 0 passed, 3 failed.
   - Cargo exit 101.

4. **Findings**

| # | file:line | Verdict / origin | What was measured | What reaches it |
|---|---|---|---|---|
| F1 | `crates/connectors-host/src/local/config.rs:1414` | NEEDS-CHANGE / introduced | `listed_instances` runs only the file admission and the uid check. It skips the rest of `Config::read`'s checks: format, `[connectors]` only in `/5`, selector names, disabled connector with instances, partial pins, alias uniqueness. Its doc comment ("admitted as `Config::read` admits it") and semantics.md ("refused as for every other command") are both false. | A hand-edited configuration. `list` then reports instances that every other command refuses. |
| F2 | `apps/connectors/src/local/list.rs:46` (`human`) | NEEDS-CHANGE / introduced | Instance names are written to the terminal unescaped. The ESC sequence `\u001b[2J` reached stdout. | Same entry point as F1, a quoted TOML key. Fixing F1 (checking selectors) closes it. |
| F3 | `apps/connectors/src/local/list.rs:10-14` | NEEDS-CHANGE / introduced | When `Paths::resolve` fails for any reason, including an unresolvable *state* directory while the config path resolves, `list` silently lists no instances and exits 0. With an absolute `--state-dir`, `list` refuses a bad file correctly (`config_not_private`, exit 2). | `--state-dir` given as a relative path, or no `HOME`/`XDG_STATE_HOME` (a sandbox or `env -i`) together with `--config`. Fix: resolve the config path on its own, since `list` needs no state. |

Suggested fix for F1: call `Config::read_with(path, None)`. It runs the same checks without deriving anything, so no process starts. One snag: with `None`, an unpinned `/5` instance currently fails through `pinned()`. So it needs a "no derive, but admit unpinned" mode, which is a design call for the implementor.

5. **Attacked and not broken**
   - **No process started:** `select::resolve` skips `list` because its input is null. `name()` skips it because the result has no `connector` field. `execute` returns before `resolved()`. Custody and the owner are not touched.
   - **No implementation id in output:** none in the JSON, the human table, `list --help`, `help list`, root help or the refusal stderr.
   - **Global flag placement:** `list --output json`, `list --output=json`, `--output json list` and `list --config X` all work.
   - **Legacy route:** `list describe` fails with a parse error (`cli_parse`), not the legacy route.
   - **Admission parity:** a symlinked, too-large or non-private file and a directory given as `--config` share `text_at` with `Config::read`. With a directory, `list` refuses `config_not_private`, the same as `instances list`.
   - **`task install`:** `service` is already the default feature of `adapters/catalog`, so `--features service` does nothing but is harmless. Both paths are correct, and `apps/connectors` has no other binaries.
   - **Existing tests:** the "nothing started" test can fail, because its stand-in provider exits 1, so any derivation would turn into a refusal.

6. **Paths written outside the worktree**
   - `<wave scratch>/u4/tmp` (the assigned TMPDIR, created; holds the tests' temp dirs, which clean themselves up)
   - `<wave scratch>/u4/probe.sh` (deleted)
   - `<wave scratch>/u4/probe2.sh` (deleted)
   - The lease `adversary-w1022-u4` was taken and released.

```findings
- file: crates/connectors-host/src/local/config.rs
  line: 1414
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: listed_instances skips Config::read's format, connectors-table, selector and enabled checks, so list exits 0 on four configurations instances list refuses with exit 2 (list_refuses_every_configuration_instances_list_refuses)
- file: apps/connectors/src/local/list.rs
  line: 46
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the human table writes unvalidated instance names, so a terminal escape sequence from the configuration reaches stdout (list_never_writes_a_control_character_from_the_configuration)
- file: apps/connectors/src/local/list.rs
  line: 10
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: an unresolvable state directory makes list ignore a resolvable configuration and report no instances with exit 0 (an_unresolvable_state_directory_does_not_hide_the_configured_instances)
```
