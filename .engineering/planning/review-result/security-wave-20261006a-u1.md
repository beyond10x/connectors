---
format: aep.planning-md/3
id: review-result:security-wave-20261006a-u1
kind: review-result
status: active
title: Security review, wave 20261006a U1 (consumer launch)
relations:
- reviews: story:launch-consumer-with-connection-credential
revision: 1
---
```
unit: story:launch-consumer-with-connection-credential (wave 20261006a U1), uncommitted working tree on unit/launch-consumer at 3427d41838
verdict: CONFIRMED
cases: executed 313→318, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 4 paths (part 6)
needs-coordinator: register my two #[ignore] helpers in crates/connectors-build/src/ignored.rs (non-test file), or drop them; connectors-build not run, disk at 4.0 GB
```

Of the 8 invariants, 7 hold. Invariant 5 holds for descriptors the launcher opens itself, but not for descriptors its caller left open: those reach the consumer.

**1. Diff stat.** Unchanged from the unit's own 22 files. My only edit is a new `mod review_conformance` appended at line 534 of `crates/connectors-host/src/local/runtime/launch/tests.rs`. That file is untracked, so the stat does not show it. No non-test path was touched.

**2. Cases added** (`local::runtime::launch::tests::review_conformance::`, each run alone first)

| case | invariant | now |
|---|---|---|
| `review_a_descriptor_the_caller_left_open_does_not_reach_the_consumer` | 5 | **red** |
| `review_the_launchers_own_descriptors_do_not_reach_the_consumer` | 5 | green |
| `review_an_entry_without_pass_env_starts_the_consumer_with_an_empty_environment` | 4 | green |
| `review_the_consumer_cannot_change_descriptor_three_or_its_seals` | 2, 7 | green |
| `review_a_swapped_binary_is_refused_and_the_captured_image_is_what_runs` | 3 | green |

Output of the red case, run alone:
```
panicked at crates/connectors-host/src/local/runtime/launch/tests.rs:752:9:
assertion `left == right` failed: {"add_seal":1,"descriptors":[[0,"/dev/null"],[1,"pipe:[2923295483]"],[2,"pipe:[2923295484]"],[3,"/memfd:connectors-credential (deleted)"],[7,"~/.cache/b10x-target/connectors-w-launch/debug/deps/connectors_host-21a081423ba250bf"]],"env":[],...}
  left: [0, 1, 2, 3, 7]
 right: [0, 1, 2, 3]
```

**3. Suite run.** `cargo test -p connectors-host --lib` (brief's env), after `--list` confirmed this tree's binary has the cases: `test result: FAILED. 317 passed; 1 failed; 31 ignored`, exit 101. The only failure is the red case. The before count is the implementor's `green2-host.log`: 313 passed, 29 ignored.

**4. Findings**

| file:line | severity | verdict / origin | finding |
|---|---|---|---|
| `crates/connectors-host/src/local/runtime/launch.rs:283` | warning | CONFIRMED / introduced | The `pre_exec` places fd 3 but closes nothing above it. Any descriptor the caller passed without close-on-exec (a shell's `exec 7<file`, a supervisor's sockets) reaches the consumer. **Reached by:** the production `connections launch` path, `Consumer::run` from `connections.rs`. Custody is unaffected: the leaked descriptors are the caller's own, never the owner's. Fix: `close_range(4, ~0U, 0)` in the hook, or state the inheritance in consumer-launch.md §4. |
| `crates/connectors-host/src/local/owner/transport.rs:1519` | note | CONFIRMED / introduced | The owner captures and hashes the image before reading the credential (`:1531`). That holds when read, but no test pins the order: moving `capture` after the custody read stays green, because the disposable journey's wrong-digest case is refused earlier in `admit_launch`. Owner-side ordering is untestable without a Secret Service, so this stays a judgement finding. |

**5. Confirmed, and how**

| # | evidence |
|---|---|
| 1 | Read the code. The CLI receives descriptors, never reads them, and drops them after spawn. The CLI's custody probe reads no item, and its socket is a std `UnixStream`, so close-on-exec. |
| 2 | Green case: fd 3 digest matches the run-time material, env empty. Code: the reply carries only `args` and `pass_env`, and the final answer is `{}`. |
| 3 | Green case: after an on-disk swap, check and capture are refused and the captured `true` still exits 0. `capture` hashes the copy it seals. |
| 4 | Green case, plus the implementor's config test refusing `pass_env = ['']`. |
| 5 | Holds for the owner socket, the CLI's own file, a bus-like socket and the image copy (green case). Fails for caller-inherited descriptors (red case). |
| 6 | Read: owner order is admit, capture, then custody read. Malformed and NUL `--args` are refused in the CLI before admission (unit's CLI test). |
| 7 | Green case, run inside the consumer: write is EBADF; reopen-and-write, shrink, grow, adding a seal and a shared writable mmap are all EPERM; seals = 15. |
| 8 | Read: owner socket mode 0600, peer uid checked with `SO_PEERCRED` (`channel.rs:53`). Any same-user process can request a launch and receive the descriptor; consumer-launch.md §5 and the user doc state that. |

**6. Paths written outside the worktree**
- `~/.cache/connectors-wave-20261006a/u1/review/red-inherited.log`
- `~/.cache/connectors-wave-20261006a/u1/review/green-cases.log`
- `~/.cache/connectors-wave-20261006a/u1/review/suite-host.log`
- `~/.cache/b10x-target/connectors-w-launch` (rebuilt the `connectors-host` test binary)

`git status --short` of my additions: the file was already `?? crates/connectors-host/src/local/runtime/launch/` before I started, so status is unchanged.

**7.**
```findings
[
  {"file": "crates/connectors-host/src/local/runtime/launch.rs", "line": 283, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "the consumer inherits every non-close-on-exec descriptor the caller passed to the CLI, not only 0-3; test review_a_descriptor_the_caller_left_open_does_not_reach_the_consumer is red"},
  {"file": "crates/connectors-host/src/local/owner/transport.rs", "line": 1519, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "capturing the consumer image before the custody read is correct as written, but no test fails if the order is reversed"}
]
```