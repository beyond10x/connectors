---
format: aep.planning-md/3
id: review-result:test-step-stories-design-round-3
kind: review-result
status: active
title: Design critic, round 3, the test-step story set
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Third read, design critic, 2026-11-25. Read-only; nothing was run.

## Verdict

needs-revision for the set. One story blocks:
story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping. The other 19 are approved
on their own.

## Verdict per story

- `the-lane-slow-runs-the-ignored-timing-family`: approve
- `one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build`: approve
- `eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing`: approve
- `eighteen-tests-take-a-template-store-or-jail-built-once-per-run`: approve (notes 2 and 3)
- `six-cli-tests-take-jail-binaries-built-once-per-run`: approve
- `seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds`: approve
- `the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition`: approve
- `the-catalog-crate-tests-ingest-the-released-definition-once-per-process`: approve (note 6)
- `four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps`: approve
- `seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait`: approve
- `five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts`: approve
- `seven-cli-tests-shorten-their-waits-or-a-production-wait`: approve
- `eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping`: needs-revision (finding 1)
- `where-do-the-connection-lease-tests-clock-and-the-30-s-admission-bound-live`: approve. The
  answer can be checked: a `## Answer` with `file:line` and run output per test. The eleven-tests
  story, lines 88 to 93, says it reads the answer.
- `where-do-the-10-s-of-the-identification-retry-loop-test-go`: approve (note 4). The answer can
  be checked and names the story that then claims the test.
- `where-do-the-1298-s-of-the-tests-under-5-s-go`: approve (note 5). Acceptance 3 forces one of two
  stated endings, and question 3 is read by the operator.
- `store-tests-end-within-60s-under-load`: approve. The edge to the eighteen-tests story is in its
  relations, and the new section matches the parent.
- `four-host-tests-no-longer-wait-out-the-30-s-lock-wait` and
  `the-100-run-capacity-stress-keeps-10-runs-in-the-gate`: approve. `git diff` shows only the
  `--bin` change.
- The parent `the-gates-test-step-ends-within-120-s-on-the-forge`: approve. Its wave list matches
  the output of `aep plan artifact waves` (waves 1 to 8, none in wave 5), and it states that
  acceptance 1 needs the question story on the tests under 5 s.
- `five-host-unit-tests-shorten-the-waits-they-chose`: not read again.

## Round-2 findings

- 1 resolved. The user-namespace step is added to `slow.yml`, and the lane tests that need it are
  named: lane story lines 75 to 94.
- 2 resolved. `--no-tests fail` plus the new `release_build_has_no_debug_assertions` test: lane
  story lines 102 to 115 and acceptance 6.
- 3 resolved. The story is rewritten around a hard link inside `Jail::install` with a copy
  fallback, and its edge to the eighteen-tests story is gone: six-cli story lines 81 to 95 and
  relations (only `informed_by`). `fixture.rs` lines 118 to 127 hold no other write to an installed
  file.
- 4 resolved. The test is removed and the removal is stated: eight-adapter story lines 100 to 116.
  The lane story agrees at lines 211 to 216.
- 5 resolved. The story extracts a function from `spawn` and tests it: helper story lines 71 to 76
  and acceptance 3. The inline loop is at `transport.rs` 1686 to 1698.
- 6 resolved. A scan test, `no_debug_override_is_read_outside_the_helper`, with both sides: helper
  story lines 83 to 91 and acceptance 6.
- 7 partly resolved. Resolved for four stories: idle-exit lines 109 to 118, slow-owner lines 83 to
  93, seven-cli lines 135 to 157, five-tests lines 103 to 123. Not resolved for the 30 s admission
  bound: eleven story lines 91 to 93 defer the sentence until the answer. This is deliberate and is
  part of finding 1.
- 8 resolved. Question 2 is a throwaway probe that needs no other story: identification question
  lines 36 and 48 to 50.
- 9 resolved. The digest covers every file under `crates/connectors-host/src/local/` and
  `Cargo.lock`: eighteen-tests story lines 155 to 160.

Graph walk: the `depends_on` frontmatter of all 20 stories was read (20 edges inside the set), and
no edge enters the set from outside. There is no cycle. The `transport.rs` chain is helper, idle,
slow-owner, seven-cli, eleven, and each edge has a written reason. 11 acceptance reads of state a
sibling produces were traced, and `depends_on` edges record all 11: the lane command listing a
lane test (7 stories), the helper reading the lane's release step, the crate story reading the
provider story's fixture directory, the store-tests story reading the eighteen-tests store, the
eleven-tests story reading the question's answer.

## New findings

1. blocker. `eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping`: the Change
   section is three open routes and the scope is "settled by the answer" (lines 88 to 97 and 180).
   The story says it is revised before it is proposed, so its outcome cannot be stated yet.
   Approve once the question has its `## Answer` and the body names the route, the files, the
   sentence on what the 30 s bound protects, and a bound that follows from the route (line 146).
2. note. Eighteen-tests story, line 147: the setup script is not scoped by a filter. It would run
   `cargo run … test-fixtures` for every `cargo nextest run`, including the lane's release step
   and a developer's run of `-p connectors-sdk`: a dev-profile build of `connectors-build` with
   `connectors-host`, plus an `ess generate` that needs the pinned `ess` on PATH. A
   `[[profile.default.scripts]]` filter on the host and sql packages would avoid it.
3. note. Eighteen-tests story, lines 155 to 160 and 260 to 262: the template is made by the library
   without `cfg(test)`, but the tests run the `cfg(test)` build. `metadata.rs` has more than 25
   `cfg(test)` items before line 700, and `metadata/er.rs` has `cfg(not(test))` variants at lines
   701 and 3157. The "not established" list does not name this.
4. note. `where-do-the-10-s-of-the-identification-retry-loop-test-go`, lines 49 and 61: the scope
   says "no file of the repository is written", but the probe edits `transport.rs` in the working
   tree, the file five stories write in sequence. Say the probe runs in a throwaway tree.
5. note. `where-do-the-1298-s-of-the-tests-under-5-s-go`, lines 61 to 63: question 3 offers "more
   tests at a time" and "a split of the step". The folder rule and the parent's Limits rule out
   shards and larger runners as the fix. Say the options are listed with their cost and that the
   rule bars them unless the operator lifts it.
6. note. `the-catalog-crate-tests-ingest-the-released-definition-once-per-process`, lines 98 to
   103: the fixture lives in the catalog crate, but its staleness check lives in the provider
   crate's tests. `cargo test -p connectors-catalog` alone reads an unchecked file. The gate runs
   both.
7. note. Eight-adapter story, lines 117 to 123: the candidate paths for the kubernetes test come
   from the runner image and are read from the first red gate job. The test then fails in CI
   whenever the image changes. It should say who owns the list afterwards.

## Not checked

- Whether cargo-nextest 0.9.143 runs setup scripts without an experimental switch, and whether
  `--no-tests fail` behaves as the lane story says.
- That `cargo nextest run --release` resolves to a profile without `debug_assertions`.
  `Cargo.toml` lines 64 to 74 only show that dev and test enable them.
- That a hard link from `target/` into the jail root works on the forge runner.
- Which of the 26 and 9 unclaimed tests the story set leaves untouched.
- Every "after" figure. The sums were not recomputed beyond the parent's arithmetic.
- The `ess generate` pin and the Secret Service prerequisite of the one catalog-provider `Timing`
  entry, beyond `ignored.rs` lines 169 to 176 and 249.

```findings
- file: .engineering/planning/story/eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping.md
  line: 88
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Change section is three open routes and the scope is settled only by the answer of the question story, so the outcome cannot be stated and implementation must not start; revise from the answer, naming the route, the files, the sentence on what the 30 s bound protects and a bound that follows from the route"
- file: .engineering/planning/story/eighteen-tests-take-a-template-store-or-jail-built-once-per-run.md
  line: 147
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the setup script has no filter, so it runs for every cargo nextest invocation including the lane's release step and builds connectors-build with connectors-host and needs ess on PATH; scope it to the packages that read the fixtures"
- file: .engineering/planning/story/eighteen-tests-take-a-template-store-or-jail-built-once-per-run.md
  line: 260
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the template is built by the non-cfg(test) library and read by cfg(test) tests while metadata.rs and metadata/er.rs have cfg(test) branches; the not-established list does not name this"
- file: .engineering/planning/story/where-do-the-10-s-of-the-identification-retry-loop-test-go.md
  line: 61
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "scope says no file is written but the probe edits transport.rs in the working tree; say the probe runs in a throwaway tree"
- file: .engineering/planning/story/where-do-the-1298-s-of-the-tests-under-5-s-go.md
  line: 61
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "question 3 lists more tests at a time and a split of the step, which the folder rule and the parent's Limits exclude as fixes; say they are listed with cost and barred unless the operator lifts the rule"
- file: .engineering/planning/story/the-catalog-crate-tests-ingest-the-released-definition-once-per-process.md
  line: 98
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the fixture the crate test reads is checked for staleness only by the provider crate's test, so cargo test -p connectors-catalog alone reads an unchecked file"
- file: .engineering/planning/story/eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing.md
  line: 117
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the kubernetes candidate list is read from the first red gate job and pins the runner image; the story does not say who owns the list afterwards"
```
