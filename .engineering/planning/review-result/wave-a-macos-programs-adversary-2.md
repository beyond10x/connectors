---
format: aep.planning-md/3
id: review-result:wave-a-macos-programs-adversary-2
kind: review-result
status: active
title: Adversary pass 2 on provider-programs-run-on-macos (wave A, issue 229)
relations:
- reviews: story:provider-programs-run-on-macos
revision: 1
---
```
unit: story:provider-programs-run-on-macos, pass 2 on HEAD~1..HEAD (e5713176f), unit 9069c43a4..e5713176f, worktree cz-u229b, clean
verdict: green
cases: executed n/a→n/a (no case added: every target is cfg(macos) and can't run here), red 0
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: 6 paths under <unit scratch>/scratch/adv2/ (part 6)
needs-coordinator: no
```

The correction answers pass 1 except one half of F3: the owner's own clean-up call in `serve` is still untested. Nothing I found blocks the Acceptance, but it still needs a green macOS CI run on this head.

1. `git --no-pager diff --stat`: empty. I changed nothing in the worktree.
2. Cases added: none. The new test and everything it covers only build and run on macOS.
3. Suite run: none, because I added no case. Two checks instead:
   - **macOS lint:** `cargo clippy --locked --target aarch64-apple-darwin -p connectors-host --all-targets -- -D warnings`, with a fake C compiler and no sccache, gave `Finished ... in 2m 17s`, exit 0. The new test compiles and lints clean for macOS.
   - **Generated docs:** `cargo run --locked -p connectors-docs -- generate --check` gave `139 generated files: current`, exit 0. The regenerated contract page matches `semantics.md` word for word.
4. Findings:
   - **The owner's clean-up call is still untested** (`crates/connectors-host/src/local/owner/transport.rs:1580`, warning, CONFIRMED, introduced). The new test proves `own_copy()` finds the copy and that dropping it removes it. It never runs `serve_with_read_policy`. If line 1580 were deleted, every macOS owner would leave a full copy of `connectors` in `$TMPDIR` each time it starts, and no test on any platform would fail. No test lists the temp directory or names `connectors-image`.
     - What reaches it: every owner start on macOS.
     - Fix: a macOS test under `apps/connectors/tests` that starts the owner through the CLI with a fresh `TMPDIR`, stops it, waits for it to exit and asserts no `connectors-image.*` directory is left.
   - **Two comments still say the CLI removes the copy once the owner answers.** Both are note, CONFIRMED, introduced, and neither is in the base. Pass 1 missed them.
     - `crates/connectors-custody/src/keychain.rs:142`
     - `crates/connectors-custody/src/tests.rs:553`
   - **The journey's doc comment overpromises** (`adapters/grafana/tests/local_runtime/cli_journey.rs:579`, note, CONFIRMED, introduced). It says the test "removes every item it wrote at the end". The body only tries to remove them and reports the result, because the test can't delete items the owner created.
5. Attacked and could not break:
   - **Host test step:** `cargo test -p connectors-host -p connectors` runs the lib tests. The new test is not ignored and `TMPDIR` is set, so it runs.
   - **The name `image`:** the test assumes it, but `capture` always names the copy `IMAGE`, the same as production, and `own_copy` requires it.
   - **Wrong child test name:** the child would run 0 tests and exit 0, but the copy would stay, so the parent's `!present` assertion goes red. The test can't pass without running.
   - **Mutants on `own_copy`:** if it always returns `None`, the child's assertion goes red. If the name check is dropped, the existing assertion `a_left_launch…: own_copy().is_none()` goes red.
   - **Running a test binary from a copy on CI runners:** the existing transport probe already does this, and it has been green on macOS CI before.
   - **Journey step:** with `--exact` and `--include-ignored` it runs exactly one test, and the grep for `1 passed` catches a name mismatch. The `--skip` in "Test the programs" is now redundant and harmless.
   - **Contract text vs code:** the timeout reaper, a failed greeting (kill, then the launch is dropped) and owner exits all match the new text in `semantics.md`, `owner.md` and `cli.yaml`. The owner always returns from `serve`; every `process::exit` in the host is test-only.
6. Paths written outside the worktree, all under `<unit scratch>/scratch/adv2/`: `clippy-mac-host.sh`, `clippy-mac-host.log`, `docs-check.sh`, `docs-check.log`, `fakecc/cc`, `fakecc/ar`.
   - Inside the worktree, gitignored: `target/aarch64-apple-darwin/` (the cross lint), the `connectors-docs` build under `target/debug`, and `.local/tmp/`.
   - The lease `adv2-cz-u229b` is released.
7.
```findings
[
  {"file": "crates/connectors-host/src/local/owner/transport.rs", "line": 1580, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "the new test covers own_copy and OwnCopy::drop but never runs serve_with_read_policy, so deleting the owner's own_copy() hold leaks the copy on every macOS owner start with every test green"},
  {"file": "crates/connectors-custody/src/keychain.rs", "line": 142, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the any_application_access doc says the CLI removes the owner's private copy once the owner has answered, but the owner now keeps it for its whole run"},
  {"file": "crates/connectors-custody/src/tests.rs", "line": 553, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the launch-copy keychain test's doc says the CLI removes the owner's copy once the owner has answered, but the owner now keeps it for its whole run"},
  {"file": "adapters/grafana/tests/local_runtime/cli_journey.rs", "line": 579, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the macOS journey's doc says it removes every item it wrote at the end, while its body only attempts removal and reports it because the test cannot delete the owner's items"}
]
```
