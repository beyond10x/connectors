---
format: aep.planning-md/3
id: review-result:wave-20261019-post0540-u2-adversary-2
kind: review-result
status: active
title: 'U2 adversary pass 2: write-postflight-present-check'
relations:
- reviews: story:write-postflight-present-check
revision: 1
---
unit: story:write-postflight-present-check, head 9478182c1 plus one untracked test file
verdict: red (one note-sized finding, INFEASIBLE / introduced)
cases: executed 772→777 (connectors-catalog-provider), red 1
origin: introduced 1, pre-existing 0, undecided 0
wrote-outside-worktree: 1 directory, ~/.cache/connectors-wave-20261019b/u2/adversary-2/
needs-coordinator: no

**1. Diff stat.** `git --no-pager diff --stat` is empty: no tracked file changed. `git status --short` shows one new file, a test:
```
?? adapters/catalog/tests/write_postflight_present_adversary_2.rs
```

**2. Cases added** in `adapters/catalog/tests/write_postflight_present_adversary_2.rs`, run alone first (`red-alone.log`).

| Case | What it asserts | Now |
|---|---|---|
| `a_further_preflight_presence_refuses_absent_and_null_with_its_own_code` | a presence check in a further preflight: absent and null both give `Forbidden`, or `Conflict` when that preflight declares `mismatch: conflict` | green |
| `the_correction_leaves_an_absent_text_or_boolean_member_a_protocol_failure` | literal check: absent or null → `UpstreamProtocol`, a different value → `Conflict`. Boolean check: absent → `UpstreamProtocol`, null → `Conflict` | green |
| `an_empty_preflight_answer_is_not_read_as_a_changed_target_by_a_presence` | a preflight read answered 200 with an empty body gives `UpstreamProtocol`, whether a text check or a presence check comes first | **red** |
| `a_declared_refusal_beside_a_presence_reads_only_an_answer_that_fails_it` | create with presence checks plus `refusals` | green |
| `a_presence_after_a_read_reads_an_empty_reread_as_unknown` | `postflight.read` answered 200 with an empty body → unknown | green |

Red output from running the file alone:
```
assertion `left == right` failed: empty answer, presence check first
  left: Some(Conflict)
 right: Some(UpstreamProtocol)
test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
EXIT=101
```

**3. Suite runs** (`-j 8`, TMPDIR at `<worktree>/.local/tmp`):

| Command | Result | Exit |
|---|---|---|
| `cargo test --locked -j 8 -p connectors-catalog-provider --no-fail-fast` | 776 passed, 1 failed (`write_postflight_present_adversary_2`), 64 ignored | 101 |
| `cargo clippy --locked -j 8 -p connectors-catalog-provider --all-targets -- -D warnings` | clean | 0 |
| `cargo fmt -p connectors-catalog-provider -- --check` | clean | 0 |
| `cargo run --locked -j 8 -p connectors-docs -- generate --check` | `135 generated files: current` | 0 |
| `cargo test --locked -j 8 -p connectors-docs` | 50 passed | 0 |

**4. Findings** (head 9478182c1 plus the untracked test):

| Finding | Where | What reaches it | Verdict / origin |
|---|---|---|---|
| The fix makes a presence check true for any answer, including an empty one. A preflight answered 200 with an empty body becomes `Value::Null`, so a presence check that comes first gives `Conflict` marked `.answered()` (or `Forbidden` without `mismatch`); a text or boolean check on the same answer gives `UpstreamProtocol`. Possible fix, not applied: in the preflight loop at `lib.rs:1985`, refuse an `observed` that is `null` (empty or non-JSON body) as `UpstreamProtocol` before reading any check. | `adapters/catalog/src/lib.rs:884` (preflight loop `:1986`) | No shipped selection has a presence check in a preflight; the definition guide offers `"?"` in `before.expect`. Also needs a provider GET that answers 200 with an empty body. | INFEASIBLE / introduced, note |

**5. Tried and could not break:** a presence check in a further preflight with and without its own `mismatch`; text and boolean checks after the fix; a declared `refusals` beside presence checks; a presence check after an empty read; doc generation drift; other code matching on `Expectation` (none outside `lib.rs` and `import.rs`); `"?"` in `also`.

**6. Paths written outside the worktree:** `~/.cache/connectors-wave-20261019b/u2/adversary-2/` (`run.sh`, `red-alone.log`, `suite-provider.log`, `clippy.log`, `fmt.log`, `fmt-apply.log`, `docs-check.log`, `docs-test.log`, `after-fmt.log`).

```findings
[
  {"file": "adapters/catalog/src/lib.rs", "line": 884, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "Since Pinned::Present => true, a preflight answered 200 with an empty body and a presence check first is refused as an answered conflict (or forbidden), where a text or boolean check gives upstream_protocol (test: adapters/catalog/tests/write_postflight_present_adversary_2.rs::an_empty_preflight_answer_is_not_read_as_a_changed_target_by_a_presence)"}
]
```
