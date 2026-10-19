---
format: aep.planning-md/3
id: story:ci-timing-flakes-20261018
kind: story
status: active
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
  path: adapters/kubernetes/tests/exec_credential.rs
- confidence: cited
  path: adapters/kubernetes/tests/exec_credential_security.rs
- confidence: cited
  path: crates/connectors-host/src/local/oauth.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-19T21:36:33Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-19T21:36:33Z", actor: "human:timo", revision: 7}
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
