---
format: aep.planning-md/3
id: story:local-cli-capacity-probe-flake
kind: story
status: implemented
title: The build probe against a full owner reports not_running instead of capacity in 2 of 5 runs
relations:
- serves: vision:independent-contract-adapters
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-29T13:54:21Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-29T13:54:21Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-29T14:54:45Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Defect

`adversary2_the_build_probe_against_an_owner_at_capacity_reports_capacity`
(`apps/connectors/tests/local_cli.rs`, assertion at :1409) answers `ok:not_running` where
`capacity` is expected, about 1 to 2 s after the owner was filled with 32 clients, well before the
owner's 10-second greeting deadline that the test's comment names as the only way a slot frees.

Observed on 2026-10-29 on Linux, on `main` `f4df1c4f8` (ESS 0.75.0), without load from other
suites: 2 of 5 runs of `cargo test --locked --offline -p connectors --test local_cli -- --exact
adversary2_the_build_probe_against_an_owner_at_capacity_reports_capacity` failed (elapsed 1.06 s in
the failing full-gate run). The same tree with only the ESS pin moved failed 3 of 5 runs, and the
full repository gate failed on it once.

## Outcome

The test passes only when the CLI reports `capacity` against a full same-build owner, and its
verdict does not depend on scheduling. If the CLI or the owner, not the test, frees a slot early
(a held client closed, a probe connection counted differently), the code is fixed and the cause is
named.

## Acceptance

- The cause of `ok:not_running` within 2 s of filling the owner is named with a reproduction.
- The test passes 200 of 200 runs on Linux under load (`nice -n 19`, at most 4 workers).
- No `#[ignore]`, retry or longer deadline without the named cause.

## Cause

The test, not the CLI or the owner. The owner serves its startup channel as its first client, in one of its 32 slots (`crates/connectors-host/src/local/owner/transport.rs`, `serve_with_read_policy`: `handle(owner.clone(), startup)`). The test helper `start_owner_in` closed its end of that channel before the owner ran, and `owner_start::wait_listening` returns as soon as the socket file exists, before the owner takes that channel. `fill_owner` then connected 32 clients and a probe. When the owner's thread for the startup channel had not yet read the closed end, the owner counted it plus 31 of the test's clients, refused the 32nd and the probe, and `fill_owner` returned. Then the thread read end-of-file and freed the slot. The CLI's `build` probe and its greeting took that slot and were served, so `instances status` answered `ok:not_running`. On a quiet multi-core host the thread reads the closed channel before the clients arrive, so the race needs load or a single CPU.

The owner behaves as designed: a starting CLI greets on that channel at once, and a slot whose peer has gone frees when the owner reads it.

Reproduction on `wave/d210` from `616aef64f`, with each run executing the test binary on its own (`--exact`):

- Pinned to one CPU (`taskset -c 3`, `nice -n 19`): this test failed 18 of 20 and 19 of 20 runs, and `a_same_build_owner_at_its_client_limit_answers_capacity_not_another_build`, which fills the owner the same way, failed 20 of 20 (exit 0, not 1). Unpinned, it passed 60 of 60.
- Owner instrumentation (a throw-away build, not committed) showed 34 owner clients for the test's 33 connections. The first was the startup channel: its peer was the test process, and `recv(MSG_PEEK)` on it returned 0 right after the two `capacity` refusals. Its exchange ended `Unavailable`, and the CLI's probe was then admitted at "count before 31".

## Fix

`start_owner_holding_startup` returns the test's end of the startup channel open. `fill_owner` takes that end and holds it with the clients, so a refusal proves the test holds every slot until the owner's 10-second greeting deadline. Both capacity tests use it. The assertion is unchanged. The failure message now gives the elapsed time from the owner's start, when the startup channel's deadline begins.

## Result

After the fix, with the same commands: pinned to one CPU, 20 of 20 and then 50 of 50 runs passed for each of the two tests. Unpinned under 4 parallel workers at `nice -n 19`, 200 of 200 passed. `cargo test -p connectors --test local_cli` passed 27, with 1 ignored that was already ignored.
