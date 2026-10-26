---
format: aep.planning-md/3
id: story:ci-timing-flakes-20261018
kind: story
status: implemented
title: Four tests failed on CI under load on 2026-10-18 without a change causing them
refs:
- provider: github
  reference: beyond10x/connectors#183
relations:
- serves: vision:independent-contract-adapters
- supersedes: story:oauth-drain-macos-race
- supersedes: story:kubernetes-exec-fixture-busy
scope:
- confidence: cited
  path: adapters/kubernetes/src/exec_credential.rs
- confidence: cited
  path: adapters/kubernetes/src/local.rs
- confidence: cited
  path: adapters/kubernetes/tests/exec_credential.rs
- confidence: cited
  path: adapters/kubernetes/tests/exec_credential_adversary.rs
- confidence: cited
  path: adapters/kubernetes/tests/exec_credential_security.rs
- confidence: cited
  path: adapters/kubernetes/tests/exec_credential_verification.rs
- confidence: cited
  path: adapters/kubernetes/tests/local_runtime.rs
- confidence: cited
  path: contracts/cli/v1alpha1/owner.md
- confidence: cited
  path: crates/connectors-host/src/local/oauth.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-10-19T21:36:33Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-19T21:36:33Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-10-23T20:45:59Z", actor: "human:timo", revision: 12, decided_on: {"recorded":{"test_result":2,"review_outcome":1,"verification":1}}}
---
## Defect

Four tests failed on CI on 2026-10-18 on pull requests whose change did not cause them; each passes
on a rerun, and every main gate run since `7e1fa1a5a` has passed. All four wait on another thread, process or child within a fixed time, which points at
scheduling under load. That is a hypothesis, not observed.

| Test | Defined at (main `7e1fa1a5a`) | Failure location as reported | Job, runs |
|---|---|---|---|
| `local::owner::transport::tests::a_connection_closed_by_a_retiring_owner_reaches_the_next_owner` | `crates/connectors-host/src/local/owner/transport.rs:2464` | `transport.rs:2561` | macOS "host and CLI": main at `82a681bc4` run 39093413199; PR #185 run 39108098384 |
| `local::oauth::tests::a_request_queued_before_the_flow_returns_is_answered` | `crates/connectors-host/src/local/oauth.rs:1079` | `oauth.rs:1090` | macOS "host and CLI": run 39134820838 |
| `exec_plugin_deadline_overrun_is_refused_and_killed` | `adapters/kubernetes/tests/exec_credential.rs:284` | `exec_credential.rs:299`, `PluginUnavailable` where `Timeout` was expected | Linux repository gate: run 39123570136 |
| `exec_plugin_native_executable_inherits_no_descriptor_of_the_adapter` | `adapters/kubernetes/tests/exec_credential_security.rs:159` | `exec_credential_security.rs:131`, "the plugin answered: PluginUnavailable" | Linux repository gate: run 39125756481; PR #189 run 39142186809 |

## Hypothesis: exec of a just-copied plugin (unverified)

`exec_plugin_native_executable_inherits_no_descriptor_of_the_adapter` failed a third time (run
39142186809, PR #189). The test `fs::copy`s a native binary and then executes it. If another test
thread forks while the copy's write descriptor is still open, the forked child holds that descriptor
until its own exec, and the plugin's exec fails with `ETXTBSY`, which the adapter reports as
`PluginUnavailable`. The S1 unit saw "Text file busy" in `live_read_loop` the same way. Not observed
for this test yet. Candidate fixes: retry the exec on `ETXTBSY`, or write the copy through a child
process. The first step is to confirm the hypothesis by logging the exec errno on that path. The
same mechanism may explain `exec_plugin_deadline_overrun_is_refused_and_killed` answering
`PluginUnavailable` instead of `Timeout`; that too is unverified.

## Outcome

Each test's verdict depends only on the behaviour it checks, not on how fast the machine schedules
its threads, processes or children. Where the code under test, not the test, is the cause, the code
is fixed instead and the cause is named.

## Acceptance

- For each of the four tests the cause of the recorded failure is named with a reproduction (for
  example the same assertion failing under `stress-ng --cpu 20` or a loop of 200 runs), or the test
  is shown to fail that way no longer.
- Each test passes 200 of 200 runs in a loop on Linux while every core is held busy (load average at
  least the core count), run as `cargo test -p <package> --test <target> -- --exact <name>` or the
  `--lib` equivalent, with the count recorded as evidence.
- Over 20 consecutive CI runs of the macOS "host and CLI" job and 20 of the Linux repository gate
  after the change, none of the four tests fails, with the run ids recorded.
- No test is marked `#[ignore]`, retried or given a longer deadline without the cause above being
  named.

## Not in scope

Other flaky tests already recorded (`story:registry-passive-approval-test-flake`,
`story:gmail-attachment-read-flaky-under-load` and the rest); CI retry policy.

## Folded from earlier flake stories

Two earlier draft stories recorded the same failure family and are archived into this one. Their
observations and acceptance lines carry over.

- `story:oauth-drain-macos-race`: `local::oauth::tests::a_request_queued_before_the_flow_returns_is_answered`
  (the second row of the table above) also failed on pull request
  https://github.com/beyond10x/connectors/pull/170, run 38633201731, 2026-10-14, and
  `local::oauth::tests::second_request_refused` failed on `main`, run 38455605662, 2026-10-13; both
  on the macOS runner only. Commit `be3f6f201` targeted the race and it still occurs. Added
  acceptance: the cause is shown from a macOS reproduction (the drain's accept or poll order, the
  non-blocking accept on macOS, or the test's read of a reset connection), and the two oauth tests
  pass 500 consecutive repetitions on the macOS runner.
- `story:kubernetes-exec-fixture-busy`: `exec_plugin_native_executable_answers_through_its_descriptor`
  (`adapters/kubernetes/tests/exec_credential.rs:456`) answered `Err(PluginUnavailable)` once in a
  full `cargo test -p connectors-kubernetes --no-fail-fast` run on 2026-10-16 and passed alone;
  the same `ETXTBSY` hypothesis as the exec rows above. Added acceptance: `PluginUnavailable`
  from an exec plugin carries the spawn error kind, so an operator can tell a missing plugin from
  a busy one.

## Scope delivered

Delivered by wave 20261019-post0540 (`git diff --name-only 52769afb3...impl/ci-timing-flakes-20261018`); every path is **cited** by that diff. The `## Scope` section above is the plan; differences are listed under *Corrections*.

- `adapters/kubernetes/src/exec_credential.rs`
- `adapters/kubernetes/src/local.rs`
- `adapters/kubernetes/tests/exec_credential.rs`
- `adapters/kubernetes/tests/exec_credential_adversary.rs`
- `adapters/kubernetes/tests/exec_credential_security.rs`
- `adapters/kubernetes/tests/exec_credential_verification.rs`
- `adapters/kubernetes/tests/local_runtime.rs`
- `contracts/cli/v1alpha1/owner.md`
- `crates/connectors-host/src/local/oauth.rs`
- `crates/connectors-host/src/local/owner/transport.rs`

**Corrections:** `adapters/kubernetes/src/exec_credential.rs` and `local.rs`, three more Kubernetes test files with the same `ETXTBSY` race, and `contracts/cli/v1alpha1/owner.md` were touched beyond the plan.

## Remaining after wave 20261019-post0540

Delivered and merged in `wave/20261019-post0540` (U4, merge cf742e7d3): the exec-plugin `ETXTBSY`
cause (58 of 58 local failures `PluginUnavailable(Some(ExecutableFileBusy))`; `exec_credential`
253/300 before, 1000/1000 after), the owner socket bind-then-chmod window (reproduced, fixed in
`transport.rs` `listen`), the oauth drain wait (cause inferred for macOS, not reproduced on Linux),
each story test 200/200 under load on Linux.

Not done, so the story stays active:
- 20 consecutive CI runs of the macOS "host and CLI" job and 20 of the Linux repository gate, with
  run ids. The macOS hard link of the owner socket has not run anywhere until the wave's pull
  request (review-result:wave-20261019-post0540-u4-adversary-1, finding 2).
- "An operator can tell a missing plugin from a busy one" moved to
  story:exec-plugin-busy-reaches-the-operator.

## CI runs after the change (recorded 2026-10-23)

Every completed, non-cancelled run of `macos-host.yml` and `rust-gate.yml` created after the merge of wave 20261019-post0540 into `main` (`9201b1177`, 2026-10-20T03:10Z), read with `gh run list` and `gh run view --log` on 2026-10-23.

| job | runs that ran the tests | each of the story's tests that job runs reported `ok` | `FAILED` lines for the four tests |
|---|---|---|---|
| macOS "host and CLI" | 30 | the transport and oauth tests, 30 of 30 | 0 |
| Linux repository gate | 31 | all four, 31 of 31 | 0 |

macOS run 39918212761 (PR head `5b647963e`, 2026-10-23) failed at its clippy step on a compile error that `4a2646bc8` fixed; no test ran in it. Six runs of each workflow were cancelled before their tests finished. The macOS job does not run the two Kubernetes exec tests; they run in the Linux gate.

macOS run ids: 39362858325 39401801061 39406844139 39408743876 39414455491 39418872472 39432076220 39728541014 39733507912 39738214531 39743582053 39748364572 39749153505 39754687483 39760985042 39766024936 39926635535 39937230529 39943283679 39949605761 39962346430 39965593281 39970497575 39972033131 39978616586 39983955961 39991148611 39997882523 40004358988 40004469187 

Linux gate run ids: 39362858339 39401801053 39406844133 39408743936 39414455442 39418872423 39432076386 39728541027 39733507867 39738214383 39743581994 39748364439 39749153397 39754687343 39760985051 39766025021 39918212799 39926635504 39937230304 39943283073 39949605770 39962346477 39965593121 39970497699 39972032514 39978616351 39983955881 39991148557 39997882592 40004359189 40004469191 

The owner socket is bound through `listen` (`crates/connectors-host/src/local/owner/transport.rs:1518`), which links it into place with `std::fs::hard_link` (:1527) on every unix target. The transport test above, which starts owners through that path, passed in each of the 30 macOS runs, so the macOS hard link (review-result:wave-20261019-post0540-u4-adversary-1, finding 2) has run there 30 times. That it ran is read from the code path; no run logged the link itself.
